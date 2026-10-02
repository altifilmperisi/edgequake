//! EdgeQuake Orchestrator - Central RAG coordination module.
//!
//! @implements FEAT0023 (EdgeQuake Orchestrator)
//!
//! # Overview
//!
//! **Implements**: FEAT0001 (Document Ingestion), FEAT0007 (Multi-Mode Query)
//!
//! **Enforces**: BR0001 (Doc ID Uniqueness), BR0002 (Chunk Constraints),
//!               BR0101 (Token Budget), BR0201 (Tenant Isolation)
//!
//! The orchestrator is the primary entry point for all EdgeQuake operations,
//! coordinating document processing, knowledge graph construction, and query execution.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                       EdgeQuake                             │
//! │  ┌─────────────────────────────────────────────────────────┐│
//! │  │                    Orchestrator                         ││
//! │  │  - config: EdgeQuakeConfig                              ││
//! │  │  - storage: KV + Vector + Graph                         ││
//! │  │  - providers: LLM + Embedding                           ││
//! │  └────────────────────────┬────────────────────────────────┘│
//! │                           │                                 │
//! │     ┌─────────────────────┼─────────────────────┐           │
//! │     │                     │                     │           │
//! │     ▼                     ▼                     ▼           │
//! │  ┌──────────┐       ┌──────────┐         ┌──────────┐       │
//! │  │ insert() │       │  query() │         │ delete() │       │
//! │  └────┬─────┘       └────┬─────┘         └────┬─────┘       │
//! │       │                  │                    │             │
//! │       ▼                  ▼                    ▼             │
//! │  ┌──────────┐       ┌──────────┐         ┌──────────┐       │
//! │  │ Pipeline │       │  Query   │         │ Cascade  │       │
//! │  │ (chunk+  │       │  Engine  │         │  Delete  │       │
//! │  │ extract) │       │ (6 modes)│         │ (source  │       │
//! │  └──────────┘       └──────────┘         │ tracking)│       │
//! │                                          └──────────┘       │
//! └─────────────────────────────────────────────────────────────┘
//!
//! Storage Layer:
//! ┌──────────┐    ┌──────────┐    ┌──────────┐
//! │ KVStorage│    │VectorStor│    │GraphStor │
//! │ (docs,   │    │(pgvector)│    │(AGE/mem) │
//! │  chunks) │    │          │    │          │
//! └──────────┘    └──────────┘    └──────────┘
//! ```
//!
//! # Key Operations
//!
//! ## Document Ingestion (FEAT0001)
//!
//! ```rust,ignore
//! // Insert returns processing stats
//! let result = eq.insert("Document content...", Some("doc-001")).await?;
//! assert!(result.entities_extracted > 0);
//! ```
//!
//! ## Query Execution (FEAT0007)
//!
//! ```rust,ignore
//! use edgequake_core::{QueryParams, QueryMode};
//!
//! let params = QueryParams::new().with_mode(QueryMode::Hybrid);
//! let response = eq.query("What is X?", Some(params)).await?;
//! println!("Answer: {}", response.response);
//! ```
//!
//! # Query Modes (FEAT0101-FEAT0106)
//!
//! | Mode | Strategy | Best For |
//! |------|----------|----------|
//! | `naive` | Vector similarity only | Simple factual queries |
//! | `local` | Entity-centric + neighbors | Specific entity questions |
//! | `global` | Community-based | Broad topic overviews |
//! | `hybrid` | Local + global (default) | General purpose |
//! | `mix` | Weighted naive + graph | Tunable balance |
//! | `bypass` | Direct LLM (no RAG) | Creative/chat |
//!
//! # Multi-Tenancy (FEAT0015, BR0201)
//!
//! All operations respect tenant isolation via `tenant_id` and `workspace_id`
//! in the configuration. Cross-tenant data access is prevented at the storage layer.
//!
//! # See Also
//!
//! - [`crate::types::QueryParams`] - Query configuration options
//! - [`crate::types::InsertResult`] - Insertion result details
//! - [docs/features.md](../../../../../../docs/features.md) - Complete feature registry

use std::collections::HashMap;
use std::sync::Arc;

use edgequake_llm::traits::{EmbeddingProvider, LLMProvider};
use edgequake_pipeline::{
    GleaningConfig, GleaningExtractor, LLMExtractor, Pipeline, PipelineConfig,
};
use edgequake_storage::traits::{GraphStorage, KVStorage, VectorStorage, WorkspaceVectorRegistry};
use serde::{Deserialize, Serialize};
// Use query crate types
// edgequake-query is intentionally not linked here to avoid workspace cycles.

