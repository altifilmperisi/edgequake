//! Thin ingestion status helpers (SPEC-057 P0).
//!
//! DRY cancel/status projection for HTTP cancel and PDF cancel writers.
//! Full stage_bridge collapse is deferred (P0.3b).

use chrono::Utc;
use edgequake_storage::PdfProcessingStatus;
use serde_json::{json, Map, Value};

use crate::services::classify_ingestion_failure;

/// Terminal kind for document metadata writers (cancel vs fail).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocTerminalKind {
    Cancelled,
    Failed,
}

impl DocTerminalKind {
    fn status_slug(self) -> &'static str {
        match self {
            Self::Cancelled => "cancelled",
            Self::Failed => "failed",
        }
    }
}

/// PDF terminal status for user/system cancel (never Failed).
pub fn pdf_status_for_cancel() -> PdfProcessingStatus {
    PdfProcessingStatus::Cancelled
}

fn is_pipeline_freeze_stage(stage: &str) -> bool {
    !matches!(
        stage.to_ascii_lowercase().as_str(),
        "cancelled" | "stopping" | "failed" | "completed" | ""
    )
}

/// Capture the last non-terminal pipeline stage before cancel overwrites it.
/// Idempotent: keeps an existing `cancelled_from_stage` on re-apply.
pub fn capture_cancelled_from_stage(metadata: &mut Map<String, Value>) {
    if metadata
        .get("cancelled_from_stage")
        .and_then(|v| v.as_str())
        .is_some_and(is_pipeline_freeze_stage)
    {
        return;
    }
    let prior = metadata
        .get("current_stage")
        .and_then(|v| v.as_str())
        .filter(|s| is_pipeline_freeze_stage(s))
        .map(|s| s.to_string());
    if let Some(stage) = prior {
        metadata.insert("cancelled_from_stage".to_string(), json!(stage));
    }
}

/// Single writer for terminal document metadata fields (cancel + fail).
///
/// Always clears `stage_progress` so Active Runs cannot show residual embedding 99%.
pub fn apply_doc_terminal_fields(
    metadata: &mut Map<String, Value>,
    kind: DocTerminalKind,
    message: &str,
) {
    let failure = classify_ingestion_failure(message);
    let workspace = metadata
        .get("workspace_id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    edgequake_observability::record_ingestion_failure(failure.as_str(), workspace);

    // INV-10: remember where cancel froze the run before rewriting stage.
    if kind == DocTerminalKind::Cancelled {
        capture_cancelled_from_stage(metadata);
    }

    let slug = kind.status_slug();
    metadata.insert("status".to_string(), json!(slug));
    metadata.insert("current_stage".to_string(), json!(slug));
    metadata.insert("stage_message".to_string(), json!(message));
    // Clear embedding-band progress so UI cannot show residual 99% after terminal.
    metadata.insert("stage_progress".to_string(), json!(0.0));
    crate::services::clear_run_progress(metadata);
    crate::services::clear_progress_counts(metadata);
    metadata.insert("error_message".to_string(), json!(message));
    metadata.insert("failure_class".to_string(), json!(failure.as_str()));
    metadata.insert(
        "recommended_action".to_string(),
        json!(failure.recommended_action()),
    );
    metadata.insert("updated_at".to_string(), json!(Utc::now().to_rfc3339()));
}

/// Apply cancelled document KV fields including `failure_class` (SPEC-045 / SPEC-057).
pub fn apply_doc_cancelled_fields(metadata: &mut Map<String, Value>, message: &str) {
    apply_doc_terminal_fields(metadata, DocTerminalKind::Cancelled, message);
}

/// Apply failed document KV fields (orphan / heartbeat fail paths).
pub fn apply_doc_failed_fields(metadata: &mut Map<String, Value>, message: &str) {
    apply_doc_terminal_fields(metadata, DocTerminalKind::Failed, message);
}

/// SSOT completion copy for ingest finalize + completed-orphan heal (DRY).
pub fn format_ingest_completion_stage_message(
    chunk_count: u64,
    entity_count: u64,
    relationship_count: u64,
) -> String {
    format!(
        "Processed {chunk_count} chunks, extracted {entity_count} entities and {relationship_count} relationships"
    )
}

/// Detect stale completion `stage_message` left under mid-pipeline status.
pub fn is_ingest_completion_stage_message(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    lower.contains("processed") && lower.contains("chunk") && lower.contains("extracted")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Map;

    #[test]
    fn apply_doc_cancelled_sets_failure_class() {
        let mut meta = Map::new();
        meta.insert("workspace_id".to_string(), json!("ws-1"));
        meta.insert("stage_progress".to_string(), json!(0.99));
        meta.insert("current_stage".to_string(), json!("embedding"));
        apply_doc_cancelled_fields(&mut meta, "Task cancelled by user");
        assert_eq!(
            meta.get("status").and_then(|v| v.as_str()),
            Some("cancelled")
        );
        assert_eq!(
            meta.get("current_stage").and_then(|v| v.as_str()),
            Some("cancelled")
        );
        assert_eq!(
            meta.get("stage_progress").and_then(|v| v.as_f64()),
            Some(0.0)
        );
        assert_eq!(
            meta.get("failure_class").and_then(|v| v.as_str()),
            Some("cancelled")
        );
        assert_eq!(
            meta.get("recommended_action").and_then(|v| v.as_str()),
            Some("none")
        );
    }

    #[test]
    fn apply_doc_failed_clears_stage_progress() {
        let mut meta = Map::new();
        meta.insert("workspace_id".to_string(), json!("ws-1"));
        meta.insert("stage_progress".to_string(), json!(0.99));
        meta.insert("current_stage".to_string(), json!("extracting"));
        apply_doc_failed_fields(&mut meta, "Pipeline interrupted — no active task");
        assert_eq!(meta.get("status").and_then(|v| v.as_str()), Some("failed"));
        assert_eq!(
            meta.get("current_stage").and_then(|v| v.as_str()),
            Some("failed")
        );
        assert_eq!(
            meta.get("stage_progress").and_then(|v| v.as_f64()),
            Some(0.0)
        );
    }

    #[test]
    fn apply_doc_cancelled_persists_cancelled_from_stage() {
        let mut meta = Map::new();
        meta.insert("workspace_id".to_string(), json!("ws-1"));
        meta.insert("current_stage".to_string(), json!("extracting"));
        apply_doc_cancelled_fields(&mut meta, "Task cancelled by user");
        assert_eq!(
            meta.get("current_stage").and_then(|v| v.as_str()),
            Some("cancelled")
        );
        assert_eq!(
            meta.get("cancelled_from_stage").and_then(|v| v.as_str()),
            Some("extracting")
        );

        // Idempotent re-apply must not wipe the freeze stage.
        apply_doc_cancelled_fields(&mut meta, "Task cancelled by user");
        assert_eq!(
            meta.get("cancelled_from_stage").and_then(|v| v.as_str()),
            Some("extracting")
        );
    }

    #[test]
    fn pdf_cancel_status_is_cancelled() {
        assert_eq!(pdf_status_for_cancel(), PdfProcessingStatus::Cancelled);
    }

    #[test]
    fn ingest_completion_stage_message_roundtrip() {
        let msg = format_ingest_completion_stage_message(22, 658, 381);
        assert_eq!(
            msg,
            "Processed 22 chunks, extracted 658 entities and 381 relationships"
        );
        assert!(is_ingest_completion_stage_message(&msg));
        assert!(!is_ingest_completion_stage_message(
            "Converting PDF to Markdown (0/9 pages)"
        ));
    }
}
