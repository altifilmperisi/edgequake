//! SPEC-048: ProgressFacade — assemble ingestion progress + pipeline activity (SRP).
//!
//! DRY: one place maps KV document metadata → progress DTOs used by HTTP handlers.

use chrono::Utc;
use serde_json::Value;

use crate::document_metadata::is_active_processing_status;
use crate::handlers::ingestion_types::{
    IngestionProgressCounts, IngestionProgressDetail, IngestionProgressResponse,
    IngestionStageProgressItem, PipelineActivityDoc, PipelineActivityResponse,
    PipelineActivityTask,
};
use edgequake_pipeline::ingestion_types::UnifiedStage;

/// Waiting / admission statuses (queued for a worker, not actively extracting).
pub fn is_queued_status(status: &str) -> bool {
    matches!(status.to_lowercase().as_str(), "pending" | "queued")
}

/// Normalize stage wire value (lowercase UnifiedStage or admission `queued`).
pub fn normalize_stage(raw: Option<&str>, status: Option<&str>) -> String {
    let candidate = raw.or(status).unwrap_or("uploading").trim().to_lowercase();
    match candidate.as_str() {
        "pending" => "queued".to_string(),
        "indexing" => "storing".to_string(),
        "processing" => "preprocessing".to_string(),
        other => other.to_string(),
    }
}

fn display_for_stage(stage: &str) -> String {
    match stage {
        "queued" => "Queued".to_string(),
        "uploading" => UnifiedStage::Uploading.display_name().to_string(),
        "converting" => UnifiedStage::Converting.display_name().to_string(),
        "preprocessing" => UnifiedStage::Preprocessing.display_name().to_string(),
        "chunking" => UnifiedStage::Chunking.display_name().to_string(),
        "extracting" => UnifiedStage::Extracting.display_name().to_string(),
        "gleaning" => UnifiedStage::Gleaning.display_name().to_string(),
        "merging" => UnifiedStage::Merging.display_name().to_string(),
        "summarizing" => UnifiedStage::Summarizing.display_name().to_string(),
        "embedding" => UnifiedStage::Embedding.display_name().to_string(),
        "storing" => UnifiedStage::Storing.display_name().to_string(),
        "completed" => UnifiedStage::Completed.display_name().to_string(),
        "failed" => UnifiedStage::Failed.display_name().to_string(),
        "cancelled" => "Cancelled".to_string(),
        other => other.to_string(),
    }
}

fn stage_status_for(stage: &str, status: &str) -> String {
    let s = status.to_lowercase();
    if s == "cancelled" || stage == "cancelled" {
        return "cancelled".to_string();
    }
    if s == "failed" || stage == "failed" {
        return "failed".to_string();
    }
    if s == "completed" || stage == "completed" {
        return "complete".to_string();
    }
    if is_queued_status(&s) || stage == "queued" {
        return "pending".to_string();
    }
    "active".to_string()
}

/// Ordered processing stages (+ completed) for timeline projection.
const TIMELINE_STAGES: &[&str] = &[
    "uploading",
    "converting",
    "preprocessing",
    "chunking",
    "extracting",
    "gleaning",
    "merging",
    "summarizing",
    "embedding",
    "storing",
    "completed",
];

fn stage_rank(stage: &str) -> Option<usize> {
    TIMELINE_STAGES.iter().position(|s| *s == stage)
}

fn should_skip_stage(stage: &str, source_type: Option<&str>, mode: Option<&str>) -> bool {
    let src = source_type.unwrap_or("").to_lowercase();
    let mode = mode.unwrap_or("full").to_lowercase();
    if stage == "converting" && src != "pdf" && !src.is_empty() {
        return true;
    }
    if mode == "merge" {
        if let Some(r) = stage_rank(stage) {
            if let Some(m) = stage_rank("merging") {
                return r < m;
            }
        }
    }
    if mode == "entities" && (stage == "uploading" || stage == "converting") {
        return true;
    }
    false
}

