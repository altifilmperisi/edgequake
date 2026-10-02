//! Ingestion failure taxonomy and retry policy (SPEC-045 SSOT).
//!
//! Single source for classifying permanent ingestion errors so task workers
//! do not waste retry budget on deterministic failures (embedding 400, graph merge).

/// Typed failure classes for document ingestion (SPEC-045 SSOT).
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IngestionFailureClass {
    TimeoutPhaseConvert,
    TimeoutPhaseExtract,
    CircuitBreaker,
    DocumentTooLarge,
    EmbeddingLimit,
    GraphMerge,
    ProviderUnavailable,
    /// Provider misconfiguration: missing/invalid credentials or an unsupported
    /// runtime provider/model selection. Deterministic within a process — the
    /// operator must fix env/config and restart, so it never resolves on retry.
    /// Distinct from transient `ProviderUnavailable` (network blip, local server
    /// momentarily down).
    ProviderMisconfigured,
    /// SPEC-131: model rejected a request parameter (e.g. temperature unsupported_value).
    /// Permanent until operator sets omit env / switches API format / changes model.
    LlmUnsupportedParam,
    /// User/system cancel — terminal, never retry.
    Cancelled,
    /// The document was tombstoned (deleted) by the P0 lifecycle authority but
    /// its physical cleanup never finished. Ingest into it can never succeed:
    /// the tombstone is irreversible, so retry/reprocess only burn LLM spend.
    /// Remedy is to finish the delete (or re-upload the file as a new document).
    DocumentDeleted,
    Unknown,
}

impl IngestionFailureClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TimeoutPhaseConvert => "timeout_phase_convert",
            Self::TimeoutPhaseExtract => "timeout_phase_extract",
            Self::CircuitBreaker => "circuit_breaker",
            Self::DocumentTooLarge => "document_too_large",
            Self::EmbeddingLimit => "embedding_limit",
            Self::GraphMerge => "graph_merge",
            Self::ProviderUnavailable => "provider_unavailable",
            Self::ProviderMisconfigured => "provider_misconfigured",
            Self::LlmUnsupportedParam => "llm_unsupported_param",
            Self::Cancelled => "cancelled",
            Self::DocumentDeleted => "document_deleted",
            Self::Unknown => "unknown",
        }
    }

    /// Parse a structured `failure_class=` / ingestion token (SPEC-083 X-30).
    pub fn from_token(token: &str) -> Option<Self> {
        match token.trim().to_ascii_lowercase().as_str() {
            "timeout_phase_convert" => Some(Self::TimeoutPhaseConvert),
            "timeout_phase_extract" | "timeout" | "extraction_timeout" => {
                Some(Self::TimeoutPhaseExtract)
            }
            "circuit_breaker" | "circuit_breaker_open" => Some(Self::CircuitBreaker),
            "document_too_large" => Some(Self::DocumentTooLarge),
            "embedding_limit" => Some(Self::EmbeddingLimit),
            "graph_merge" => Some(Self::GraphMerge),
            "provider_unavailable" | "rate_limited" => Some(Self::ProviderUnavailable),
            "provider_misconfigured" => Some(Self::ProviderMisconfigured),
            "llm_unsupported_param" => Some(Self::LlmUnsupportedParam),
            "cancelled" => Some(Self::Cancelled),
            "document_deleted" => Some(Self::DocumentDeleted),
            "unknown" => Some(Self::Unknown),
            _ => None,
        }
    }

    pub fn recommended_action(self) -> &'static str {
        match self {
            Self::TimeoutPhaseConvert => "reprocess_edgeparse",
            Self::TimeoutPhaseExtract => "retry_faster_model",
            Self::CircuitBreaker => "reprocess_edgeparse",
            Self::DocumentTooLarge => "split_document",
            Self::EmbeddingLimit => "retry_or_support",
            Self::GraphMerge => "reprocess_full",
            Self::ProviderUnavailable => "reduce_concurrency_or_check_provider",
            Self::ProviderMisconfigured => "configure_provider_credentials",
            Self::LlmUnsupportedParam => "omit_llm_temperature_or_switch_api_format",
            Self::Cancelled => "none",
            Self::DocumentDeleted => "finish_delete_or_reupload",
            Self::Unknown => "retry",
        }
    }

    /// Permanent failures must not consume the task retry budget (SPEC-045 EC-045-09).
    pub fn is_permanent(self) -> bool {
        matches!(
            self,
            Self::CircuitBreaker
                | Self::DocumentTooLarge
                | Self::EmbeddingLimit
                | Self::GraphMerge
                | Self::ProviderMisconfigured
                | Self::LlmUnsupportedParam
                | Self::Cancelled
                | Self::DocumentDeleted
        )
    }
}

