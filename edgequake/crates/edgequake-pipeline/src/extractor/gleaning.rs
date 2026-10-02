//! Gleaning (re-extraction) extractor for finding missed entities.
//!
//! @implements FEAT0305

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::completion_options::{
    extraction_completion_options_with_effort, maybe_lift_extract_effort_from_llm_error,
};
use super::{EntityExtractor, ExtractedEntity, ExtractedRelationship, ExtractionResult};
use crate::chunker::TextChunk;
use crate::error::{PipelineError, Result};
use crate::prompts::{
    enforce_entity_type, enforce_relationship_against_schema, EntityExtractionSchema,
    JsonExtractionParser, JsonParseOptions,
};

/// Configuration for gleaning (re-extraction).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GleaningConfig {
    /// Maximum number of gleaning iterations.
    pub max_gleaning: usize,
}

impl Default for GleaningConfig {
    fn default() -> Self {
        Self {
            max_gleaning: 1, // LightRAG default
        }
    }
}

/// A wrapper extractor that performs gleaning (re-extraction) to find missed entities.
///
/// # WHY: Multi-Pass Extraction (Gleaning)
///
/// LLMs often miss entities in a single pass due to:
/// - Attention limits on long texts
/// - Implicit entities (e.g., "the company" referring to earlier-mentioned "Apple")
/// - Context overload when many entities are present
///
/// **Gleaning Strategy:**
/// 1. First pass: Normal extraction with base extractor
/// 2. Subsequent passes: Re-prompt LLM with "What did you miss?"
///    - Include previously-found entities to avoid duplicates
///    - Focus on implicit/indirect entity mentions
///
/// **LightRAG Research Finding:**
/// - 1-2 gleaning iterations improve recall by 15-25%
/// - Diminishing returns after 2 iterations
/// - Cost: Each iteration = 1 additional LLM call
///
/// This implements GAP-018: Max Gleaning from LightRAG.
pub struct GleaningExtractor {
    /// The underlying LLM provider.
    llm_provider: std::sync::Arc<dyn edgequake_llm::LLMProvider>,
    /// The base extractor to use.
    base_extractor: std::sync::Arc<dyn EntityExtractor>,
    /// Workspace entity type allow-list (must match base extractor).
    entity_schema: EntityExtractionSchema,
    /// Gleaning configuration.
    config: GleaningConfig,
    /// Natural-language output language (must match base extractor; SPEC-096).
    language: String,
    /// Desired reasoning effort for gleaning LLM calls (SPEC-113 think-off).
    reasoning_effort: Option<String>,
    /// SPEC-117: resolved per-response caps (`None` → fleet env at use time).
    extraction_caps: Option<crate::prompts::ExtractionCaps>,
}

impl GleaningExtractor {
    /// Create a new gleaning extractor.
    pub fn new(
        llm_provider: std::sync::Arc<dyn edgequake_llm::LLMProvider>,
        base_extractor: std::sync::Arc<dyn EntityExtractor>,
    ) -> Self {
        Self {
            llm_provider,
            base_extractor,
            entity_schema: EntityExtractionSchema::server_default(),
            config: GleaningConfig::default(),
            language: crate::prompts::DEFAULT_EXTRACTION_LANGUAGE.to_string(),
            reasoning_effort: None,
            extraction_caps: None,
        }
    }

    /// Set the entity type schema (strict/permissive allow-list).
    pub fn with_entity_schema(mut self, schema: EntityExtractionSchema) -> Self {
        self.entity_schema = schema;
        self
    }

