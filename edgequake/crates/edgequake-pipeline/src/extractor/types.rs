//! Extraction domain types (entities, relationships, results).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Result of entity and relationship extraction.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExtractionResult {
    /// Extracted entities.
    pub entities: Vec<ExtractedEntity>,

    /// Extracted relationships.
    pub relationships: Vec<ExtractedRelationship>,

    /// Source chunk ID.
    pub source_chunk_id: String,

    /// Processing metadata.
    pub metadata: HashMap<String, serde_json::Value>,

    /// Input tokens used for this extraction.
    pub input_tokens: usize,

    /// Output tokens generated for this extraction.
    pub output_tokens: usize,

    /// Extraction time in milliseconds.
    pub extraction_time_ms: u64,
}

impl ExtractionResult {
    /// Create a new empty extraction result.
    pub fn new(source_chunk_id: impl Into<String>) -> Self {
        Self {
            entities: Vec::new(),
            relationships: Vec::new(),
            source_chunk_id: source_chunk_id.into(),
            metadata: HashMap::new(),
            input_tokens: 0,
            output_tokens: 0,
            extraction_time_ms: 0,
        }
    }

    /// SPEC-151: re-attach this extraction to a new positional chunk id.
    ///
    /// Updates `source_chunk_id` and entity/relationship source chunk lists
    /// that referenced the previous id.
    pub fn rebind_chunk_id(mut self, new_chunk_id: &str) -> Self {
        let old = self.source_chunk_id.clone();
        self.source_chunk_id = new_chunk_id.to_string();
        for entity in &mut self.entities {
            entity.source_chunk_ids.retain(|id| id != &old);
            if !entity.source_chunk_ids.iter().any(|id| id == new_chunk_id) {
                entity.source_chunk_ids.push(new_chunk_id.to_string());
            }
        }
        for rel in &mut self.relationships {
            rel.source_chunk_ids.retain(|id| id != &old);
            if !rel.source_chunk_ids.iter().any(|id| id == new_chunk_id) {
                rel.add_source_chunk_id(new_chunk_id);
            }
            if rel.source_chunk_id.as_deref() == Some(old.as_str()) {
                rel.source_chunk_id = Some(new_chunk_id.to_string());
            }
        }
        self
    }

    /// Add an entity.
    pub fn add_entity(&mut self, entity: ExtractedEntity) {
        self.entities.push(entity);
    }

    /// Add a relationship.
    pub fn add_relationship(&mut self, rel: ExtractedRelationship) {
        self.relationships.push(rel);
    }

    /// Set token usage information.
    pub fn with_token_usage(mut self, input_tokens: usize, output_tokens: usize) -> Self {
        self.input_tokens = input_tokens;
        self.output_tokens = output_tokens;
        self
    }

    /// Set extraction timing.
    pub fn with_timing(mut self, extraction_time_ms: u64) -> Self {
        self.extraction_time_ms = extraction_time_ms;
        self
    }

    /// Stamp chunk/document lineage on every entity and relationship.
    ///
    /// WHY: Without chunk linkage, Local/Global query cannot find related
    /// chunks and the merger's SPEC-091 RM2 citation gate rejects
    /// relationships (`source_chunk_ids required`). Shared by ingest and
    /// chunk-retry so both paths stay in lockstep.
    pub fn stamp_chunk_lineage(&mut self, document_id: &str) {
        let chunk_id = self.source_chunk_id.clone();
        let derived_doc = if !document_id.is_empty() {
            Some(document_id.to_string())
        } else {
            crate::merger::lineage::document_id_from_chunk_id(&chunk_id)
        };
        tracing::debug!(
            "Linking {} entities and {} relationships to chunk {}",
            self.entities.len(),
            self.relationships.len(),
            chunk_id
        );
        for entity in &mut self.entities {
            entity.add_source_chunk_id(&chunk_id);
            if entity.source_document_id.is_none() {
                if let Some(ref doc) = derived_doc {
                    entity.source_document_id = Some(doc.clone());
                }
            }
        }
        for rel in &mut self.relationships {
            if rel.source_chunk_id.is_none() {
                rel.source_chunk_id = Some(chunk_id.clone());
            }
            if rel.source_document_id.is_none() {
                if let Some(ref doc) = derived_doc {
                    rel.source_document_id = Some(doc.clone());
                }
            }
        }
    }
}

