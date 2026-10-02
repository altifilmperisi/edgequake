//! Tombstone facts for ingest admission (First Principles).
//!
//! Deletion is two-phase: an **irreversible** tombstone in the P0 authority,
//! then physical cleanup. If cleanup fails the document stays visible but can
//! never be ingested into again. The `status` projection (KV/SQL) can lag or
//! disagree with that authority, so every gate that admits ingest work must ask
//! the authority — not the projection — *before* spending PDF/LLM budget.
//!
//! Single owner of: "is this document tombstoned?" and "mark a half-deleted
//! document so it stops looking reprocessable".

use crate::middleware::TenantContext;
use crate::state::AppState;

/// Human sentence shown to the user; carries the typed `failure_class` marker
/// so the task classifier maps it to `document_deleted` (permanent, no retry).
pub fn document_deleted_message(document_id: &str) -> String {
    format!(
        "Document {document_id} was deleted but its cleanup did not finish — \
         it cannot be processed again. Finish the delete or re-upload the file \
         [failure_class=document_deleted]"
    )
}

/// Reason stored on the half-deleted document row.
pub const DELETE_INCOMPLETE_REASON: &str =
    "Delete did not finish — retry the delete, or re-upload the file as a new document.";

/// True when the P0 authority says `document_id` is tombstoned.
///
/// Availability over strictness on *read errors* (returns `false`, warns): the
/// persist gate (`allocate_ingest_generation`) still fails closed as backstop.
/// Memory-only / non-P0 harnesses (no reader, non-UUID scope) are never blocked.
pub async fn is_document_tombstoned(
    state: &AppState,
    tenant_ctx: &TenantContext,
    document_id: &str,
) -> bool {
    #[cfg(feature = "postgres")]
    {
        let Some(reader) = state.document_reader.as_ref() else {
            return false;
        };
        let Some(workspace_id) = tenant_ctx.workspace_id.as_deref() else {
            return false;
        };
        let Ok((scope, document)) = crate::services::ingestion_persist::resolve_authority_target(
            tenant_ctx.tenant_id.as_deref(),
            workspace_id,
            document_id,
        ) else {
            return false;
        };
        match reader.get_many(&scope, &[document]).await {
            Ok(views) => views
                .into_iter()
                .next()
                .flatten()
                .is_some_and(|view| view.deleted),
            Err(error) => {
                tracing::warn!(
                    document_id = %document_id,
                    error = %error,
                    "tombstone authority read failed — admitting (persist gate is the backstop)"
                );
                false
            }
        }
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (state, tenant_ctx, document_id);
        false
    }
}

/// Stamp the `delete_failed` lifecycle fields onto a metadata object.
///
/// Shared by `reset_deleting_status` (cascade failed) and
/// [`flag_delete_incomplete`] (tombstone found on a still-visible document).
pub(crate) fn apply_delete_failed_fields(
    obj: &mut serde_json::Map<String, serde_json::Value>,
    reason: &str,
) {
    use serde_json::json;
    obj.insert("status".to_string(), json!("delete_failed"));
    obj.insert("current_stage".to_string(), json!("delete_failed"));
    obj.insert("stage_message".to_string(), json!(reason));
    obj.insert("error_message".to_string(), json!(reason));
}

/// `delete_failed` fields plus the typed class: the authority tombstone exists,
/// so the only valid remedy is finishing the delete (or re-uploading).
fn apply_tombstoned_fields(obj: &mut serde_json::Map<String, serde_json::Value>, reason: &str) {
    use serde_json::json;
    apply_delete_failed_fields(obj, reason);
    obj.insert("failure_class".to_string(), json!("document_deleted"));
    obj.insert(
        "recommended_action".to_string(),
        json!("finish_delete_or_reupload"),
    );
}

/// Mark a tombstoned-but-visible document as `delete_failed` (KV + SQL).
///
/// Unconditional on the current status: once tombstoned nothing may be
/// processing, so a stale `failed`/`cancelled`/`processing` projection is wrong.
/// This closes the projection↔authority gap that let it be reprocessed.
pub async fn flag_delete_incomplete(state: &AppState, document_id: &str) {
    let metadata_key =
        crate::services::document_metadata_scan::metadata_key_for_document(document_id);
    if let Ok(Some(mut metadata)) = state.storage.kv_storage.get_by_id(&metadata_key).await {
        if let Some(obj) = metadata.as_object_mut() {
            apply_tombstoned_fields(obj, DELETE_INCOMPLETE_REASON);
            let _ = crate::services::upsert_metadata_kv_with_index(
                state.storage.kv_storage.as_ref(),
                &metadata_key,
                metadata,
            )
            .await;
        }
    }
    #[cfg(feature = "postgres")]
    let pool = state.optional_pg_pool();
    #[cfg(not(feature = "postgres"))]
    let pool = None;
    crate::services::touch_sql_delete_failed(pool, document_id).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_carries_typed_class_marker() {
        let msg = document_deleted_message("doc-1");
        assert!(msg.contains("doc-1"));
        assert_eq!(
            edgequake_tasks::classify_ingestion_failure(&msg),
            edgequake_tasks::IngestionFailureClass::DocumentDeleted
        );
        assert!(edgequake_tasks::is_permanent_ingestion_failure(&msg));
    }

    #[test]
    fn delete_failed_fields_overwrite_stale_status() {
        let mut obj = serde_json::json!({"status": "failed", "current_stage": "failed"})
            .as_object()
            .cloned()
            .unwrap();
        apply_delete_failed_fields(&mut obj, "x");
        assert_eq!(obj["status"], "delete_failed");
        assert_eq!(obj["current_stage"], "delete_failed");
        assert!(obj.get("failure_class").is_none());
        apply_tombstoned_fields(&mut obj, "y");
        assert_eq!(obj["failure_class"], "document_deleted");
    }

    #[tokio::test]
    async fn memory_state_is_never_tombstoned() {
        let state = AppState::test_state();
        let ctx = TenantContext {
            tenant_id: None,
            workspace_id: Some("ws".into()),
            user_id: None,
        };
        assert!(!is_document_tombstoned(&state, &ctx, "doc").await);
    }
}
