//! Document processing pipeline.
//!
//! ## Architecture
//!
//! - `config`: Pipeline configuration and env-var defaults
//! - `types`: Result types and progress callbacks
//! - `extraction`: Parallel and resilient chunk extraction
//! - `helpers`: Stats, embeddings, lineage submodules
//! - `processing`: Document processing entry points

mod config;
mod extraction;
pub(crate) mod helpers;
// SPEC-047 P6: unique-before-embed helpers (integration-test surface).
pub use helpers::unique_embed;
pub mod admission_resolver;
mod processing;
mod types;

pub use admission_resolver::{
    queue_target_wait_secs_from_env, resolve as resolve_admission_plan, AdmissionPlan,
    ProviderKind, ProviderProfile, DEFAULT_QUEUE_TARGET_WAIT_SECS, QUEUE_TARGET_WAIT_SECS_ENV,
};

use std::sync::Arc;

use edgequake_llm::traits::EmbeddingProvider;

use crate::chunker::{resolve_chunker, resolve_chunker_with_embedder, Chunker};
use crate::extractor::EntityExtractor;

pub use config::{
    allow_local_gleaning, allow_local_high_concurrency, apply_local_concurrency_safety_clamp,
    cap_for_local_provider, clamp_max_concurrent_extractions, clamp_max_gleaning,
    classify_extract_error, default_chunk_timeout_for_provider,
    default_max_concurrent_for_provider, extract_retry_budget, is_local_extraction_provider,
    is_local_provider_overload_error, resolve_extract_provider_name_for_fairness,
    resolve_extract_provider_name_for_fairness_from, resolve_gleaning_for_provider,
    resolve_worker_pool_limits, resolve_worker_pool_limits_from, retry_delay_ms_for_chunk_error,
    ExtractErrorClass, IngestProfile, PipelineConfig, WorkerPoolLimits,
    ALLOW_LOCAL_HIGH_CONCURRENCY_ENV, DEFAULT_CHUNK_MAX_RETRIES, DEFAULT_CHUNK_TIMEOUT_SECS,
    DEFAULT_INITIAL_RETRY_DELAY_MS, DEFAULT_MAX_CONCURRENT_EXTRACTIONS, LOCAL_CHUNK_TIMEOUT_SECS,
    LOCAL_DEFAULT_LIFECYCLE_TASKS_PER_TENANT, LOCAL_ENABLE_GLEANING_ENV,
    LOCAL_MAX_CONCURRENT_EXTRACTIONS, LOCAL_MAX_INGEST_TASKS_PER_TENANT_CAP,
    LOCAL_MAX_LIFECYCLE_TASKS_PER_TENANT_CAP, LOCAL_OVERLOAD_RETRY_DELAY_MS,
    LOCAL_WORKER_THREADS_CAP, MAX_CHUNK_MAX_RETRIES, MAX_CONCURRENT_EXTRACTIONS_CAP,
    MAX_GLEANING_CAP, MAX_RETRY_DELAY_MS, MIN_CHUNK_TIMEOUT_SECS,
};
pub use types::{
    ChunkErrorInfo, ChunkExtractedCallback, ChunkProgressCallback, ChunkProgressPhase,
    ChunkProgressUpdate, CostBreakdownStats, EmbedProgressCallback, EmbedProgressUpdate,
    ProcessingResult, ProcessingStats,
};

/// Document processing pipeline.
pub struct Pipeline {
    pub(super) config: PipelineConfig,
    pub(super) chunker: Chunker,
    pub(super) extractor: Option<Arc<dyn EntityExtractor>>,
    pub(super) embedding_provider: Option<Arc<dyn EmbeddingProvider>>,
}

impl Pipeline {
    /// Create a new pipeline with the given configuration.
    pub fn new(config: PipelineConfig) -> Self {
        tracing::info!(
            chunk_timeout_secs = config.chunk_extraction_timeout_secs,
            max_retries = config.chunk_max_retries,
            retry_delay_ms = config.initial_retry_delay_ms,
            max_concurrent = config.max_concurrent_extractions,
            "Pipeline created (effective extraction config)"
        );
        let chunker = resolve_chunker(config.chunk_strategy, config.chunker.clone());

        Self {
            config,
            chunker,
            extractor: None,
            embedding_provider: None,
        }
    }

    /// Create a pipeline with default configuration, honouring environment variables.
    pub fn default_pipeline() -> Self {
        Self::new(PipelineConfig::from_env())
    }