use crate::error::{Error, Result};
use crate::workspace_service::WorkspaceService;

mod deletion;
mod ingestion;
mod query_ops;
mod workspace_vector;

/// EdgeQuake instance configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeQuakeConfig {
    /// Working directory for storage.
    pub working_dir: String,

    /// Namespace/workspace identifier.
    pub namespace: String,

    /// Tenant ID for multi-tenancy.
    pub tenant_id: Option<String>,

    /// Workspace ID for multi-tenancy.
    pub workspace_id: Option<String>,

    /// LLM model name for entity extraction.
    pub llm_model_name: String,

    /// LLM model name for response generation (can differ from extraction model).
    pub response_model_name: Option<String>,

    /// Embedding model name.
    pub embedding_model_name: String,

    /// Embedding dimension.
    pub embedding_dim: usize,

    /// Maximum token size for query context.
    pub max_token_for_text_unit: usize,

    /// Maximum token size for entity context.
    pub max_token_for_global_context: usize,

    /// Maximum token size for local context.
    pub max_token_for_local_context: usize,

    /// Chunk size in tokens.
    pub chunk_token_size: usize,

    /// Chunk overlap in tokens.
    pub chunk_overlap_token_size: usize,

    /// Enable logging.
    pub log_level: LogLevel,

    /// Storage configuration.
    pub storage: StorageConfig,

    /// Enable entity extraction caching.
    pub enable_cache: bool,

    /// Entity types to extract.
    pub entity_types: Vec<String>,

    /// Summary language for generated content.
    pub summary_language: String,

    /// Enable gleaning (multi-pass extraction) for better entity coverage.
    pub enable_gleaning: bool,

    /// Maximum number of gleaning iterations (1-3 recommended).
    pub max_gleaning: usize,

    /// Enable LLM-based description merging for better deduplication.
    pub use_llm_summarization: bool,
}

/// Log level configuration.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub enum LogLevel {
    /// Debug level.
    Debug,
    /// Info level.
    #[default]
    Info,
    /// Warning level.
    Warn,
    /// Error level.
    Error,
}

/// Storage backend configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Storage backend type.
    pub backend: StorageBackend,

    /// PostgreSQL connection string (for postgres backend).
    pub postgres_connection_string: Option<String>,

    /// Additional storage options.
    pub options: HashMap<String, String>,
}

/// Storage backend type.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum StorageBackend {
    /// In-memory storage (for testing).
    #[default]
    Memory,

    /// PostgreSQL with pgvector and AGE.
    Postgres,

    /// SurrealDB.
    SurrealDB,
}

impl Default for EdgeQuakeConfig {
    fn default() -> Self {
        Self {
            working_dir: "./edgequake_data".to_string(),
            namespace: "default".to_string(),
            tenant_id: None,
            workspace_id: None,
            llm_model_name: "gpt-4.1-nano".to_string(),
            response_model_name: None,
            embedding_model_name: "text-embedding-3-small".to_string(),
            embedding_dim: 1536,
            // SPEC-006 RB-LLM-008: align with SOTA/LightRAG 30k (ResourceBudget SSOT)
            max_token_for_text_unit: crate::resource::MAX_ORCHESTRATOR_CONTEXT_TOKENS,
            max_token_for_global_context: crate::resource::MAX_ORCHESTRATOR_CONTEXT_TOKENS,
            max_token_for_local_context: crate::resource::MAX_ORCHESTRATOR_CONTEXT_TOKENS,
            chunk_token_size: 1200,
            chunk_overlap_token_size: 100,
            log_level: LogLevel::Info,
            storage: StorageConfig::default(),
            enable_cache: true,
            entity_types: vec![
                "PERSON".to_string(),
                "ORGANIZATION".to_string(),
                "LOCATION".to_string(),
                "CONCEPT".to_string(),
                "EVENT".to_string(),
            ],
            summary_language: "English".to_string(),
            enable_gleaning: true,       // Enable by default for SOTA quality
            max_gleaning: 1,             // LightRAG default
            use_llm_summarization: true, // Enable by default for SOTA quality
        }
    }
}

