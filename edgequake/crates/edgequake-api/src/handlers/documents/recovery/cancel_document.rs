//! POST `/documents/{document_id}/cancel` — cancel in-flight work by document id.
//!
//! WHY: `/tasks/{track_id}/cancel` needs a `track_id`. Legacy / orphaned rows
//! (worker died, task purged, seeded fixtures) can sit on a "processing" status
//! with no track at all, leaving the user with no way out except Delete.
//! This is the document-scoped escape hatch (idempotent, tenant-scoped).

use axum::{
    extract::{Path, State},
    Json,
};
use tracing::warn;

use crate::document_metadata::{is_terminal_document_status, is_terminal_failure_status};
use crate::error::{ApiError, ApiResult};
use crate::handlers::documents_types::CancelDocumentResponse;
use crate::middleware::TenantContext;
use crate::state::AppState;

use super::super::storage_helpers::metadata_matches_tenant_context;

const CANCEL_MESSAGE: &str = "Cancelled by user";

/// Cancel a document's in-flight pipeline work, even without a live task.
#[utoipa::path(
    post,
    path = "/api/v1/documents/{document_id}/cancel",
    tag = "Documents",
    params(("document_id" = String, Path, description = "Document ID")),
    responses(
        (status = 200, description = "Document cancelled (idempotent)", body = CancelDocumentResponse),
        (status = 404, description = "Document not found"),
        (status = 409, description = "Document already finished and cannot be cancelled")
    )
)]
pub async fn cancel_document(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Path(document_id): Path<String>,
) -> ApiResult<Json<CancelDocumentResponse>> {
    let kv = std::sync::Arc::clone(&state.storage.kv_storage);

    let metadata = crate::services::load_staging_first_metadata(kv.as_ref(), &document_id)
        .await
        .map_err(ApiError::Internal)?
        .map(|(_, value)| value)
        .filter(|value| metadata_matches_tenant_context(value, &tenant_ctx))
        .ok_or_else(|| ApiError::NotFound(format!("Document not found: {document_id}")))?;

    let status = metadata
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let already_cancelled = status.eq_ignore_ascii_case("cancelled");
    if !already_cancelled
        && (is_terminal_document_status(status) || is_terminal_failure_status(status))
    {
        return Err(ApiError::Conflict(format!(
            "Cannot cancel document in status: {status}"
        )));
    }

    let track_id = metadata
        .get("track_id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    let mut task_cancelled = false;
    if !already_cancelled {
        if let Some(ref track) = track_id {
            // Task-aware path: signal the worker token, retract indexes, sync KV.
            match crate::handlers::cancel_task(
                State(state.clone()),
                tenant_ctx.clone(),
                Path(track.clone()),
            )
            .await
            {
                Ok(_) => task_cancelled = true,
                // Task + metadata both gone for this track: fall through to KV sync.
                Err(ApiError::NotFound(_)) => {}
                Err(e) => return Err(e),
            }
        }
        task_cancelled |= cancel_linked_pdf_tasks(&state, &tenant_ctx, &metadata).await;
    }

    // Always converge the document row (idempotent; covers the no-track orphan).
    crate::services::sync_doc_cancelled_by_document_id(
        kv,
        state.optional_pg_pool(),
        &document_id,
        CANCEL_MESSAGE,
    )
    .await
    .map_err(ApiError::Internal)?;

    Ok(Json(CancelDocumentResponse {
        document_id,
        status: "cancelled".to_string(),
        track_id,
        task_cancelled,
    }))
}

/// Best-effort cancel of Convert/Insert tasks chained to the document's `pdf_id`.
async fn cancel_linked_pdf_tasks(
    state: &AppState,
    tenant_ctx: &TenantContext,
    metadata: &serde_json::Value,
) -> bool {
    let pdf_uuid = metadata
        .get("pdf_id")
        .and_then(|v| v.as_str())
        .and_then(|s| uuid::Uuid::parse_str(s).ok());
    let (Some(pdf_uuid), Some(workspace_id)) = (pdf_uuid, tenant_ctx.workspace_id_uuid()) else {
        return false;
    };
    match crate::services::apply_cancel_pdf_pipeline_tasks(
        &state.tasks.storage,
        &state.tasks.cancellation_registry,
        pdf_uuid,
        workspace_id,
    )
    .await
    {
        Ok(results) => results.iter().any(|r| r.cancelled && r.task.is_some()),
        Err(e) => {
            warn!(error = %e, %pdf_uuid, "cancel_document: linked PDF task cancel failed");
            false
        }
    }
}