    /// Set natural-language output language (SPEC-096).
    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.language = language.into();
        self
    }

    /// Set desired gleaning reasoning effort (mirrors base extract).
    pub fn with_reasoning_effort(mut self, effort: Option<String>) -> Self {
        self.reasoning_effort = effort
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        self
    }

    /// Set resolved extract caps (SPEC-117).
    pub fn with_extraction_caps(mut self, caps: crate::prompts::ExtractionCaps) -> Self {
        self.extraction_caps = Some(caps);
        self
    }

    fn resolved_caps(&self) -> crate::prompts::ExtractionCaps {
        self.extraction_caps
            .unwrap_or_else(crate::prompts::ExtractionCaps::from_env)
    }

    /// Set the gleaning configuration.
    pub fn with_config(mut self, config: GleaningConfig) -> Self {
        self.config = config;
        self
    }

    /// Set the maximum gleaning iterations.
    pub fn with_max_gleaning(mut self, max: usize) -> Self {
        self.config.max_gleaning = max;
        self
    }

    /// Build the gleaning prompt.
    #[allow(dead_code)]
    fn build_gleaning_prompt(
        &self,
        chunk: &TextChunk,
        previous_entities: &[String],
        after_caps_truncate: bool,
    ) -> String {
        let text =
            crate::prompts::text_with_section_context(&chunk.content, chunk.section.as_ref());
        crate::prompts::json_gleaning_prompt_with_caps(
            &text,
            previous_entities,
            &self.entity_schema,
            &crate::prompts::effective_extraction_language(&self.language),
            self.resolved_caps(),
            after_caps_truncate,
        )
    }

    /// Parse gleaning response via shared JSON parser (normalization + BR0006 filters).
    fn parse_gleaning_response(
        &self,
        response: &str,
        chunk_id: &str,
    ) -> Result<(Vec<ExtractedEntity>, Vec<ExtractedRelationship>)> {
        let parsed = JsonExtractionParser::new().parse_with_options(
            response,
            chunk_id,
            JsonParseOptions {
                entity_schema: Some(&self.entity_schema),
                extraction_caps: Some(self.resolved_caps()),
                ..Default::default()
            },
        )?;
        Ok((parsed.entities, parsed.relationships))
    }

    fn apply_entity_schema_to_entities(&self, entities: &mut [ExtractedEntity]) {
        for entity in entities.iter_mut() {
            let (enforced, _) = enforce_entity_type(&entity.entity_type, &self.entity_schema);
            entity.entity_type = enforced;
        }
    }

    fn apply_relation_schema_to_relationships(
        &self,
        relationships: &mut [ExtractedRelationship],
        name_to_type: &std::collections::HashMap<String, String>,
    ) {
        for rel in relationships.iter_mut() {
            let (enforced, _) = enforce_relationship_against_schema(
                &rel.source,
                &rel.target,
                &rel.relation_type,
                name_to_type,
                &self.entity_schema,
            );
            rel.relation_type = enforced;
        }
    }

    /// Merge gleaning results with original results.
    fn merge_results(
        &self,
        original: &mut ExtractionResult,
        glean_entities: Vec<ExtractedEntity>,
        glean_relationships: Vec<ExtractedRelationship>,
    ) {
        // For entities: compare descriptions and keep the better (longer) one
        for glean_entity in glean_entities {
            let glean_key = crate::prompts::normalize_entity_name(&glean_entity.name);
            let existing = original
                .entities
                .iter_mut()
                .find(|e| crate::prompts::normalize_entity_name(&e.name) == glean_key);

            if let Some(existing) = existing {
                // Keep the entity with the longer description
                if glean_entity.description.len() > existing.description.len() {
                    existing.description = glean_entity.description;
                    existing.entity_type = glean_entity.entity_type;
                }
            } else {
                // New entity from gleaning
                original.entities.push(glean_entity);
            }
        }

        // For relationships: compare and keep better descriptions
        for glean_rel in glean_relationships {
            let source_key = crate::prompts::normalize_entity_name(&glean_rel.source);
            let target_key = crate::prompts::normalize_entity_name(&glean_rel.target);
            let existing = original.relationships.iter_mut().find(|r| {
                crate::prompts::normalize_entity_name(&r.source) == source_key
                    && crate::prompts::normalize_entity_name(&r.target) == target_key
            });

            if let Some(existing) = existing {
                if glean_rel.description.len() > existing.description.len() {
                    existing.description = glean_rel.description;
                    existing.relation_type = glean_rel.relation_type;
                }
            } else {
                original.relationships.push(glean_rel);
            }
        }
    }

    /// Run one gleaning LLM turn (effort-lift retry included).
    ///
    /// Returns `Err` on LLM failure so the caller can fail-open (SPEC-156).
    async fn run_glean_llm_turn(
        &self,
        chunk: &TextChunk,
        entity_names: &[String],
        after_caps_truncate: bool,
        iteration: usize,
    ) -> Result<edgequake_llm::traits::LLMResponse> {
        let mut desired_effort = self.reasoning_effort.clone();
        let mut options = extraction_completion_options_with_effort(
            self.llm_provider.model(),
            16_384,
            desired_effort.as_deref(),
            self.llm_provider.name(),
        )
        .with_provider_prompt_cache(
            "glean",
            self.llm_provider.name(),
            self.llm_provider.model(),
        );
        let model = self.llm_provider.model().to_string();
        let provider_name = self.llm_provider.name().to_string();
        let resp = edgequake_observability::with_llm_generation(
            "extract-entities-glean",
            &model,
            &provider_name,
            async {
                let text = crate::prompts::text_with_section_context(
                    &chunk.content,
                    chunk.section.as_ref(),
                );
                let caps = self.resolved_caps();
                let messages = vec![
                    edgequake_llm::traits::ChatMessage::system(
                        crate::prompts::json_gleaning_system_prompt_with_caps(
                            &self.entity_schema,
                            &crate::prompts::effective_extraction_language(&self.language),
                            caps,
                            after_caps_truncate,
                        ),
                    ),
                    edgequake_llm::traits::ChatMessage::user(
                        crate::prompts::json_gleaning_user_prompt(&text, entity_names),
                    ),
                ];
                let mut last_err: Option<String> = None;
                let mut resp = None;
                for _attempt in 0..2 {
                    match self.llm_provider.chat(&messages, Some(&options)).await {
                        Ok(r) => {
                            resp = Some(r);
                            break;
                        }
                        Err(e) => {
                            if let Some(lifted) = maybe_lift_extract_effort_from_llm_error(
                                self.llm_provider.name(),
                                self.llm_provider.model(),
                                options
                                    .reasoning_effort
                                    .as_deref()
                                    .or(desired_effort.as_deref()),
                                &e.to_string(),
                            ) {
                                tracing::warn!(
                                    from = desired_effort.as_deref().unwrap_or("none"),
                                    to = %lifted,
                                    "Glean reasoning-off rejected; lifting effort and retrying"
                                );
                                desired_effort = Some(lifted);
                                options = extraction_completion_options_with_effort(
                                    self.llm_provider.model(),
                                    16_384,
                                    desired_effort.as_deref(),
                                    self.llm_provider.name(),
                                )
                                .with_provider_prompt_cache(
                                    "glean",
                                    self.llm_provider.name(),
                                    self.llm_provider.model(),
                                );
                                last_err = Some(e.to_string());
                                continue;
                            }
                            return Err(PipelineError::ExtractionError(format!(
                                "Gleaning LLM error: {e}"
                            )));
                        }
                    }
                }
                let resp = resp.ok_or_else(|| {
                    PipelineError::ExtractionError(format!(
                        "Gleaning LLM error: {}",
                        last_err.unwrap_or_else(|| "unknown".into())
                    ))
                })?;
                edgequake_observability::record_observation_meta(
                    "gleaning_iteration",
                    &iteration.to_string(),
                );
                let llm_input = edgequake_observability::format_llm_chat_turns_for_observation(
                    messages.iter().map(|m| {
                        let role = match m.role {
                            edgequake_llm::traits::ChatRole::System => "System",
                            edgequake_llm::traits::ChatRole::Assistant => "Assistant",
                            edgequake_llm::traits::ChatRole::User => "User",
                            edgequake_llm::traits::ChatRole::Tool => "Tool",
                            edgequake_llm::traits::ChatRole::Function => "Function",
                        };
                        (
                            role,
                            m.content.as_str(),
                            m.images.as_ref().map(|i| i.len()).unwrap_or(0),
                        )
                    }),
                );
                let rec = edgequake_observability::LlmGenerationRecord::from_response(
                    Some(&llm_input),
                    &resp.content,
                    resp.prompt_tokens as u64,
                    resp.completion_tokens as u64,
                )
                .with_provider_cache(resp.cache_hit_tokens, resp.cache_write_tokens);
                Ok::<_, PipelineError>((resp, rec))
            },
        )
        .await?;
        Ok(resp)
    }
}