/// True when the error is a deterministic provider misconfiguration —
/// missing/invalid credentials or an unsupported runtime provider/model.
///
/// WHY separate from transient `ProviderUnavailable`: a missing `*_API_KEY`,
/// an invalid/incorrect key (HTTP 401), or an unconfigured runtime provider
/// will **never** succeed on retry within the same server process. Retrying
/// only burns the retry budget (exponential backoff) and delays an actionable
/// failure. This must be classified as permanent and surfaced immediately.
///
/// Conservative by design: only fires on explicit configuration/credential
/// markers so genuinely transient "failed to create provider" errors (e.g. a
/// network blip during model discovery) stay retryable as `ProviderUnavailable`.
pub fn is_provider_misconfig_message(error_msg: &str) -> bool {
    let lower = error_msg.to_ascii_lowercase();
    lower.contains("configuration error")
        || lower.contains("api_key is not set")
        || lower.contains("api key is not set")
        || lower.contains("api_key environment variable not set")
        || lower.contains("environment variable not set")
        || lower.contains("is not set. to use")
        || lower.contains("credentials not configured")
        || lower.contains("not configured for this runtime")
        || lower.contains("invalid api key")
        || lower.contains("invalid_api_key")
        || lower.contains("incorrect api key")
        || lower.contains("authentication error")
        || (lower.contains("unauthorized") && lower.contains("api key"))
        // pgvector CheckExpectedDim / Rust typed-embedding write gate (SPEC-091)
        || (lower.contains("expected")
            && lower.contains("dimensions")
            && lower.contains("not"))
        || lower.contains("embedding dimension mismatch")
        || lower.contains("mixed dimensions in one batch")
}

/// True when the error says ingest hit a tombstoned (deleted) document.
///
/// Matches the typed storage conflict raised by the persist authority gate
/// (`cannot ingest into a tombstoned document`) and the worker pre-flight guard.
pub fn is_document_deleted_message(error_msg: &str) -> bool {
    let lower = error_msg.to_ascii_lowercase();
    lower.contains("tombstoned document") || lower.contains("document was deleted")
}

/// True when an error string represents user/system cancel (SPEC-057).
pub fn is_cancel_failure_message(error_msg: &str) -> bool {
    let lower = error_msg.to_ascii_lowercase();
    lower.contains("task cancelled")
        || lower.contains("cancelled by user")
        || lower.contains("cancelled during")
}

/// True when the message carries a **typed** timeout marker (SPEC-083 X-30).
///
/// Prefers [`crate::types::TaskFailureInfo::timeout`]'s fixed message
/// `"Operation timed out"`, structured `failure_class=timeout` tokens, or an
/// explicit `[timeout]` marker — not a bare English substring match on business text.
pub fn is_typed_timeout_message(error_msg: &str) -> bool {
    let lower = error_msg.to_ascii_lowercase();
    lower.starts_with("operation timed out")
        || lower.contains("operation timed out")
        || lower.contains("task processing timed out")
        || lower.contains("task timed out")
        || lower.contains("llm request timed out")
        || lower.contains("request timed out")
        || lower.contains("extraction timeout after")
        || lower.contains("failure_class=timeout")
        || lower.contains("[timeout]")
        || lower.contains("[ingestion_failure_class=timeout")
}

