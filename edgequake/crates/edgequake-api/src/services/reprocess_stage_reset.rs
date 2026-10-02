//! SPEC-048 / SPEC-054: reset document stage fields when reprocess is accepted.
//!
//! Admission stages (before worker):
//!   `cleaning` — graph cleanup in the HTTP handler (honest UX)
//!   `queued`   — waiting for a free worker after cleanup
//!
//! Do not move cleanup after enqueue (worker race). Status-first is enough.

use chrono::Utc;
use serde_json::{Map, Value};

use edgequake_storage::error::StorageError;
use edgequake_storage::traits::KVStorage;
use edgequake_tasks::ReprocessMode;

use crate::error::ApiError;
use crate::services::graph_cleanup_timeout::{
    graph_cleanup_timeout_user_message, is_source_discovery_timeout, GraphCleanupAction,
};
use crate::services::reprocess_admission::ReprocessSkipReason;
use crate::services::upsert_metadata_kv_with_index;

/// Stats from graph cascade cleanup (optional UX detail in stage_message).
#[derive(Debug, Clone, Copy, Default)]
pub struct CleanupAdmitStats {
    pub entities_removed: usize,
    pub relationships_removed: usize,
}

/// Overlay a post-admit failure onto the pre-admit snapshot (pure).
///
/// Keeps prior `status` / `track_id` so the failed attempt remains the audit
/// pointer. Only `error_message` is replaced when `overlay_error` is set.
pub fn apply_pre_admit_restore_overlay(previous: &mut Value, overlay_error: Option<&str>) {
    let Some(msg) = overlay_error.map(str::trim).filter(|m| !m.is_empty()) else {
        return;
    };
    if let Some(obj) = previous.as_object_mut() {
        obj.insert("error_message".to_string(), Value::String(msg.to_string()));
    }
}

/// Restore metadata written by [`apply_early_reprocess_admit`] (issue #385).
///
/// After early admit, the only legal exits are enqueue or this compensate path.
pub async fn restore_pre_admit_metadata(
    kv: &dyn KVStorage,
    metadata_key: &str,
    mut previous: Value,
    overlay_error: Option<&str>,
) -> Result<(), StorageError> {
    apply_pre_admit_restore_overlay(&mut previous, overlay_error);
    upsert_metadata_kv_with_index(kv, metadata_key, previous).await
}

/// Best-effort compensate: log if restore fails (do not abort the batch).
pub async fn restore_pre_admit_metadata_best_effort(
    kv: &dyn KVStorage,
    metadata_key: &str,
    document_id: &str,
    previous: Value,
    overlay_error: Option<&str>,
) {
    if let Err(e) = restore_pre_admit_metadata(kv, metadata_key, previous, overlay_error).await {
        tracing::error!(
            document_id = %document_id,
            error = %e,
            "Failed to roll back early reprocess admit; document may remain processing without a task"
        );
    }
}

/// Classify a failure after early admit for `skip_reasons` (SPEC-119 vs enqueue).
pub fn post_admit_skip_reason(err: &ApiError) -> ReprocessSkipReason {
    match err {
        ApiError::ServiceUnavailable { message, .. }
            if message.contains("Graph cleanup timed out") =>
        {
            ReprocessSkipReason::GraphCleanupFailed
        }
        other if is_source_discovery_timeout(&other.to_string()) => {
            ReprocessSkipReason::GraphCleanupFailed
        }
        _ => ReprocessSkipReason::EnqueueFailed,
    }
}

/// Product copy to overlay when retract fails closed on discovery timeout.
pub fn graph_cleanup_overlay_error(err: &ApiError) -> Option<String> {
    if post_admit_skip_reason(err) == ReprocessSkipReason::GraphCleanupFailed {
        Some(graph_cleanup_timeout_user_message(
            GraphCleanupAction::Reprocess,
        ))
    } else {
        None
    }
}