/// An extracted entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEntity {
    /// Entity name (normalized).
    pub name: String,

    /// Entity type (e.g., "PERSON", "ORGANIZATION", "CONCEPT").
    pub entity_type: String,

    /// Description of the entity.
    pub description: String,

    /// Importance score (0.0 to 1.0).
    pub importance: f32,

    /// Source text spans.
    pub source_spans: Vec<String>,

    /// Entity embedding.
    pub embedding: Option<Vec<f32>>,

    /// Source chunk IDs where this entity was mentioned.
    #[serde(default)]
    pub source_chunk_ids: Vec<String>,

    /// Source document ID (the document this entity was extracted from).
    #[serde(default)]
    pub source_document_id: Option<String>,

    /// Original file path of the source document.
    #[serde(default)]
    pub source_file_path: Option<String>,

    /// Human-facing label for multimodal entities (066). Identity stays in `name`.
    #[serde(default)]
    pub display_name: Option<String>,

    /// 1-indexed page for multimodal crops (066).
    #[serde(default)]
    pub page_num: Option<u32>,

    /// Figure index within page when applicable (066).
    #[serde(default)]
    pub figure_index: Option<u32>,

    /// `document_mm_assets.asset_id` stem hint (e.g. `page-0002-fig-01`).
    #[serde(default)]
    pub asset_id: Option<String>,

    /// VLM image type / subtype (Chart, Flowchart, …).
    #[serde(default)]
    pub mm_subtype: Option<String>,
}

impl ExtractedEntity {
    /// Create a new extracted entity.
    pub fn new(
        name: impl Into<String>,
        entity_type: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            entity_type: entity_type.into(),
            description: description.into(),
            importance: 0.5,
            source_spans: Vec::new(),
            embedding: None,
            source_chunk_ids: Vec::new(),
            source_document_id: None,
            source_file_path: None,
            display_name: None,
            page_num: None,
            figure_index: None,
            asset_id: None,
            mm_subtype: None,
        }
    }

    /// Set the importance score.
    pub fn with_importance(mut self, importance: f32) -> Self {
        self.importance = importance.clamp(0.0, 1.0);
        self
    }

    /// Add a source span.
    pub fn with_source_span(mut self, span: impl Into<String>) -> Self {
        self.source_spans.push(span.into());
        self
    }

    /// Add a source chunk ID.
    pub fn with_source_chunk_id(mut self, chunk_id: impl Into<String>) -> Self {
        let id = chunk_id.into();
        if !self.source_chunk_ids.contains(&id) {
            self.source_chunk_ids.push(id);
        }
        self
    }

    /// Set the source document ID.
    pub fn with_source_document_id(mut self, document_id: impl Into<String>) -> Self {
        self.source_document_id = Some(document_id.into());
        self
    }

    /// Set the source file path.
    pub fn with_source_file_path(mut self, file_path: impl Into<String>) -> Self {
        self.source_file_path = Some(file_path.into());
        self
    }

    /// Attach multimodal display metadata (066). Does not change identity (`name`).
    pub fn with_mm_display(
        mut self,
        display_name: impl Into<String>,
        page_num: Option<u32>,
        figure_index: Option<u32>,
        asset_id: Option<String>,
        mm_subtype: Option<String>,
    ) -> Self {
        self.display_name = Some(display_name.into());
        self.page_num = page_num;
        self.figure_index = figure_index;
        self.asset_id = asset_id;
        self.mm_subtype = mm_subtype;
        self
    }

    /// Add source chunk ID (mutable reference version).
    pub fn add_source_chunk_id(&mut self, chunk_id: impl Into<String>) {
        let id = chunk_id.into();
        if !self.source_chunk_ids.contains(&id) {
            self.source_chunk_ids.push(id);
        }
    }
}