#[async_trait]
impl EntityExtractor for GleaningExtractor {
    async fn extract(&self, chunk: &TextChunk) -> Result<ExtractionResult> {
        // First pass: use base extractor
        let mut result = self.base_extractor.extract(chunk).await?;

        // Skip gleaning if disabled
        if self.config.max_gleaning == 0 {
            return Ok(result);
        }

        // SPEC-117: when hard truncate applied, continue prompts ask for additional ents.
        let after_caps_truncate = crate::prompts::extract_caps_were_applied(&result);

        let mut iterations_run = 0usize;
        for iteration in 0..self.config.max_gleaning {
            tracing::debug!(
                chunk_id = %chunk.id,
                iteration = iteration,
                after_caps_truncate = after_caps_truncate,
                "Performing gleaning iteration"
            );

            let entity_names: Vec<String> =
                result.entities.iter().map(|e| e.name.clone()).collect();

            // SPEC-156: fail-open — keep the paid base result if glean LLM fails.
            let response = match self
                .run_glean_llm_turn(chunk, &entity_names, after_caps_truncate, iteration)
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!(
                        chunk_id = %chunk.id,
                        iteration = iteration,
                        error = %e,
                        "Gleaning LLM failed — keeping base extraction (fail-open)"
                    );
                    break;
                }
            };