/// Project full per-step timeline from current stage (SPEC-048 detail progress).
fn build_timeline_stages(
    current: &str,
    status: &str,
    message: &str,
    progress_01: Option<f32>,
    counts: &Option<IngestionProgressCounts>,
    source_type: Option<&str>,
    mode: Option<&str>,
) -> Vec<IngestionStageProgressItem> {
    let current_norm = normalize_stage(Some(current), Some(status));
    let admission = current_norm == "queued" || is_queued_status(status);
    let cancelled = status.eq_ignore_ascii_case("cancelled") || current_norm == "cancelled";
    let failed = !cancelled && (status.eq_ignore_ascii_case("failed") || current_norm == "failed");
    let complete = status.eq_ignore_ascii_case("completed") || current_norm == "completed";
    let cur_rank = stage_rank(&current_norm);
    let completion_percentage = progress_01
        .map(|p| (p * 100.0).clamp(0.0, 100.0))
        .unwrap_or(0.0);

    TIMELINE_STAGES
        .iter()
        .map(|stage| {
            let skipped = should_skip_stage(stage, source_type, mode);
            if skipped {
                return IngestionStageProgressItem {
                    stage: (*stage).to_string(),
                    status: "skipped".to_string(),
                    progress: 100.0,
                    total_items: 0,
                    completed_items: 0,
                    message: Some("Skipped".to_string()),
                };
            }
            // Terminal cancel: no green complete priors, no Failed chip.
            if cancelled {
                return IngestionStageProgressItem {
                    stage: (*stage).to_string(),
                    status: "skipped".to_string(),
                    progress: 0.0,
                    total_items: 0,
                    completed_items: 0,
                    message: if *stage == "completed" {
                        Some(message.to_string())
                    } else {
                        None
                    },
                };
            }
            if complete {
                return IngestionStageProgressItem {
                    stage: (*stage).to_string(),
                    status: "complete".to_string(),
                    progress: 100.0,
                    total_items: 0,
                    completed_items: 0,
                    message: None,
                };
            }
            if admission {
                return IngestionStageProgressItem {
                    stage: (*stage).to_string(),
                    status: "pending".to_string(),
                    progress: 0.0,
                    total_items: 0,
                    completed_items: 0,
                    message: None,
                };
            }
            if failed {
                let is_fail_step =
                    *stage == current_norm || (*stage == "completed" && current_norm == "failed");
                if is_fail_step {
                    return IngestionStageProgressItem {
                        stage: (*stage).to_string(),
                        status: "failed".to_string(),
                        progress: completion_percentage,
                        total_items: counts.as_ref().map(|c| c.total).unwrap_or(0),
                        completed_items: counts.as_ref().map(|c| c.current).unwrap_or(0),
                        message: Some(message.to_string()),
                    };
                }
                if let (Some(sr), Some(cr)) = (stage_rank(stage), cur_rank) {
                    if sr < cr {
                        return IngestionStageProgressItem {
                            stage: (*stage).to_string(),
                            status: "complete".to_string(),
                            progress: 100.0,
                            total_items: 0,
                            completed_items: 0,
                            message: None,
                        };
                    }
                }
                return IngestionStageProgressItem {
                    stage: (*stage).to_string(),
                    status: "pending".to_string(),
                    progress: 0.0,
                    total_items: 0,
                    completed_items: 0,
                    message: None,
                };
            }
            if *stage == current_norm {
                return IngestionStageProgressItem {
                    stage: (*stage).to_string(),
                    status: "active".to_string(),
                    progress: completion_percentage,
                    total_items: counts.as_ref().map(|c| c.total).unwrap_or(0),
                    completed_items: counts.as_ref().map(|c| c.current).unwrap_or(0),
                    message: Some(message.to_string()),
                };
            }
            if let (Some(sr), Some(cr)) = (stage_rank(stage), cur_rank) {
                if sr < cr {
                    return IngestionStageProgressItem {
                        stage: (*stage).to_string(),
                        status: "complete".to_string(),
                        progress: 100.0,
                        total_items: 0,
                        completed_items: 0,
                        message: None,
                    };
                }
            }
            IngestionStageProgressItem {
                stage: (*stage).to_string(),
                status: "pending".to_string(),
                progress: 0.0,
                total_items: 0,
                completed_items: 0,
                message: None,
            }
        })
        .collect()
}