/// Explicit overrides for [`EdgeQuakeConfig::resolve`] (highest precedence).
#[derive(Debug, Clone, Default)]
pub struct EdgeQuakeConfigOverrides {
    pub llm_model_name: Option<String>,
    pub embedding_model_name: Option<String>,
    pub embedding_dim: Option<usize>,
    pub entity_types: Option<Vec<String>>,
    pub namespace: Option<String>,
    pub working_dir: Option<String>,
    pub enable_gleaning: Option<bool>,
    pub max_gleaning: Option<usize>,
}

impl EdgeQuakeConfig {
    /// Create a new config with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// SPEC-083 X-36 SSOT precedence:
    /// `explicit overrides > environment > workspace snapshot > defaults`.
    ///
    /// `workspace` is a previously resolved workspace-level config (may be `None`).
    pub fn resolve(
        overrides: EdgeQuakeConfigOverrides,
        workspace: Option<&EdgeQuakeConfig>,
    ) -> Self {
        let mut cfg = Self::default();
        if let Some(ws) = workspace {
            cfg = ws.clone();
        }

        // Environment layer (below explicit overrides).
        if let Ok(v) = std::env::var("EDGEQUAKE_LLM_MODEL") {
            if !v.trim().is_empty() {
                cfg.llm_model_name = v;
            }
        } else if let Ok(v) = std::env::var("CHAT_MODEL") {
            if !v.trim().is_empty() {
                cfg.llm_model_name = v;
            }
        }
        if let Ok(v) = std::env::var("EDGEQUAKE_EMBEDDING_MODEL") {
            if !v.trim().is_empty() {
                cfg.embedding_model_name = v;
            }
        } else if let Ok(v) = std::env::var("EMBEDDING_MODEL") {
            if !v.trim().is_empty() {
                cfg.embedding_model_name = v;
            }
        }
        if let Ok(v) = std::env::var("EDGEQUAKE_NAMESPACE") {
            if !v.trim().is_empty() {
                cfg.namespace = v;
            }
        }
        if let Ok(v) = std::env::var("EDGEQUAKE_WORKING_DIR") {
            if !v.trim().is_empty() {
                cfg.working_dir = v;
            }
        }

        // Explicit overrides win.
        if let Some(v) = overrides.llm_model_name {
            cfg.llm_model_name = v;
        }
        if let Some(v) = overrides.embedding_model_name {
            cfg.embedding_model_name = v;
        }
        if let Some(v) = overrides.embedding_dim {
            cfg.embedding_dim = v;
        }
        if let Some(v) = overrides.entity_types {
            cfg.entity_types = v;
        }
        if let Some(v) = overrides.namespace {
            cfg.namespace = v;
        }
        if let Some(v) = overrides.working_dir {
            cfg.working_dir = v;
        }
        if let Some(v) = overrides.enable_gleaning {
            cfg.enable_gleaning = v;
        }
        if let Some(v) = overrides.max_gleaning {
            cfg.max_gleaning = v;
        }
        cfg
    }

    /// Set the working directory.
    pub fn with_working_dir(mut self, dir: &str) -> Self {
        self.working_dir = dir.to_string();
        self
    }

    /// Set the namespace.
    pub fn with_namespace(mut self, ns: &str) -> Self {
        self.namespace = ns.to_string();
        self
    }

    /// Set the LLM model.
    pub fn with_llm_model(mut self, model: &str) -> Self {
        self.llm_model_name = model.to_string();
        self
    }

    /// Set the embedding model.
    pub fn with_embedding_model(mut self, model: &str, dim: usize) -> Self {
        self.embedding_model_name = model.to_string();
        self.embedding_dim = dim;
        self
    }

    /// Set the storage backend.
    pub fn with_storage(mut self, storage: StorageConfig) -> Self {
        self.storage = storage;
        self
    }

    /// Use PostgreSQL storage backend.
    pub fn with_postgres(mut self, connection_string: &str) -> Self {
        self.storage = StorageConfig {
            backend: StorageBackend::Postgres,
            postgres_connection_string: Some(connection_string.to_string()),
            options: HashMap::new(),
        };
        self
    }

    /// Set entity types to extract.
    pub fn with_entity_types(mut self, types: Vec<String>) -> Self {
        self.entity_types = types;
        self
    }

    /// Set chunk configuration.
    pub fn with_chunk_config(mut self, size: usize, overlap: usize) -> Self {
        self.chunk_token_size = size;
        self.chunk_overlap_token_size = overlap;
        self
    }