/// Parse structured `failure_class=` / `[ingestion_failure_class=…]` tokens first (X-30).
///
/// Typed Display markers (and API `classify_from_pipeline_error`) emit these so
/// string `contains` taxonomy is never needed when the enum path is available.
pub fn classify_from_failure_markers(error_msg: &str) -> Option<IngestionFailureClass> {
    let lower = error_msg.to_ascii_lowercase();
    // Prefer explicit failure_class=<token> (and bracketed forms).
    for key in ["failure_class=", "ingestion_failure_class="] {
        if let Some(idx) = lower.find(key) {
            let rest = &lower[idx + key.len()..];
            let token: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if let Some(class) = IngestionFailureClass::from_token(&token) {
                return Some(class);
            }
        }
    }
    None
}

/// Classify a permanent failure message into a stable `failure_class` key.
///
/// Order (SPEC-083 X-30):
/// 1. Structured `failure_class=` tokens (typed enum → Display markers)
/// 2. Cancel / misconfig / other stable markers
/// 3. String taxonomy last resort for Unknown-wrapped payloads only
pub fn classify_ingestion_failure(error_msg: &str) -> IngestionFailureClass {
    if let Some(class) = classify_from_failure_markers(error_msg) {
        // Generic timeout token may still need convert-phase refinement from body.
        if matches!(
            class,
            IngestionFailureClass::TimeoutPhaseExtract | IngestionFailureClass::TimeoutPhaseConvert
        ) {
            let lower = error_msg.to_ascii_lowercase();
            if lower.contains("vision") || lower.contains("convert") || lower.contains("markdown") {
                return IngestionFailureClass::TimeoutPhaseConvert;
            }
            return IngestionFailureClass::TimeoutPhaseExtract;
        }
        return class;
    }
    let lower = error_msg.to_ascii_lowercase();
    if is_cancel_failure_message(error_msg) {
        return IngestionFailureClass::Cancelled;
    }
    if is_document_deleted_message(error_msg) {
        return IngestionFailureClass::DocumentDeleted;
    }
    // Deterministic credential/config failure — must precede the transient
    // `ProviderUnavailable` branch (which also matches "failed to create").
    if is_provider_misconfig_message(error_msg) {
        return IngestionFailureClass::ProviderMisconfigured;
    }
    if lower.contains("circuit breaker") {
        return IngestionFailureClass::CircuitBreaker;
    }
    if lower.contains("document too large") || lower.contains("exceeds maximum size") {
        return IngestionFailureClass::DocumentTooLarge;
    }
    if lower.contains("too many inputs")
        || lower.contains("too many tokens")
        || lower.contains("invalid_request_prompt")
        || (lower.contains("embedding") && lower.contains("400"))
    {
        return IngestionFailureClass::EmbeddingLimit;
    }
    if lower.contains("knowledge-graph merge error")
        || lower.contains("merge error(s) during persist")
        || (lower.contains("graph error") && lower.contains("merge"))
        || lower.contains("typed fleet mirror")
        || lower.contains("relational entity/rel fk")
        || lower.contains("fk miss")
        || lower.contains("documents_valid_status")
    {
        return IngestionFailureClass::GraphMerge;
    }
    // X-30: phase convert heuristic only when a typed timeout marker is present.
    if is_typed_timeout_message(error_msg) {
        if lower.contains("vision") || lower.contains("convert") || lower.contains("markdown") {
            return IngestionFailureClass::TimeoutPhaseConvert;
        }
        return IngestionFailureClass::TimeoutPhaseExtract;
    }
    if lower.contains("provider")
        && (lower.contains("unavailable") || lower.contains("failed to create"))
    {
        return IngestionFailureClass::ProviderUnavailable;
    }
    if lower.contains("network error")
        || lower.contains("connection refused")
        || lower.contains("error sending request")
        || lower.contains("localhost:11434")
    {
        return IngestionFailureClass::ProviderUnavailable;
    }
    // SPEC-131: temperature / unsupported_value param rejections (#379).
    if is_llm_unsupported_param_message(error_msg) {
        return IngestionFailureClass::LlmUnsupportedParam;
    }
    IngestionFailureClass::Unknown
}

/// True when upstream rejected an LLM sampling/effort parameter (SPEC-131).
pub fn is_llm_unsupported_param_message(error_msg: &str) -> bool {
    let lower = error_msg.to_ascii_lowercase();
    let has_unsupported = lower.contains("unsupported_value")
        || lower.contains("unsupported_parameter")
        || lower.contains("does not support");
    let has_temperature = lower.contains("temperature")
        || lower.contains("param: temperature")
        || lower.contains("'temperature'");
    (has_unsupported && has_temperature)
        || (lower.contains("unsupported_value") && lower.contains("param:"))
}