    /// Set the entity extractor.
    pub fn with_extractor(mut self, extractor: Arc<dyn EntityExtractor>) -> Self {
        self.extractor = Some(extractor);
        self
    }

    /// Set the embedding provider (caps chunk_size to fit model context when needed).
    ///
    /// SPEC-046: when `chunk_strategy` is Semantic, re-resolves the chunker with
    /// this provider so V-mode breakpoints use real embeddings (DIP).
    pub fn with_embedding_provider(mut self, provider: Arc<dyn EmbeddingProvider>) -> Self {
        let max_tokens = provider.max_tokens();
        if max_tokens > 0 {
            let max_safe = (max_tokens / 2).max(100);
            if self.config.chunker.chunk_size > max_safe {
                tracing::info!(
                    current_chunk_size = self.config.chunker.chunk_size,
                    capped_chunk_size = max_safe,
                    embedding_max_tokens = max_tokens,
                    "Reducing chunk_size to fit embedding model context (2x safety margin)"
                );
                self.config.chunker.chunk_size = max_safe;
            }
        }
        self.embedding_provider = Some(provider.clone());
        // Always re-bind chunker when Semantic so embedder is not lost after size cap.
        if self.config.chunk_strategy.requires_embeddings() || max_tokens > 0 {
            self.chunker = resolve_chunker_with_embedder(
                self.config.chunk_strategy,
                self.config.chunker.clone(),
                Some(provider),
            );
        }
        self
    }

    /// Get the pipeline configuration.
    pub fn config(&self) -> &PipelineConfig {
        &self.config
    }

    /// Get the chunker.
    pub fn chunker(&self) -> &Chunker {
        &self.chunker
    }

    /// Get the extractor.
    pub fn extractor(&self) -> Option<Arc<dyn EntityExtractor>> {
        self.extractor.clone()
    }

    /// Get the embedding provider.
    pub fn embedding_provider(&self) -> Option<Arc<dyn EmbeddingProvider>> {
        self.embedding_provider.clone()
    }
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::default_pipeline()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extractor::SimpleExtractor;

    #[tokio::test]
    async fn test_pipeline_basic_processing() {
        let pipeline = Pipeline::default_pipeline();

        let result = pipeline
            .process("doc-1", "This is a test document with some content.")
            .await
            .unwrap();

        assert_eq!(result.document_id, "doc-1");
        assert!(!result.chunks.is_empty());
    }

    #[tokio::test]
    async fn test_pipeline_with_extractor() {
        let extractor = Arc::new(SimpleExtractor::default());
        let pipeline = Pipeline::default_pipeline().with_extractor(extractor);

        let result = pipeline
            .process("doc-1", "John Doe works at Acme Corp in New York.")
            .await
            .unwrap();

        assert!(result.stats.llm_calls > 0);
    }

    #[tokio::test]
    async fn test_pipeline_batch_processing() {
        let pipeline = Pipeline::default_pipeline();

        let documents = vec![
            ("doc-1".to_string(), "First document content.".to_string()),
            ("doc-2".to_string(), "Second document content.".to_string()),
        ];

        let results = pipeline.process_batch(&documents).await.unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].document_id, "doc-1");
        assert_eq!(results[1].document_id, "doc-2");
    }

    #[test]
    fn test_pipeline_config_defaults() {
        let config = PipelineConfig::default();

        assert_eq!(config.extraction_batch_size, 10);
        assert!(config.enable_entity_extraction);
        assert!(config.enable_chunk_embeddings);
        assert!(config.enable_lineage_tracking);
    }

    #[tokio::test]
    async fn test_pipeline_with_lineage_tracking() {
        let extractor = Arc::new(SimpleExtractor::default());
        let config = PipelineConfig::default();
        assert!(config.enable_lineage_tracking);

        let pipeline = Pipeline::new(config).with_extractor(extractor);

        let result = pipeline
            .process("doc-1", "John Doe works at Acme Corp in New York.")
            .await
            .unwrap();

        assert!(result.lineage.is_some());

        let lineage = result.lineage.unwrap();
        assert_eq!(lineage.document_id, "doc-1");
        assert!(!lineage.chunks.is_empty());
        assert_eq!(lineage.total_chunks, result.chunks.len());
    }

    #[tokio::test]
    async fn test_pipeline_without_lineage_tracking() {
        let config = PipelineConfig {
            enable_lineage_tracking: false,
            ..Default::default()
        };
        let pipeline = Pipeline::new(config);

        let result = pipeline
            .process("doc-1", "Simple document content.")
            .await
            .unwrap();

        assert!(result.lineage.is_none());
    }
}