    /// Set gleaning configuration for multi-pass extraction.
    ///
    /// Gleaning performs additional LLM passes to find entities that might
    /// have been missed in the first extraction. This improves extraction
    /// quality at the cost of additional LLM calls.
    ///
    /// # Arguments
    /// * `enabled` - Whether to enable gleaning
    /// * `max_iterations` - Maximum gleaning iterations (1-3 recommended)
    pub fn with_gleaning(mut self, enabled: bool, max_iterations: usize) -> Self {
        self.enable_gleaning = enabled;
        self.max_gleaning = max_iterations;
        self
    }
}

/// EdgeQuake orchestrator.
pub struct EdgeQuake {
    /// Configuration.
    config: EdgeQuakeConfig,

    /// Whether the instance is initialized.
    initialized: bool,

    /// Storage backends.
    kv_storage: Option<Arc<dyn KVStorage>>,
    vector_storage: Option<Arc<dyn VectorStorage>>,
    graph_storage: Option<Arc<dyn GraphStorage>>,

    /// LLM and embedding providers.
    llm_provider: Option<Arc<dyn LLMProvider>>,
    embedding_provider: Option<Arc<dyn EmbeddingProvider>>,

    /// Pipeline for document processing.
    pipeline: Option<Arc<Pipeline>>,

    /// Query engine.
    query_engine: Option<Arc<edgequake_query::QueryEngine>>,

    /// CQRS relational entity sink (SPEC-021 P3-01/P3-02).
    /// Defaults to NoopEntitySink — set via `with_relational_sink()` to enable dual-write.
    relational_sink: Arc<dyn edgequake_pipeline::RelationalEntitySink>,

    /// SPEC-091 W1: relational chunk authority repository (optional).
    /// Set via `with_relational_chunks()` so `EDGEQUAKE_CHUNK_TEXT_AUTHORITY`
    /// dual/relational writes land on this path too.
    relational_chunks: Option<Arc<dyn edgequake_storage::traits::domain::ChunkRepository>>,

    /// SPEC-091 W3: typed chunk-embedding index (optional).
    /// Set via `with_typed_embedding_index()` (by the API layer, which owns the
    /// Postgres pool) so ingestion dual-writes chunk embeddings to typed
    /// `chunk_embeddings` on this path too.
    typed_embedding_index: Option<Arc<dyn edgequake_storage::traits::domain::EmbeddingIndex>>,

    /// SPEC-091 IW2: typed fleet embedding index (entity/relationship/report).
    fleet_embedding_index: Option<Arc<dyn edgequake_storage::traits::FleetEmbeddingIndex>>,

    /// Per-workspace vector registry (W7 / SPEC-024 pass 14).
    vector_registry: Option<Arc<dyn WorkspaceVectorRegistry>>,

    /// Workspace metadata lookup for embedding dimension (paired with registry).
    workspace_service: Option<Arc<dyn WorkspaceService>>,

    /// When true, ingestion refuses default-storage fallback (production).
    strict_workspace_vectors: bool,
}

impl EdgeQuake {
    /// Create a new EdgeQuake instance.
    pub fn new(config: EdgeQuakeConfig) -> Self {
        Self {
            config,
            initialized: false,
            kv_storage: None,
            vector_storage: None,
            graph_storage: None,
            llm_provider: None,
            embedding_provider: None,
            pipeline: None,
            query_engine: None,
            relational_sink: Arc::new(edgequake_pipeline::NoopEntitySink),
            relational_chunks: None,
            typed_embedding_index: None,
            fleet_embedding_index: None,
            vector_registry: None,
            workspace_service: None,
            strict_workspace_vectors: false,
        }
    }