            iterations_run += 1;
            result.input_tokens += response.prompt_tokens;
            result.output_tokens += response.completion_tokens;

            match self.parse_gleaning_response(&response.content, &chunk.id) {
                Ok((glean_entities, glean_relationships)) => {
                    let new_entities = glean_entities.len();
                    let new_relationships = glean_relationships.len();

                    self.merge_results(&mut result, glean_entities, glean_relationships);

                    tracing::debug!(
                        chunk_id = %chunk.id,
                        iteration = iteration,
                        new_entities = new_entities,
                        new_relationships = new_relationships,
                        "Gleaning completed"
                    );

                    // SPEC-156: early stop when an iteration adds nothing new.
                    if new_entities == 0 && new_relationships == 0 {
                        tracing::debug!(
                            chunk_id = %chunk.id,
                            iteration = iteration,
                            "Gleaning early stop — no new entities or relationships"
                        );
                        break;
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        chunk_id = %chunk.id,
                        iteration = iteration,
                        error = %e,
                        "Gleaning parse error, continuing"
                    );
                }
            }
        }

        // Defense in depth: re-apply schema after merge (base pass + gleaning).
        self.apply_entity_schema_to_entities(&mut result.entities);
        let name_to_type: std::collections::HashMap<String, String> = result
            .entities
            .iter()
            .map(|e| (e.name.clone(), e.entity_type.clone()))
            .collect();
        self.apply_relation_schema_to_relationships(&mut result.relationships, &name_to_type);

        result.metadata.insert(
            "gleaning_iterations".to_string(),
            serde_json::json!(iterations_run),
        );

        Ok(result)
    }

    fn name(&self) -> &str {
        "gleaning"
    }

    fn model_name(&self) -> &str {
        self.llm_provider.model()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    fn api_strict_schema() -> EntityExtractionSchema {
        EntityExtractionSchema {
            types: vec![
                "API_OR_INTERFACE".into(),
                "CODE_ELEMENT".into(),
                "SOFTWARE_COMPONENT".into(),
                "OTHER".into(),
            ],
            strict: true,
            relation_types: Vec::new(),
            relation_strict: true,
            relation_edges: Vec::new(),
        }
    }

    struct FixedBaseExtractor {
        entities: Vec<ExtractedEntity>,
    }

    #[async_trait]
    impl EntityExtractor for FixedBaseExtractor {
        async fn extract(&self, chunk: &TextChunk) -> Result<ExtractionResult> {
            let mut result = ExtractionResult::new(&chunk.id);
            result.entities = self.entities.clone();
            Ok(result)
        }
        fn name(&self) -> &str {
            "fixed-base"
        }
        fn model_name(&self) -> &str {
            "fixed"
        }
    }

    /// LLM that fails every `chat` call.
    struct AlwaysFailLlm;

    #[async_trait]
    impl edgequake_llm::LLMProvider for AlwaysFailLlm {
        fn name(&self) -> &str {
            "fail"
        }
        fn model(&self) -> &str {
            "fail-model"
        }
        fn max_context_length(&self) -> usize {
            8192
        }
        async fn complete(
            &self,
            _prompt: &str,
        ) -> edgequake_llm::error::Result<edgequake_llm::traits::LLMResponse> {
            Err(edgequake_llm::error::LlmError::NetworkError(
                "injected glean failure".into(),
            ))
        }
        async fn complete_with_options(
            &self,
            prompt: &str,
            _options: &edgequake_llm::traits::CompletionOptions,
        ) -> edgequake_llm::error::Result<edgequake_llm::traits::LLMResponse> {
            self.complete(prompt).await
        }
        async fn chat(
            &self,
            _messages: &[edgequake_llm::traits::ChatMessage],
            _options: Option<&edgequake_llm::traits::CompletionOptions>,
        ) -> edgequake_llm::error::Result<edgequake_llm::traits::LLMResponse> {
            Err(edgequake_llm::error::LlmError::NetworkError(
                "injected glean failure".into(),
            ))
        }
        async fn chat_with_tools(
            &self,
            messages: &[edgequake_llm::traits::ChatMessage],
            _tools: &[edgequake_llm::traits::ToolDefinition],
            _tool_choice: Option<edgequake_llm::traits::ToolChoice>,
            options: Option<&edgequake_llm::traits::CompletionOptions>,
        ) -> edgequake_llm::error::Result<edgequake_llm::traits::LLMResponse> {
            self.chat(messages, options).await
        }
    }

    /// LLM that returns empty JSON on every call and counts invocations.
    struct EmptyGleanLlm {
        calls: AtomicUsize,
    }

    #[async_trait]
    impl edgequake_llm::LLMProvider for EmptyGleanLlm {
        fn name(&self) -> &str {
            "empty-glean"
        }
        fn model(&self) -> &str {
            "empty-model"
        }
        fn max_context_length(&self) -> usize {
            8192
        }
        async fn complete(
            &self,
            _prompt: &str,
        ) -> edgequake_llm::error::Result<edgequake_llm::traits::LLMResponse> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(edgequake_llm::traits::LLMResponse::new(
                r#"{"entities":[],"relationships":[]}"#,
                "empty-model",
            )
            .with_usage(10, 5))
        }
        async fn complete_with_options(
            &self,
            prompt: &str,
            _options: &edgequake_llm::traits::CompletionOptions,
        ) -> edgequake_llm::error::Result<edgequake_llm::traits::LLMResponse> {
            self.complete(prompt).await
        }
        async fn chat(
            &self,
            _messages: &[edgequake_llm::traits::ChatMessage],
            _options: Option<&edgequake_llm::traits::CompletionOptions>,
        ) -> edgequake_llm::error::Result<edgequake_llm::traits::LLMResponse> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(edgequake_llm::traits::LLMResponse::new(
                r#"{"entities":[],"relationships":[]}"#,
                "empty-model",
            )
            .with_usage(10, 5))
        }
        async fn chat_with_tools(
            &self,
            messages: &[edgequake_llm::traits::ChatMessage],
            _tools: &[edgequake_llm::traits::ToolDefinition],
            _tool_choice: Option<edgequake_llm::traits::ToolChoice>,
            options: Option<&edgequake_llm::traits::CompletionOptions>,
        ) -> edgequake_llm::error::Result<edgequake_llm::traits::LLMResponse> {
            self.chat(messages, options).await
        }
    }

    #[test]
    fn gleaning_parse_enforces_strict_types() {
        let extractor = GleaningExtractor::new(
            Arc::new(edgequake_llm::MockProvider::new()),
            Arc::new(crate::extractor::LLMExtractor::new(Arc::new(
                edgequake_llm::MockProvider::new(),
            ))),
        )
        .with_entity_schema(api_strict_schema());

        let response = r#"{
            "entities": [
                {"name": "Redis Client", "type": "Library", "description": "x"},
                {"name": "Vector DB", "type": "Concept", "description": "y"},
                {"name": "REST API", "type": "API_OR_INTERFACE", "description": "z"}
            ],
            "relationships": []
        }"#;

        let (entities, _) = extractor
            .parse_gleaning_response(response, "chunk-1")
            .expect("parse");

        assert_eq!(entities[0].entity_type, "OTHER");
        assert_eq!(entities[1].entity_type, "OTHER");
        assert_eq!(entities[2].entity_type, "API_OR_INTERFACE");
    }

    #[test]
    fn gleaning_post_merge_reapplies_schema() {
        let extractor = GleaningExtractor::new(
            Arc::new(edgequake_llm::MockProvider::new()),
            Arc::new(crate::extractor::LLMExtractor::new(Arc::new(
                edgequake_llm::MockProvider::new(),
            ))),
        )
        .with_entity_schema(api_strict_schema());

        let mut entities = vec![ExtractedEntity::new("FOO", "Unknown", "d")];

        extractor.apply_entity_schema_to_entities(&mut entities);
        assert_eq!(entities[0].entity_type, "OTHER");
    }

    #[tokio::test]
    async fn glean_fail_open_keeps_base_extraction() {
        let base = Arc::new(FixedBaseExtractor {
            entities: vec![ExtractedEntity::new("ALPHA", "OTHER", "from base")],
        });
        let glean = GleaningExtractor::new(Arc::new(AlwaysFailLlm), base).with_max_gleaning(2);
        let chunk = TextChunk::new("c1", "Alpha works here.", 0, 0, 20);
        let result = glean.extract(&chunk).await.expect("must succeed fail-open");
        assert_eq!(result.entities.len(), 1);
        assert_eq!(result.entities[0].name, "ALPHA");
        assert_eq!(
            result
                .metadata
                .get("gleaning_iterations")
                .and_then(|v| v.as_u64()),
            Some(0)
        );
    }

    #[tokio::test]
    async fn glean_early_stop_on_empty_iteration() {
        let llm = Arc::new(EmptyGleanLlm {
            calls: AtomicUsize::new(0),
        });
        let base = Arc::new(FixedBaseExtractor {
            entities: vec![ExtractedEntity::new("BETA", "OTHER", "from base")],
        });
        let glean =
            GleaningExtractor::new(llm.clone() as Arc<dyn edgequake_llm::LLMProvider>, base)
                .with_max_gleaning(3);
        let chunk = TextChunk::new("c2", "Beta only.", 0, 0, 10);
        let result = glean.extract(&chunk).await.expect("extract");
        assert_eq!(result.entities.len(), 1);
        // One empty iteration then early stop — not all 3.
        assert_eq!(llm.calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            result
                .metadata
                .get("gleaning_iterations")
                .and_then(|v| v.as_u64()),
            Some(1)
        );
    }

    /// LAW-124-15: gleaning LLM iterations must use GenAI generation SSOT.
    #[test]
    fn gleaning_source_wraps_llm_generation() {
        let src = include_str!("gleaning.rs");
        let prod = src
            .split("mod tests")
            .next()
            .expect("production source before tests");
        assert!(
            prod.contains("with_llm_generation"),
            "gleaning.rs must call with_llm_generation"
        );
        assert!(
            prod.contains("\"extract-entities-glean\""),
            "gleaning must use stable operation name extract-entities-glean"
        );
        assert!(
            prod.contains("LlmGenerationRecord"),
            "gleaning must record usage + I/O via LlmGenerationRecord"
        );
        for cost_key in [
            "gen_ai.usage.cost",
            "langfuse.observation.cost_details",
            "langfuse.observation.cost",
        ] {
            assert!(
                !prod.contains(cost_key),
                "gleaning must not emit cost attr {cost_key}"
            );
        }
    }
}
