//! SPEC-151 — Per-page parse / figures / entities state storage.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::Result;

/// Stage status vocabulary (DB CHECK + API).
pub const PAGE_STAGE_PENDING: &str = "pending";
pub const PAGE_STAGE_RUNNING: &str = "running";
pub const PAGE_STAGE_OK: &str = "ok";
pub const PAGE_STAGE_FAILED: &str = "failed";
pub const PAGE_STAGE_SKIPPED: &str = "skipped";

/// One durable page-state row.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DocumentPageState {
    pub page_state_id: Uuid,
    pub document_id: Uuid,
    pub workspace_id: Uuid,
    pub page_number: i32,
    pub parse_status: String,
    pub parse_error: Option<String>,
    pub parse_attempts: i32,
    pub parse_method: Option<String>,
    pub parse_model: Option<String>,
    pub raw_markdown: Option<String>,
    pub raw_sha256: Option<String>,
    pub figures_status: String,
    pub figures_count: i32,
    pub figures_error: Option<String>,
    pub entities_status: String,
    pub chunk_count: i32,
    pub failed_chunk_count: i32,
    pub entities_error: Option<String>,
    pub last_track_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Upsert parse-stage fields for one page.
#[derive(Debug, Clone)]
pub struct UpsertPageParse {
    pub page_number: i32,
    pub status: String,
    pub error: Option<String>,
    pub method: Option<String>,
    pub model: Option<String>,
    pub raw_markdown: Option<String>,
    pub raw_sha256: Option<String>,
    pub increment_attempts: bool,
    pub track_id: Option<String>,
}

/// Upsert figures-stage fields for one page.
#[derive(Debug, Clone)]
pub struct UpsertPageFigures {
    pub page_number: i32,
    pub status: String,
    pub count: i32,
    pub error: Option<String>,
    pub track_id: Option<String>,
}

/// Upsert entities-stage fields for one page.
#[derive(Debug, Clone)]
pub struct UpsertPageEntities {
    pub page_number: i32,
    pub status: String,
    pub chunk_count: i32,
    pub failed_chunk_count: i32,
    pub error: Option<String>,
    pub track_id: Option<String>,
}

/// Storage for `document_page_states` (SPEC-151).
#[async_trait]
pub trait PageStateStorage: Send + Sync {
    async fn list_page_states(
        &self,
        document_id: Uuid,
        workspace_id: Uuid,
    ) -> Result<Vec<DocumentPageState>>;

    async fn upsert_parse_batch(
        &self,
        document_id: Uuid,
        workspace_id: Uuid,
        rows: &[UpsertPageParse],
    ) -> Result<()>;

    async fn upsert_figures_batch(
        &self,
        document_id: Uuid,
        workspace_id: Uuid,
        rows: &[UpsertPageFigures],
    ) -> Result<()>;

    async fn upsert_entities_batch(
        &self,
        document_id: Uuid,
        workspace_id: Uuid,
        rows: &[UpsertPageEntities],
    ) -> Result<()>;

    async fn delete_for_document(&self, document_id: Uuid, workspace_id: Uuid) -> Result<()>;
}