/// True when the error will not resolve by retrying the same request.
pub fn is_permanent_ingestion_failure(error_msg: &str) -> bool {
    classify_ingestion_failure(error_msg).is_permanent()
        || error_msg
            .to_ascii_lowercase()
            .contains("invalid_request_prompt")
}

/// Map failure class to task pipeline step for structured errors.
pub fn failure_step(class: IngestionFailureClass) -> &'static str {
    match class {
        IngestionFailureClass::EmbeddingLimit => "embedding",
        IngestionFailureClass::GraphMerge => "indexing",
        IngestionFailureClass::TimeoutPhaseConvert => "pdf_convert",
        IngestionFailureClass::TimeoutPhaseExtract | IngestionFailureClass::CircuitBreaker => {
            "extraction"
        }
        IngestionFailureClass::DocumentTooLarge => "admission",
        IngestionFailureClass::ProviderUnavailable => "extraction",
        IngestionFailureClass::ProviderMisconfigured => "provider_config",
        IngestionFailureClass::LlmUnsupportedParam => "extraction",
        IngestionFailureClass::Cancelled => "cancelled",
        IngestionFailureClass::DocumentDeleted => "admission",
        IngestionFailureClass::Unknown => "processing",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec045_graph_merge_is_permanent() {
        let msg = "Pipeline processing failed: 1 knowledge-graph merge error(s) during persist";
        let class = classify_ingestion_failure(msg);
        assert_eq!(class, IngestionFailureClass::GraphMerge);
        assert!(class.is_permanent());
        assert_eq!(class.recommended_action(), "reprocess_full");
    }

    #[test]
    fn tombstoned_document_is_permanent_and_never_retried() {
        let msg = "Knowledge graph persist failed: Storage error: Conflict: \
                   cannot ingest into a tombstoned document";
        let class = classify_ingestion_failure(msg);
        assert_eq!(class, IngestionFailureClass::DocumentDeleted);
        assert!(is_permanent_ingestion_failure(msg));
        assert_eq!(class.recommended_action(), "finish_delete_or_reupload");
        assert_eq!(class.as_str(), "document_deleted");
    }

    #[test]
    fn document_deleted_marker_roundtrips() {
        let msg = "Document was deleted [failure_class=document_deleted]";
        assert_eq!(
            classify_ingestion_failure(msg),
            IngestionFailureClass::DocumentDeleted
        );
        assert_eq!(
            IngestionFailureClass::from_token("document_deleted"),
            Some(IngestionFailureClass::DocumentDeleted)
        );
    }

    #[test]
    fn spec045_embedding_400_is_permanent() {
        let msg = "Embedding error: API error: Too many inputs in request (400)";
        let class = classify_ingestion_failure(msg);
        assert_eq!(class, IngestionFailureClass::EmbeddingLimit);
        assert!(is_permanent_ingestion_failure(msg));
    }

    #[test]
    fn spec045_provider_error_is_retriable() {
        let msg = "Network error: error sending request for url (http://localhost:11434/api/chat)";
        let class = classify_ingestion_failure(msg);
        assert_eq!(class, IngestionFailureClass::ProviderUnavailable);
        assert!(!class.is_permanent());
        assert_eq!(
            class.recommended_action(),
            "reduce_concurrency_or_check_provider"
        );
    }

    #[test]
    fn missing_api_key_is_permanent_misconfig() {
        // Exact message emitted by the workspace pipeline factory + vision path.
        let msg = "Processing error: Failed to create vision provider 'mistral': \
                   Configuration error: MISTRAL_API_KEY is not set. To use the Mistral \
                   provider, set the environment variable and restart the server.";
        let class = classify_ingestion_failure(msg);
        assert_eq!(class, IngestionFailureClass::ProviderMisconfigured);
        assert!(class.is_permanent());
        assert!(is_permanent_ingestion_failure(msg));
        assert_eq!(class.recommended_action(), "configure_provider_credentials");
        assert_eq!(class.as_str(), "provider_misconfigured");
        assert_eq!(failure_step(class), "provider_config");
    }

    #[test]
    fn embedding_env_var_not_set_is_permanent_misconfig() {
        let msg = "Failed to create LLM (Configuration error: MISTRAL_API_KEY is not set.) \
                   and embedding (Configuration error: MISTRAL_API_KEY environment variable \
                   not set. Get your API key from https://console.mistral.ai) providers";
        assert_eq!(
            classify_ingestion_failure(msg),
            IngestionFailureClass::ProviderMisconfigured
        );
        assert!(is_permanent_ingestion_failure(msg));
    }

    #[test]
    fn invalid_api_key_401_is_permanent_misconfig() {
        let msg = "LLM error: Authentication error: invalid_request_error: Incorrect API key \
                   provided (code: invalid_api_key)";
        assert_eq!(
            classify_ingestion_failure(msg),
            IngestionFailureClass::ProviderMisconfigured
        );
        assert!(is_permanent_ingestion_failure(msg));
    }

    #[test]
    fn pgvector_dimension_mismatch_is_permanent_misconfig() {
        // Exact pgvector CheckExpectedDim wording surfaced via KG persist.
        let msg = "Knowledge graph persist failed: Storage error: \
                   expected 1536 dimensions, not 1024";
        assert_eq!(
            classify_ingestion_failure(msg),
            IngestionFailureClass::ProviderMisconfigured
        );
        assert!(is_permanent_ingestion_failure(msg));
        assert_ne!(
            classify_ingestion_failure(msg),
            IngestionFailureClass::ProviderUnavailable
        );
    }

    #[test]
    fn typed_embedding_gate_mismatch_is_permanent_misconfig() {
        let msg = "embedding dimension mismatch: expected 1024, not 768 (row 0)";
        assert_eq!(
            classify_ingestion_failure(msg),
            IngestionFailureClass::ProviderMisconfigured
        );
    }

    #[test]
    fn typed_fleet_mirror_fk_miss_is_graph_merge() {
        let msg = "Knowledge graph persist failed: Graph error: 1 knowledge-graph merge error(s) \
                   during persist: Storage error: Database error: SPEC-091: typed fleet mirror \
                   resolved 0/25 rows (relational entity/rel FK miss or name mismatch)";
        assert_eq!(
            classify_ingestion_failure(msg),
            IngestionFailureClass::GraphMerge
        );
        assert!(is_permanent_ingestion_failure(msg));
    }

    #[test]
    fn documents_valid_status_violation_is_graph_merge() {
        let msg = "Storage error: Database error: typed document shell batch write failed for \
                   StagingMetadata: new row for relation \"documents\" violates check constraint \
                   \"documents_valid_status\"";
        assert_eq!(
            classify_ingestion_failure(msg),
            IngestionFailureClass::GraphMerge
        );
    }

    #[test]
    fn transient_failed_to_create_provider_stays_retryable() {
        // No credential/config marker → genuinely transient construction failure
        // (e.g. discovery network blip) must remain retryable, not permanent.
        let msg = "Failed to create provider: connection refused";
        let class = classify_ingestion_failure(msg);
        assert_eq!(class, IngestionFailureClass::ProviderUnavailable);
        assert!(!class.is_permanent());
        assert!(!is_permanent_ingestion_failure(msg));
    }

    #[test]
    fn spec045_rate_limit_not_classified_permanent() {
        let msg = "Embedding error: API error: rate limit exceeded (429)";
        assert!(!is_permanent_ingestion_failure(msg));
    }

    #[test]
    fn cancel_is_permanent_non_retryable() {
        let msg = "Task cancelled during 'pre-extraction' stage for document abc";
        let class = classify_ingestion_failure(msg);
        assert_eq!(class, IngestionFailureClass::Cancelled);
        assert!(class.is_permanent());
        assert!(is_permanent_ingestion_failure(msg));
    }

    #[test]
    fn vision_cancel_string_is_cancelled_class() {
        let msg = "Cancelled during vision PDF conversion";
        assert!(is_cancel_failure_message(msg));
        assert_eq!(
            classify_ingestion_failure(msg),
            IngestionFailureClass::Cancelled
        );
        assert!(is_permanent_ingestion_failure(msg));
    }

    /// SPEC-083 X-30: business text mentioning "timeout" must not classify as Timeout.
    #[test]
    fn unit_failure_class_typed() {
        let msg = "Entity description: the project timeout policy is 30 days";
        assert!(!is_typed_timeout_message(msg));
        assert_eq!(
            classify_ingestion_failure(msg),
            IngestionFailureClass::Unknown
        );
        assert_eq!(
            classify_ingestion_failure("Operation timed out: extraction hung"),
            IngestionFailureClass::TimeoutPhaseExtract
        );
        assert_eq!(
            classify_ingestion_failure("Operation timed out during vision convert"),
            IngestionFailureClass::TimeoutPhaseConvert
        );
        // Typed markers win over prose.
        assert_eq!(
            classify_ingestion_failure(
                "wrapper: LLM error: Request timed out [failure_class=timeout_phase_extract]"
            ),
            IngestionFailureClass::TimeoutPhaseExtract
        );
        assert_eq!(
            classify_ingestion_failure("Circuit breaker open … [failure_class=circuit_breaker]"),
            IngestionFailureClass::CircuitBreaker
        );
        assert_eq!(
            IngestionFailureClass::from_token("rate_limited"),
            Some(IngestionFailureClass::ProviderUnavailable)
        );
    }

    /// SPEC-083 X-30: breaker / classify must ignore business "timeout" wording.
    #[test]
    fn unit_breaker_ignores_business_timeout_word() {
        let business = "User asked about query timeout policy in the handbook";
        assert!(!is_typed_timeout_message(business));
        assert_eq!(
            classify_ingestion_failure(business),
            IngestionFailureClass::Unknown
        );
        assert!(classify_from_failure_markers(business).is_none());
        // Bare "429" in prose must not trip provider / rate-limit class.
        assert_eq!(
            classify_ingestion_failure("section 429 of the user guide"),
            IngestionFailureClass::Unknown
        );
    }

    /// SPEC-083 X-30: no bare substring retry/timeout matching in hot paths.
    #[test]
    fn contract_no_substring_retry_matching() {
        let ingestion = include_str!("ingestion_reliability.rs");
        // Inspect production code only (tests may mention the forbidden pattern).
        let prod = ingestion
            .split("#[cfg(test)]")
            .next()
            .expect("production section");
        let bare_timeout = format!("{}{}{}", "contains(", "\"timeout\"", ")");
        assert!(
            !prod.contains(&bare_timeout),
            "X-30: production classify path must not use bare timeout substring matching"
        );
        assert!(
            prod.contains("Operation timed out") && prod.contains("is_typed_timeout_message"),
            "X-30: timeout branch must use Operation timed out / is_typed_timeout_message"
        );

        let embeddings_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../edgequake-pipeline/src/pipeline/helpers/embeddings.rs"
        );
        let embeddings = std::fs::read_to_string(embeddings_path)
            .unwrap_or_else(|e| panic!("read embeddings.rs at {embeddings_path}: {e}"));
        let bare_429 = format!("{}{}{}", "contains(", "\"429\"", ")");
        assert!(
            !embeddings.contains(&bare_429),
            "X-06/X-07: pipeline embeddings must not use bare 429 substring matching"
        );
        assert!(
            embeddings.contains("retry_strategy()"),
            "pipeline embeddings must use typed retry_strategy()"
        );
    }

    /// SPEC-131 U-131-05 / E2E-131-06: #379 temperature unsupported_value.
    #[test]
    fn u131_05_temperature_unsupported_value_is_permanent() {
        let msg = "Unsupported value: 'temperature' does not support 0 with this model.\n\
                   Only the default (1) value is supported.\n\
                   (param: temperature) (code: unsupported_value)";
        let class = classify_ingestion_failure(msg);
        assert_eq!(class, IngestionFailureClass::LlmUnsupportedParam);
        assert!(class.is_permanent());
        assert_eq!(
            class.recommended_action(),
            "omit_llm_temperature_or_switch_api_format"
        );
        assert_eq!(class.as_str(), "llm_unsupported_param");
        assert!(is_llm_unsupported_param_message(msg));
    }
}