/// Write in-flight reprocess fields **before** expensive graph cleanup.
///
/// WHY: `cleanup_document_graph_data` can take 5–10s for large graphs. Polls
/// during that window must see `processing`/`cleaning` (not stale `completed`).
/// `provisional_track_id` is typically the batch `reprocess_*` id; the handler
/// overwrites it with the real task track_id after the Task is created.
pub fn apply_early_reprocess_admit(
    obj: &mut Map<String, Value>,
    provisional_track_id: &str,
    mode: ReprocessMode,
) {
    obj.insert(
        "track_id".to_string(),
        Value::String(provisional_track_id.to_string()),
    );
    obj.insert(
        "retry_at".to_string(),
        Value::String(Utc::now().to_rfc3339()),
    );
    apply_cleaning_stage(obj, mode);
}

/// Admission stage while prior graph data is being removed (sync-in-HTTP).
pub fn apply_cleaning_stage(obj: &mut Map<String, Value>, mode: ReprocessMode) {
    write_processing_stage(
        obj,
        "cleaning",
        "Removing prior knowledge graph data…",
        mode,
    );
    // Avoid "Queued + 2923 entities" lie while prior graph is wiped.
    clear_stale_count_fields(obj);
}

/// After graph cleanup succeeds: true worker-admission stage.
///
/// Full/Entities → `queued`. MergeOnly → `merging` (pipeline starts there).
pub fn apply_post_cleanup_admission(
    obj: &mut Map<String, Value>,
    mode: ReprocessMode,
    stats: Option<CleanupAdmitStats>,
) {
    let (stage, base_message) = match mode {
        ReprocessMode::Full | ReprocessMode::EntitiesOnly => {
            ("queued", "Waiting for a free worker…")
        }
        ReprocessMode::MergeOnly => ("merging", "Waiting for a free worker (merge-only)…"),
    };
    let message = match stats {
        Some(s) if s.entities_removed > 0 || s.relationships_removed > 0 => {
            format!(
                "{base_message} Removed {} entities, {} relationships.",
                s.entities_removed, s.relationships_removed
            )
        }
        _ => base_message.to_string(),
    };
    write_processing_stage(obj, stage, &message, mode);
}

/// Apply stage fields when the recovery task is created (worker-ready).
///
/// Full/Entities → `queued`. MergeOnly → `merging`.
pub fn apply_reprocess_stage_reset(obj: &mut Map<String, Value>, mode: ReprocessMode) {
    let (stage, message) = match mode {
        ReprocessMode::Full => ("queued", "Waiting for a free worker…"),
        ReprocessMode::EntitiesOnly => ("queued", "Waiting for a free worker…"),
        ReprocessMode::MergeOnly => ("merging", "Waiting for a free worker (merge-only)…"),
    };
    write_processing_stage(obj, stage, message, mode);
}

/// Force PDF re-index / convert restart: honest converting stage, no stale completion.
///
/// WHY: Partial patches that only flip `status`/`current_stage`/`stage_progress`
/// leave `Processed N chunks, extracted…` + entity counts painted under Prepare,
/// and Active Runs nests a PDF meter that 404s once the old task is purged.
pub fn apply_pdf_convert_restart_admit(obj: &mut Map<String, Value>, stage_message: Option<&str>) {
    let message = stage_message
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("Converting PDF to Markdown…");
    write_processing_stage(obj, "converting", message, ReprocessMode::Full);
    clear_stale_completion_fields(obj);
}

fn write_processing_stage(
    obj: &mut Map<String, Value>,
    stage: &str,
    message: &str,
    mode: ReprocessMode,
) {
    obj.insert(
        "status".to_string(),
        Value::String("processing".to_string()),
    );
    obj.insert(
        "current_stage".to_string(),
        Value::String(stage.to_string()),
    );
    obj.insert(
        "stage_message".to_string(),
        Value::String(message.to_string()),
    );
    obj.insert(
        "stage_progress".to_string(),
        Value::Number(serde_json::Number::from_f64(0.0).unwrap_or_else(|| 0.into())),
    );
    obj.insert(
        "reprocess_mode".to_string(),
        Value::String(mode.to_string()),
    );
    obj.insert(
        "updated_at".to_string(),
        Value::String(Utc::now().to_rfc3339()),
    );
    obj.remove("error_message");
    // SPEC-155: new run — wipe prior ledger so Prepare/Extract do not inherit
    // counters from the previous attempt.
    crate::services::clear_run_progress(obj);
    crate::services::clear_progress_counts(obj);
}