/// An extracted relationship between entities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedRelationship {
    /// Source entity name.
    pub source: String,

    /// Target entity name.
    pub target: String,

    /// Relationship type/description.
    pub relation_type: String,

    /// Relationship description.
    pub description: String,

    /// Weight/strength (0.0 to 1.0).
    pub weight: f32,

    /// Keywords associated with this relationship.
    pub keywords: Vec<String>,

    /// Relationship embedding (for similarity search).
    pub embedding: Option<Vec<f32>>,

    /// Source chunk IDs where this relationship was extracted (049: union on dedupe).
    ///
    /// Prefer this list. `source_chunk_id` is kept for serde back-compat / first-id mirror.
    #[serde(default)]
    pub source_chunk_ids: Vec<String>,

    /// Legacy singular source chunk ID (mirrored from `source_chunk_ids.first()`).
    #[serde(default)]
    pub source_chunk_id: Option<String>,

    /// Source document ID.
    #[serde(default)]
    pub source_document_id: Option<String>,

    /// Original file path of the source document.
    #[serde(default)]
    pub source_file_path: Option<String>,
}

impl ExtractedRelationship {
    /// Create a new extracted relationship.
    pub fn new(
        source: impl Into<String>,
        target: impl Into<String>,
        relation_type: impl Into<String>,
    ) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
            relation_type: relation_type.into(),
            description: String::new(),
            weight: 0.5,
            keywords: Vec::new(),
            embedding: None,
            source_chunk_ids: Vec::new(),
            source_chunk_id: None,
            source_document_id: None,
            source_file_path: None,
        }
    }

    /// Set the description.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Set the weight.
    pub fn with_weight(mut self, weight: f32) -> Self {
        self.weight = weight.clamp(0.0, 1.0);
        self
    }

    /// Add keywords.
    pub fn with_keywords(mut self, keywords: Vec<String>) -> Self {
        self.keywords = keywords;
        self
    }

    /// All provenance chunk ids (Vec ∪ legacy singular).
    pub fn all_source_chunk_ids(&self) -> Vec<String> {
        let mut ids = self.source_chunk_ids.clone();
        if let Some(ref id) = self.source_chunk_id {
            if !id.is_empty() && !ids.iter().any(|x| x == id) {
                ids.push(id.clone());
            }
        }
        ids
    }

    /// Append a source chunk id (entity-parity; mirrors singular for legacy readers).
    pub fn with_source_chunk_id(mut self, chunk_id: impl Into<String>) -> Self {
        self.add_source_chunk_id(chunk_id);
        self
    }

    /// Append a source chunk id if missing.
    pub fn add_source_chunk_id(&mut self, chunk_id: impl Into<String>) {
        let id = chunk_id.into();
        if id.is_empty() {
            return;
        }
        if !self.source_chunk_ids.iter().any(|x| x == &id) {
            self.source_chunk_ids.push(id.clone());
        }
        if self.source_chunk_id.is_none() {
            self.source_chunk_id = Some(id);
        } else {
            // Keep singular as first id for citation helpers that still read it.
            self.source_chunk_id = self.source_chunk_ids.first().cloned();
        }
    }

    /// Set the source document ID.
    pub fn with_source_document_id(mut self, document_id: impl Into<String>) -> Self {
        self.source_document_id = Some(document_id.into());
        self
    }

    /// Set the source file path.
    pub fn with_source_file_path(mut self, file_path: impl Into<String>) -> Self {
        self.source_file_path = Some(file_path.into());
        self
    }
}