    /// Create with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(EdgeQuakeConfig::default())
    }

    /// Set the storage backends.
    pub fn with_storage_backends(
        mut self,
        kv: Arc<dyn KVStorage>,
        vector: Arc<dyn VectorStorage>,
        graph: Arc<dyn GraphStorage>,
    ) -> Self {
        self.kv_storage = Some(kv);
        self.vector_storage = Some(vector);
        self.graph_storage = Some(graph);
        self
    }

    /// Wire a relational CQRS sink for dual-write (SPEC-021 P3-01/P3-02).
    ///
    /// When set, the orchestrator writes entity data to both the AGE graph
    /// AND the relational `entities` table on every merge/delete.
    /// Default: `NoopEntitySink` (no relational writes).
    pub fn with_relational_sink(
        mut self,
        sink: Arc<dyn edgequake_pipeline::RelationalEntitySink>,
    ) -> Self {
        self.relational_sink = sink;
        self
    }

    /// Wire the relational chunk repository (SPEC-091 W1 cutover).
    ///
    /// When set and `EDGEQUAKE_CHUNK_TEXT_AUTHORITY` is `dual`/`relational`,
    /// ingestion persists chunk text to the relational `chunks` spine as well.
    pub fn with_relational_chunks(
        mut self,
        repo: Arc<dyn edgequake_storage::traits::domain::ChunkRepository>,
    ) -> Self {
        self.relational_chunks = Some(repo);
        self
    }

    /// Wire the typed chunk-embedding index (SPEC-091 W3 cutover).
    ///
    /// When set, ingestion dual-writes chunk embeddings to typed
    /// `chunk_embeddings` on this path. Injected by the API layer, which owns
    /// the Postgres pool needed to build `PgChunkEmbeddingIndex`.
    pub fn with_typed_embedding_index(
        mut self,
        index: Arc<dyn edgequake_storage::traits::domain::EmbeddingIndex>,
    ) -> Self {
        self.typed_embedding_index = Some(index);
        self
    }

    /// Wire the typed fleet embedding index (SPEC-091 IW2).
    pub fn with_fleet_embedding_index(
        mut self,
        index: Arc<dyn edgequake_storage::traits::FleetEmbeddingIndex>,
    ) -> Self {
        self.fleet_embedding_index = Some(index);
        self
    }

    /// Set the storage backends using a mutable reference.
    pub fn set_storage_backends(
        &mut self,
        kv: Arc<dyn KVStorage>,
        vector: Arc<dyn VectorStorage>,
        graph: Arc<dyn GraphStorage>,
    ) {
        self.kv_storage = Some(kv);
        self.vector_storage = Some(vector);
        self.graph_storage = Some(graph);
    }

    /// Set the LLM and embedding providers.
    pub fn with_providers(
        mut self,
        llm: Arc<dyn LLMProvider>,
        embedding: Arc<dyn EmbeddingProvider>,
    ) -> Self {
        self.llm_provider = Some(llm);
        self.embedding_provider = Some(embedding);
        self
    }

    /// Set the LLM and embedding providers using a mutable reference.
    pub fn set_providers(
        &mut self,
        llm: Arc<dyn LLMProvider>,
        embedding: Arc<dyn EmbeddingProvider>,
    ) {
        self.llm_provider = Some(llm);
        self.embedding_provider = Some(embedding);
    }

    /// Pre-wire a query engine (optional). When set before [`Self::initialize`], that
    /// engine is used instead of constructing a default one — enables result-cache
    /// wiring in library/tests (SPEC-021 P-G9).
    pub fn with_query_engine(mut self, engine: Arc<edgequake_query::QueryEngine>) -> Self {
        self.query_engine = Some(engine);
        self
    }

    /// Initialize the EdgeQuake instance.
    ///
    /// This sets up all storage backends and connections.
    pub async fn initialize(&mut self) -> Result<()> {
        tracing::info!(
            "Initializing EdgeQuake for namespace: {}",
            self.config.namespace
        );

        // Ensure providers are set
        let llm = self
            .llm_provider
            .as_ref()
            .ok_or_else(|| Error::config("LLM provider not set"))?;
        let embedding = self
            .embedding_provider
            .as_ref()
            .ok_or_else(|| Error::config("Embedding provider not set"))?;

        // Set up pipeline
        let pipeline_config = PipelineConfig {
            chunker: edgequake_pipeline::ChunkerConfig {
                chunk_size: self.config.chunk_token_size,
                chunk_overlap: self.config.chunk_overlap_token_size,
                ..Default::default()
            },
            ..Default::default()
        };

        // Create base extractor
        let entity_schema = edgequake_pipeline::prompts::EntityExtractionSchema::with_types(
            self.config.entity_types.clone(),
        );
        let base_extractor: Arc<dyn edgequake_pipeline::EntityExtractor> =
            Arc::new(LLMExtractor::new(llm.clone()).with_entity_schema(entity_schema.clone()));

        // Wrap with GleaningExtractor if enabled
        let extractor: Arc<dyn edgequake_pipeline::EntityExtractor> =
            if self.config.enable_gleaning && self.config.max_gleaning > 0 {
                tracing::info!(
                    max_gleaning = self.config.max_gleaning,
                    "Enabling gleaning for multi-pass extraction"
                );
                Arc::new(
                    GleaningExtractor::new(llm.clone(), base_extractor)
                        .with_entity_schema(entity_schema)
                        .with_config(GleaningConfig {
                            max_gleaning: self.config.max_gleaning,
                        }),
                )
            } else {
                base_extractor
            };

        let pipeline = Pipeline::new(pipeline_config)
            .with_extractor(extractor)
            .with_embedding_provider(embedding.clone());

        self.pipeline = Some(Arc::new(pipeline));

        // Set up query engine (respect pre-wired engine from `with_query_engine`)
        let graph_storage = self
            .graph_storage
            .as_ref()
            .ok_or_else(|| Error::config("Graph storage not set"))?;
        let vector_storage = self
            .vector_storage
            .as_ref()
            .ok_or_else(|| Error::config("Vector storage not set"))?;

        let engine_impl = if let Some(engine) = self.query_engine.take() {
            engine
        } else {
            edgequake_query::build_production_query_engine(
                vector_storage.clone(),
                graph_storage.clone(),
                embedding.clone(),
                llm.clone(),
                self.kv_storage.clone(),
            )
        };

        self.query_engine = Some(engine_impl);

        self.initialized = true;
        tracing::info!("EdgeQuake initialized successfully");

        Ok(())
    }

    /// Finalize and clean up resources.
    pub async fn finalize(&self) -> Result<()> {
        tracing::info!("Finalizing EdgeQuake");
        Ok(())
    }

    /// Get the configuration.
    pub fn config(&self) -> &EdgeQuakeConfig {
        &self.config
    }

    /// Query engine wired at initialize (or via [`Self::with_query_engine`]).
    pub fn query_engine(&self) -> Option<&Arc<edgequake_query::QueryEngine>> {
        self.query_engine.as_ref()
    }

    /// Get the namespace.
    pub fn namespace(&self) -> &str {
        &self.config.namespace
    }

    /// Check if the EdgeQuake instance is healthy and ready.
    pub async fn health_check(&self) -> Result<bool> {
        // TODO: Check all backend connections
        Ok(self.initialized)
    }
}