/// Parse `42/351` style counters from free-text stage_message (best-effort).
///
/// DRY: delegates to [`crate::services::parse_counts_from_message`] (SSOT module).
pub fn parse_counts_from_message(message: &str) -> Option<IngestionProgressCounts> {
    crate::services::parse_counts_from_message(message)
}

fn meta_str(obj: &serde_json::Map<String, Value>, key: &str) -> Option<String> {
    obj.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
}

fn meta_f32(obj: &serde_json::Map<String, Value>, key: &str) -> Option<f32> {
    obj.get(key).and_then(|v| {
        v.as_f64()
            .map(|f| f as f32)
            .or_else(|| v.as_i64().map(|i| i as f32))
    })
}

fn meta_f64(obj: &serde_json::Map<String, Value>, key: &str) -> Option<f64> {
    obj.get(key).and_then(|v| v.as_f64())
}

/// Build track progress from a single document metadata object.
pub fn progress_from_document_metadata(
    track_id: &str,
    obj: &serde_json::Map<String, Value>,
) -> IngestionProgressResponse {
    let document_id = meta_str(obj, "id").unwrap_or_default();
    let filename = meta_str(obj, "file_name")
        .or_else(|| meta_str(obj, "title"))
        .unwrap_or_else(|| document_id.clone());
    let status = meta_str(obj, "status").unwrap_or_else(|| "processing".to_string());
    let mut stage = normalize_stage(
        obj.get("current_stage").and_then(|v| v.as_str()),
        Some(&status),
    );
    // Terminal cancel wins over stale current_stage (embedding lag).
    if status.eq_ignore_ascii_case("cancelled") {
        stage = "cancelled".to_string();
    }
    let message = meta_str(obj, "stage_message")
        .filter(|m| !m.is_empty())
        .unwrap_or_else(|| display_for_stage(&stage));
    let progress_01 = meta_f32(obj, "stage_progress");
    // LAW-IS1: structured progress_counts is SSOT; message regex is fallback only.
    let counts = crate::services::resolve_progress_counts(obj, &message);
    let run_progress = crate::services::run_progress_from_metadata(obj);
    let mode = meta_str(obj, "reprocess_mode").or_else(|| meta_str(obj, "mode"));
    let cost_usd = meta_f64(obj, "cost_usd");
    let updated_at = meta_str(obj, "updated_at").unwrap_or_else(|| Utc::now().to_rfc3339());
    let started_at = meta_str(obj, "created_at");
    let source_type = meta_str(obj, "source_type");
    let stage_status = stage_status_for(&stage, &status);
    let completion_percentage = progress_01
        .map(|p| (p * 100.0).clamp(0.0, 100.0))
        .unwrap_or(0.0);

    let stages = build_timeline_stages(
        &stage,
        &status,
        &message,
        progress_01,
        &counts,
        source_type.as_deref(),
        mode.as_deref(),
    );

    IngestionProgressResponse {
        track_id: track_id.to_string(),
        document_id: document_id.clone(),
        filename: filename.clone(),
        source_type,
        stage: stage.clone(),
        stage_status,
        message: message.clone(),
        counts,
        run_progress,
        progress_01,
        mode,
        cost_usd,
        updated_at: updated_at.clone(),
        // FE TrackProgressResponse compatibility
        document_name: filename,
        status: stage.clone(),
        progress: IngestionProgressDetail {
            current_stage: stage,
            completion_percentage,
            latest_message: message,
            stages,
            eta_seconds: None,
        },
        started_at,
        completed_at: if status.eq_ignore_ascii_case("completed") {
            Some(updated_at)
        } else {
            None
        },
    }
}

