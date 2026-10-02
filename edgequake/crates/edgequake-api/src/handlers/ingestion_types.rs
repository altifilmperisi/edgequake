//! SPEC-048 DTOs for ingestion progress + pipeline activity.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Countable progress unit (pages, chunks, entities, relationships).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IngestionProgressCounts {
    pub current: u64,
    pub total: u64,
    /// Wire unit: `pages` | `chunks` | `entities` | `relationships`
    pub unit: String,
}

/// Per-stage item for FE ProgressDetail.stages compatibility.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IngestionStageProgressItem {
    pub stage: String,
    pub status: String,
    /// 0–100 for FE compatibility
    pub progress: f32,
    pub total_items: u64,
    pub completed_items: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Nested progress block expected by FE `TrackProgressResponse.progress`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IngestionProgressDetail {
    pub current_stage: String,
    /// 0–100
    pub completion_percentage: f32,
    pub latest_message: String,
    pub stages: Vec<IngestionStageProgressItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eta_seconds: Option<u64>,
}

/// SPEC-048 IngestionProgress + FE TrackProgressResponse aliases.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IngestionProgressResponse {
    pub track_id: String,
    pub document_id: String,
    pub filename: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_type: Option<String>,
    pub stage: String,
    pub stage_status: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub counts: Option<IngestionProgressCounts>,
    /// SPEC-155: typed monotonic per-phase progress ledger (SSOT for Active Runs).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_progress: Option<crate::services::RunProgress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress_01: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,
    pub updated_at: String,
    /// Alias of `filename` for FE
    pub document_name: String,
    /// Alias of `stage` for FE status field
    pub status: String,
    pub progress: IngestionProgressDetail,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PipelineActivityDoc {
    pub document_id: String,
    pub filename: String,
    pub stage: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PipelineActivityTask {
    pub id: String,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_id: Option<String>,
}

/// SPEC-048 PipelineActivity — Busy SSOT.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PipelineActivityResponse {
    /// `busy == (working.len() + tasks.len() > 0)`
    pub busy: bool,
    pub working: Vec<PipelineActivityDoc>,
    pub queued: Vec<PipelineActivityDoc>,
    pub tasks: Vec<PipelineActivityTask>,
    pub updated_at: String,
}

/// Batch progress request (FE getMultipleTrackProgress).
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct BatchIngestionProgressRequest {
    pub track_ids: Vec<String>,
}