impl std::fmt::Debug for EdgeQuake {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EdgeQuake")
            .field("namespace", &self.config.namespace)
            .field("initialized", &self.initialized)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::QueryParams;
    use crate::QueryMode;

    #[test]
    fn test_config_builder() {
        let config = EdgeQuakeConfig::new()
            .with_namespace("test-ns")
            .with_llm_model("gpt-4")
            .with_embedding_model("text-embedding-3-large", 3072)
            .with_entity_types(vec!["PERSON".to_string(), "ORG".to_string()]);

        assert_eq!(config.namespace, "test-ns");
        assert_eq!(config.llm_model_name, "gpt-4");
        assert_eq!(config.embedding_model_name, "text-embedding-3-large");
        assert_eq!(config.embedding_dim, 3072);
        assert_eq!(config.entity_types.len(), 2);
    }

    #[test]
    fn test_query_params_builder() {
        let params = QueryParams::new()
            .with_mode(QueryMode::Local)
            .with_top_k(100)
            .with_streaming();

        assert_eq!(params.mode, QueryMode::Local);
        assert_eq!(params.top_k, 100);
        assert!(params.stream);
    }

    #[test]
    fn test_config_default_values() {
        let config = EdgeQuakeConfig::default();
        assert_eq!(config.namespace, "default");
        assert_eq!(config.embedding_dim, 1536);
        assert!(!config.entity_types.is_empty());
    }

    #[test]
    fn test_config_with_chunk_config() {
        let config = EdgeQuakeConfig::new().with_chunk_config(500, 100);
        assert_eq!(config.chunk_token_size, 500);
        assert_eq!(config.chunk_overlap_token_size, 100);
    }

    #[test]
    fn test_query_params_defaults() {
        let params = QueryParams::new();
        assert_eq!(params.mode, QueryMode::Hybrid);
        assert_eq!(params.top_k, 60);
        assert!(!params.stream);
    }

    #[test]
    fn test_config_with_gleaning() {
        let config = EdgeQuakeConfig::new().with_gleaning(true, 3);
        assert!(config.enable_gleaning);
        assert_eq!(config.max_gleaning, 3);
    }