/// Classify metadata into working vs queued activity docs.
pub fn classify_activity_doc(
    obj: &serde_json::Map<String, Value>,
) -> Option<(bool, PipelineActivityDoc)> {
    let status = meta_str(obj, "status")?;
    let stage = normalize_stage(
        obj.get("current_stage").and_then(|v| v.as_str()),
        Some(&status),
    );
    let document_id = meta_str(obj, "id")?;
    let filename = meta_str(obj, "file_name")
        .or_else(|| meta_str(obj, "title"))
        .unwrap_or_else(|| document_id.clone());

    let is_working = is_active_processing_status(&status) && !is_queued_status(&status);
    let is_queued = is_queued_status(&status);
    if !is_working && !is_queued {
        return None;
    }
    Some((
        is_working,
        PipelineActivityDoc {
            document_id,
            filename,
            stage,
            track_id: meta_str(obj, "track_id"),
            message: meta_str(obj, "stage_message"),
        },
    ))
}

/// Busy invariant: busy iff working docs or processing tasks exist.
pub fn assemble_pipeline_activity(
    docs: Vec<(bool, PipelineActivityDoc)>,
    tasks: Vec<PipelineActivityTask>,
) -> PipelineActivityResponse {
    let mut working = Vec::new();
    let mut queued = Vec::new();
    for (is_working, doc) in docs {
        if is_working {
            working.push(doc);
        } else {
            queued.push(doc);
        }
    }
    let busy = !working.is_empty() || !tasks.is_empty();
    PipelineActivityResponse {
        busy,
        working,
        queued,
        tasks,
        updated_at: Utc::now().to_rfc3339(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_chunk_counts() {
        let c = parse_counts_from_message("Extracting entities — chunk 42/351 (12%)").unwrap();
        assert_eq!(c.current, 42);
        assert_eq!(c.total, 351);
        assert_eq!(c.unit, "chunks");
    }

    #[test]
    fn parse_figure_vision_analyze_counts() {
        let c =
            parse_counts_from_message("Analyzing figures with Vision LLM — figure 3/12").unwrap();
        assert_eq!(c.current, 3);
        assert_eq!(c.total, 12);
        assert_eq!(c.unit, "figures");
    }

    #[test]
    fn busy_invariant_working_only() {
        let activity = assemble_pipeline_activity(
            vec![(
                true,
                PipelineActivityDoc {
                    document_id: "d1".into(),
                    filename: "a.pdf".into(),
                    stage: "extracting".into(),
                    track_id: Some("t1".into()),
                    message: None,
                },
            )],
            vec![],
        );
        assert!(activity.busy);
        assert_eq!(activity.working.len(), 1);
    }

    #[test]
    fn busy_false_when_only_queued() {
        let activity = assemble_pipeline_activity(
            vec![(
                false,
                PipelineActivityDoc {
                    document_id: "d1".into(),
                    filename: "a.pdf".into(),
                    stage: "queued".into(),
                    track_id: None,
                    message: None,
                },
            )],
            vec![],
        );
        assert!(!activity.busy);
        assert_eq!(activity.queued.len(), 1);
    }

    #[test]
    fn progress_from_metadata_extracting() {
        let obj = json!({
            "id": "doc-1",
            "file_name": "areal.pdf",
            "status": "processing",
            "current_stage": "extracting",
            "stage_message": "chunk 10/100",
            "stage_progress": 0.1,
            "source_type": "pdf",
            "updated_at": "2026-07-11T00:00:00Z"
        });
        let map = obj.as_object().unwrap();
        let p = progress_from_document_metadata("track-1", map);
        assert_eq!(p.stage, "extracting");
        assert_eq!(p.filename, "areal.pdf");
        assert!(p.counts.is_some());
        assert_eq!(p.progress.current_stage, "extracting");
        // Full timeline: prior complete, extracting active, later pending
        assert!(p.progress.stages.len() >= 10);
        let extracting = p
            .progress
            .stages
            .iter()
            .find(|s| s.stage == "extracting")
            .expect("extracting step");
        assert_eq!(extracting.status, "active");
        assert_eq!(extracting.completed_items, 10);
        assert_eq!(extracting.total_items, 100);
        let uploading = p
            .progress
            .stages
            .iter()
            .find(|s| s.stage == "uploading")
            .expect("uploading step");
        assert_eq!(uploading.status, "complete");
    }

    #[test]
    fn progress_counts_structured_beats_message_regex() {
        let obj = json!({
            "id": "doc-ssot",
            "file_name": "ticket.pdf",
            "status": "processing",
            "current_stage": "converting",
            "stage_message": "chunk 1/99",
            "stage_progress": 0.44,
            "source_type": "pdf",
            "progress_counts": { "unit": "pages", "current": 4, "total": 9 },
            "updated_at": "2026-07-31T00:00:00Z"
        });
        let p = progress_from_document_metadata("t-ssot", obj.as_object().unwrap());
        let counts = p.counts.expect("structured counts");
        assert_eq!(counts.unit, "pages");
        assert_eq!(counts.current, 4);
        assert_eq!(counts.total, 9);
    }

    #[test]
    fn timeline_skips_converting_for_markdown() {
        let obj = json!({
            "id": "doc-2",
            "file_name": "notes.md",
            "status": "processing",
            "current_stage": "chunking",
            "stage_message": "Chunking",
            "source_type": "markdown",
            "updated_at": "2026-07-11T00:00:00Z"
        });
        let p = progress_from_document_metadata("t2", obj.as_object().unwrap());
        let converting = p
            .progress
            .stages
            .iter()
            .find(|s| s.stage == "converting")
            .expect("converting");
        assert_eq!(converting.status, "skipped");
    }

    #[test]
    fn timeline_merge_mode_skips_early_stages() {
        let obj = json!({
            "id": "doc-3",
            "file_name": "a.pdf",
            "status": "processing",
            "current_stage": "merging",
            "stage_message": "10/100 entities",
            "stage_progress": 0.1,
            "source_type": "pdf",
            "reprocess_mode": "merge",
            "updated_at": "2026-07-11T00:00:00Z"
        });
        let p = progress_from_document_metadata("t3", obj.as_object().unwrap());
        let extracting = p
            .progress
            .stages
            .iter()
            .find(|s| s.stage == "extracting")
            .expect("extracting");
        assert_eq!(extracting.status, "skipped");
        let merging = p
            .progress
            .stages
            .iter()
            .find(|s| s.stage == "merging")
            .expect("merging");
        assert_eq!(merging.status, "active");
    }

    #[test]
    fn progress_cancelled_is_not_active_and_skips_timeline() {
        let obj = json!({
            "id": "doc-cancel",
            "file_name": "ticket.pdf",
            "status": "cancelled",
            "current_stage": "embedding",
            "stage_message": "Processing cancelled",
            "stage_progress": 0.99,
            "source_type": "pdf",
            "updated_at": "2026-07-30T00:00:00Z"
        });
        let p = progress_from_document_metadata("t-cancel", obj.as_object().unwrap());
        assert_eq!(p.stage_status, "cancelled");
        assert_ne!(p.stage_status, "active");
        assert!(
            p.progress
                .stages
                .iter()
                .all(|s| s.status == "skipped" || s.status == "pending"),
            "cancelled must not paint green complete priors: {:?}",
            p.progress.stages
        );
        assert!(
            p.progress.stages.iter().all(|s| s.status != "failed"),
            "cancelled must not show Failed chip"
        );
        assert!(
            p.progress.stages.iter().all(|s| s.status != "complete"),
            "cancelled must not show complete priors"
        );
    }

    #[test]
    fn progress_cancelled_stage_status_even_when_current_stage_lags() {
        let obj = json!({
            "id": "doc-lag",
            "file_name": "lag.pdf",
            "status": "cancelled",
            "current_stage": "cancelled",
            "stage_message": "Task cancelled by user",
            "stage_progress": 0.0,
            "source_type": "pdf",
            "updated_at": "2026-07-30T00:00:00Z"
        });
        let p = progress_from_document_metadata("t-lag", obj.as_object().unwrap());
        assert_eq!(p.stage_status, "cancelled");
        assert_eq!(p.stage, "cancelled");
    }
}