/// Zero/remove graph + cost leftovers that lie about a finished run.
fn clear_stale_count_fields(obj: &mut Map<String, Value>) {
    for key in [
        "entity_count",
        "entities_count",
        "relationship_count",
        "relationships_count",
        "total_cost",
        "cost_usd",
        "ingestion_cost",
    ] {
        if obj.contains_key(key) {
            obj.insert(key.to_string(), Value::Number(0.into()));
        }
    }
}

/// Full convert-restart cleanup: counts + completion timestamps/tokens.
fn clear_stale_completion_fields(obj: &mut Map<String, Value>) {
    clear_stale_count_fields(obj);
    for key in [
        "processed_at",
        "processing_duration_ms",
        "chunk_count",
        "input_tokens",
        "output_tokens",
        "total_tokens",
        "progress_counts",
    ] {
        obj.remove(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::graph_cleanup_timeout::{
        api_error_graph_cleanup_timeout, graph_cleanup_timeout_user_message, GraphCleanupAction,
    };
    use serde_json::json;

    #[test]
    fn reset_clears_stale_extracting_message() {
        let mut v = json!({
            "status": "completed",
            "current_stage": "completed",
            "stage_message": "Done with 9000 entities",
            "stage_progress": 1.0,
            "error_message": "old"
        });
        let obj = v.as_object_mut().unwrap();
        apply_reprocess_stage_reset(obj, ReprocessMode::EntitiesOnly);
        assert_eq!(
            obj.get("status").and_then(|x| x.as_str()),
            Some("processing")
        );
        assert_eq!(
            obj.get("current_stage").and_then(|x| x.as_str()),
            Some("queued")
        );
        assert_eq!(
            obj.get("stage_progress").and_then(|x| x.as_f64()),
            Some(0.0)
        );
        assert!(obj.get("error_message").is_none());
        assert_eq!(
            obj.get("reprocess_mode").and_then(|x| x.as_str()),
            Some("entities")
        );
    }

    #[test]
    fn merge_mode_starts_at_merging_after_task_reset() {
        let mut v = json!({});
        apply_reprocess_stage_reset(v.as_object_mut().unwrap(), ReprocessMode::MergeOnly);
        assert_eq!(
            v.get("current_stage").and_then(|x| x.as_str()),
            Some("merging")
        );
    }

    #[test]
    fn early_admit_sets_cleaning_before_cleanup_window() {
        let mut v = json!({
            "status": "completed",
            "current_stage": "completed",
            "track_id": "old-completed-track",
            "stage_progress": 1.0,
            "entity_count": 2923,
            "total_cost": 0.202,
        });
        apply_early_reprocess_admit(
            v.as_object_mut().unwrap(),
            "reprocess_20260716_batch",
            ReprocessMode::Full,
        );
        assert_eq!(v.get("status").and_then(|x| x.as_str()), Some("processing"));
        assert_eq!(
            v.get("current_stage").and_then(|x| x.as_str()),
            Some("cleaning")
        );
        assert!(
            v.get("stage_message")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .contains("Removing prior knowledge graph"),
            "cleaning message must be honest about graph cleanup"
        );
        assert_eq!(
            v.get("track_id").and_then(|x| x.as_str()),
            Some("reprocess_20260716_batch")
        );
        assert_eq!(
            v.get("entity_count").and_then(|x| x.as_u64()),
            Some(0),
            "stale entity_count must be cleared on early admit"
        );
        assert_eq!(v.get("total_cost").and_then(|x| x.as_u64()), Some(0));
        assert!(v.get("retry_at").and_then(|x| x.as_str()).is_some());
    }

    #[test]
    fn post_cleanup_transitions_to_queued_with_stats() {
        let mut v = json!({
            "status": "processing",
            "current_stage": "cleaning",
        });
        apply_post_cleanup_admission(
            v.as_object_mut().unwrap(),
            ReprocessMode::Full,
            Some(CleanupAdmitStats {
                entities_removed: 10,
                relationships_removed: 3,
            }),
        );
        assert_eq!(
            v.get("current_stage").and_then(|x| x.as_str()),
            Some("queued")
        );
        let msg = v
            .get("stage_message")
            .and_then(|x| x.as_str())
            .unwrap_or("");
        assert!(msg.contains("Waiting for a free worker"));
        assert!(msg.contains("10 entities"));
        assert!(msg.contains("3 relationships"));
    }

    #[test]
    fn post_cleanup_merge_mode_goes_to_merging() {
        let mut v = json!({});
        apply_post_cleanup_admission(v.as_object_mut().unwrap(), ReprocessMode::MergeOnly, None);
        assert_eq!(
            v.get("current_stage").and_then(|x| x.as_str()),
            Some("merging")
        );
    }

    #[test]
    fn pdf_convert_restart_clears_stale_completion_message_and_counts() {
        let mut v = json!({
            "status": "completed",
            "current_stage": "completed",
            "stage_message": "Processed 22 chunks, extracted 658 entities and 381 relationships",
            "stage_progress": 1.0,
            "processed_at": "2026-08-06T10:15:12Z",
            "entity_count": 658,
            "chunk_count": 22,
            "relationship_count": 381,
            "cost_usd": 0.04,
            "error_message": "old",
            "track_id": "insert-old",
        });
        apply_pdf_convert_restart_admit(
            v.as_object_mut().unwrap(),
            Some("Converting PDF to Markdown (0/9 pages)"),
        );
        assert_eq!(v.get("status").and_then(|x| x.as_str()), Some("processing"));
        assert_eq!(
            v.get("current_stage").and_then(|x| x.as_str()),
            Some("converting")
        );
        assert_eq!(
            v.get("stage_message").and_then(|x| x.as_str()),
            Some("Converting PDF to Markdown (0/9 pages)")
        );
        assert_eq!(v.get("stage_progress").and_then(|x| x.as_f64()), Some(0.0));
        assert_eq!(v.get("entity_count").and_then(|x| x.as_u64()), Some(0));
        assert_eq!(
            v.get("relationship_count").and_then(|x| x.as_u64()),
            Some(0)
        );
        assert_eq!(v.get("cost_usd").and_then(|x| x.as_u64()), Some(0));
        assert!(v.get("processed_at").is_none());
        assert!(v.get("chunk_count").is_none());
        assert!(
            v.get("error_message").is_none(),
            "error_message must be cleared by write_processing_stage"
        );
    }

    #[test]
    fn restore_overlay_keeps_failed_status_and_track_id() {
        let mut v = json!({
            "status": "failed",
            "track_id": "insert-old",
            "current_stage": "extracting",
            "error_message": "old persist error",
        });
        apply_pre_admit_restore_overlay(
            &mut v,
            Some(&graph_cleanup_timeout_user_message(
                GraphCleanupAction::Reprocess,
            )),
        );
        assert_eq!(v.get("status").and_then(|x| x.as_str()), Some("failed"));
        assert_eq!(
            v.get("track_id").and_then(|x| x.as_str()),
            Some("insert-old")
        );
        let msg = v
            .get("error_message")
            .and_then(|x| x.as_str())
            .unwrap_or("");
        assert!(
            msg.contains("Graph cleanup timed out"),
            "timeout copy must overlay old persist error: {msg}"
        );
        assert!(!msg.contains("old persist error"));
    }

    #[test]
    fn restore_overlay_none_leaves_previous_error() {
        let mut v = json!({
            "status": "failed",
            "track_id": "insert-old",
            "error_message": "old persist error",
        });
        apply_pre_admit_restore_overlay(&mut v, None);
        assert_eq!(
            v.get("error_message").and_then(|x| x.as_str()),
            Some("old persist error")
        );
    }

    #[test]
    fn post_admit_timeout_is_graph_cleanup_failed() {
        let err = api_error_graph_cleanup_timeout(GraphCleanupAction::Reprocess);
        assert_eq!(
            post_admit_skip_reason(&err),
            ReprocessSkipReason::GraphCleanupFailed
        );
        assert!(graph_cleanup_overlay_error(&err).is_some());
    }

    #[test]
    fn post_admit_other_error_is_enqueue_failed() {
        let err = ApiError::Internal("queue full".into());
        assert_eq!(
            post_admit_skip_reason(&err),
            ReprocessSkipReason::EnqueueFailed
        );
        assert!(graph_cleanup_overlay_error(&err).is_none());
    }
}