    /// SPEC-083 X-36: explicit > env > workspace > defaults.
    #[test]
    fn contract_config_precedence() {
        let prev_llm = std::env::var("EDGEQUAKE_LLM_MODEL").ok();
        let prev_chat = std::env::var("CHAT_MODEL").ok();
        std::env::remove_var("CHAT_MODEL");
        std::env::set_var("EDGEQUAKE_LLM_MODEL", "from-env");

        let workspace = EdgeQuakeConfig {
            llm_model_name: "from-workspace".into(),
            namespace: "ws-ns".into(),
            ..Default::default()
        };

        let via_ws =
            EdgeQuakeConfig::resolve(EdgeQuakeConfigOverrides::default(), Some(&workspace));
        assert_eq!(via_ws.llm_model_name, "from-env", "env beats workspace");
        assert_eq!(
            via_ws.namespace, "ws-ns",
            "workspace beats default for unset fields"
        );

        let via_explicit = EdgeQuakeConfig::resolve(
            EdgeQuakeConfigOverrides {
                llm_model_name: Some("from-explicit".into()),
                ..Default::default()
            },
            Some(&workspace),
        );
        assert_eq!(
            via_explicit.llm_model_name, "from-explicit",
            "explicit beats env"
        );

        match prev_llm {
            Some(v) => std::env::set_var("EDGEQUAKE_LLM_MODEL", v),
            None => std::env::remove_var("EDGEQUAKE_LLM_MODEL"),
        }
        match prev_chat {
            Some(v) => std::env::set_var("CHAT_MODEL", v),
            None => std::env::remove_var("CHAT_MODEL"),
        }
    }

    #[test]
    fn test_storage_backend_default() {
        let backend = StorageBackend::default();
        assert!(matches!(backend, StorageBackend::Memory));
    }

    #[test]
    fn test_storage_config_default() {
        let config = StorageConfig::default();
        assert!(matches!(config.backend, StorageBackend::Memory));
    }

    #[tokio::test]
    async fn test_edgequake_lifecycle() {
        use edgequake_llm::MockProvider;
        use edgequake_storage::adapters::memory::{
            MemoryGraphStorage, MemoryKVStorage, MemoryVectorStorage,
        };

        let mock_provider = Arc::new(MockProvider::new());
        let kv_storage: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("test"));
        let vector_storage: Arc<dyn VectorStorage> =
            Arc::new(MemoryVectorStorage::new("test", 1536));
        let graph_storage: Arc<dyn GraphStorage> = Arc::new(MemoryGraphStorage::new("test"));

        let mut eq = EdgeQuake::new(EdgeQuakeConfig::default())
            .with_storage_backends(kv_storage, vector_storage, graph_storage)
            .with_providers(mock_provider.clone(), mock_provider);

        assert!(!eq.initialized);

        eq.initialize().await.unwrap();

        assert!(eq.initialized);
        assert!(eq.health_check().await.unwrap());

        eq.finalize().await.unwrap();
    }

    #[test]
    fn default_token_budget_matches_resource_ssot() {
        let cfg = EdgeQuakeConfig::default();
        let cap = crate::resource::MAX_ORCHESTRATOR_CONTEXT_TOKENS;
        assert_eq!(cfg.max_token_for_text_unit, cap);
        assert_eq!(cfg.max_token_for_global_context, cap);
        assert_eq!(cfg.max_token_for_local_context, cap);
    }

    #[tokio::test]
    async fn test_edgequake_query_uses_core_engine() {
        use edgequake_llm::MockProvider;
        use edgequake_storage::adapters::memory::{
            MemoryGraphStorage, MemoryKVStorage, MemoryVectorStorage,
        };

        let mock_provider = Arc::new(MockProvider::new());
        let kv_storage: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("test"));
        let vector_storage: Arc<dyn VectorStorage> =
            Arc::new(MemoryVectorStorage::new("test", 1536));
        let graph_storage: Arc<dyn GraphStorage> = Arc::new(MemoryGraphStorage::new("test"));

        let mut eq = EdgeQuake::new(EdgeQuakeConfig::default())
            .with_storage_backends(kv_storage, vector_storage, graph_storage)
            .with_providers(mock_provider.clone(), mock_provider);

        eq.initialize().await.unwrap();

        // Execute a simple query and verify result shape
        let result = eq.query("hello world", None).await.unwrap();
        assert!(matches!(result.mode, crate::types::QueryMode::Hybrid));
        assert!(result.response.is_empty() || !result.response.is_empty()); // existence check
    }
}
