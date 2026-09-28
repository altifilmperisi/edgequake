//! SPEC-151 — GET pages/health and POST pages/reprocess.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use edgequake_pdf::ReprocessStage;
use edgequake_storage::{
    DocumentPageState, PAGE_STAGE_FAILED, PAGE_STAGE_OK, PAGE_STAGE_PENDING, PAGE_STAGE_RUNNING,
    PAGE_STAGE_SKIPPED,
};
use edgequake_tasks::{Task, TaskType};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::middleware::TenantContext;
use crate::processor::page_reprocess::{
    page_scope_from_plan, plan_partial_reprocess, PartialReprocessPlan, PlanInput,
};
use crate::services::page_health_derive::{
    derive_page_health, PageHealth, PageHealthResponse, PageHealthSummary, StageHealth,
};
use crate::services::pending_doc_task_reconcile::has_active_task_for_document;
use crate::state::AppState;

/// Request body for POST pages/reprocess.
///
/// `pages` accepts either a JSON array of page numbers or a range string
/// (`"1-3,7"`). Clients may also send `page_numbers` as an alias array.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ReprocessPagesRequest {
    /// Page list (`[1,2,3]`) or range string (`"1-3,7"`).
    #[serde(default)]
    #[schema(value_type = Option<serde_json::Value>)]
    pub pages: Option<serde_json::Value>,
    /// Alias when clients send a plain array as `page_numbers`.
    #[serde(default)]
    pub page_numbers: Option<Vec<u32>>,
    pub stages: Vec<String>,
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ReprocessPagesResponse {
    pub dry_run: bool,
    pub plan: PartialReprocessPlan,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
}

fn parse_pages_field(
    pages: Option<serde_json::Value>,
    page_numbers: Option<Vec<u32>>,
) -> ApiResult<Vec<u32>> {
    if let Some(nums) = page_numbers {
        return Ok(nums);
    }
    let Some(v) = pages else {
        return Err(ApiError::ValidationError("pages required".into()));
    };
    if let Some(arr) = v.as_array() {
        let mut out = Vec::with_capacity(arr.len());
        for item in arr {
            let n = item
                .as_u64()
                .ok_or_else(|| ApiError::ValidationError("pages must be integers".into()))?
                as u32;
            out.push(n);
        }
        return Ok(out);
    }
    if let Some(s) = v.as_str() {
        return edgequake_pdf::parse_page_list(s)
            .map_err(|e| ApiError::ValidationError(e.to_string()));
    }
    Err(ApiError::ValidationError(
        "pages must be an array or range string".into(),
    ))
}

fn stored_to_health(
    document_id: &str,
    page_count: u32,
    rows: &[DocumentPageState],
) -> PageHealthResponse {
    let pages: Vec<PageHealth> = rows
        .iter()
        .map(|r| PageHealth {
            page_number: r.page_number as u32,
            parse: StageHealth {
                status: r.parse_status.clone(),
                error: r.parse_error.clone(),
                count: None,
                chunk_count: None,
                failed_chunk_count: None,
            },
            figures: StageHealth {
                status: r.figures_status.clone(),
                error: r.figures_error.clone(),
                count: Some(r.figures_count),
                chunk_count: None,
                failed_chunk_count: None,
            },
            entities: StageHealth {
                status: r.entities_status.clone(),
                error: r.entities_error.clone(),
                count: None,
                chunk_count: Some(r.chunk_count),
                failed_chunk_count: Some(r.failed_chunk_count),
            },
        })
        .collect();
    let summary = PageHealthSummary {
        parse_failed: pages
            .iter()
            .filter(|p| p.parse.status == PAGE_STAGE_FAILED)
            .count(),
        figures_failed: pages
            .iter()
            .filter(|p| p.figures.status == PAGE_STAGE_FAILED)
            .count(),
        entities_failed: pages
            .iter()
            .filter(|p| p.entities.status == PAGE_STAGE_FAILED)
            .count(),
    };
    let _ = (
        PAGE_STAGE_OK,
        PAGE_STAGE_PENDING,
        PAGE_STAGE_RUNNING,
        PAGE_STAGE_SKIPPED,
    );
    PageHealthResponse {
        document_id: document_id.to_string(),
        page_count: page_count.max(pages.len() as u32),
        pages,
        summary,
        source: "stored".into(),
    }
}

/// GET `/api/v1/documents/{document_id}/pages/health`
#[utoipa::path(
    get,
    path = "/api/v1/documents/{document_id}/pages/health",
    tag = "Documents",
    params(("document_id" = String, Path, description = "Document UUID")),
    responses(
        (status = 200, description = "Per-page parse/figures/entities health", body = PageHealthResponse),
        (status = 400, description = "Invalid document id or missing workspace")
    )
)]
pub async fn get_pages_health(
    State(state): State<AppState>,
    tenant: TenantContext,
    Path(document_id): Path<String>,
) -> ApiResult<Json<PageHealthResponse>> {
    let workspace_id = tenant
        .workspace_id
        .as_deref()
        .ok_or_else(|| ApiError::ValidationError("workspace required".into()))?;
    let ws = Uuid::parse_str(workspace_id)
        .map_err(|_| ApiError::ValidationError("invalid workspace".into()))?;
    let doc = Uuid::parse_str(&document_id)
        .map_err(|_| ApiError::ValidationError("document_id must be a UUID".into()))?;

    // Prefer stored page states.
    if let Some(store) = state.storage.page_state_storage.as_ref() {
        match store.list_page_states(doc, ws).await {
            Ok(rows) if !rows.is_empty() => {
                let page_count = rows.iter().map(|r| r.page_number).max().unwrap_or(0) as u32;
                return Ok(Json(stored_to_health(&document_id, page_count, &rows)));
            }
            Ok(_) => {}
            Err(e) => warn!(error = %e, "list_page_states failed; falling back to derive"),
        }
    }

    // Derive from markdown + KV metadata.
    let md_key = format!("{document_id}-content");
    let markdown = state
        .storage
        .kv_storage
        .get_by_id(&md_key)
        .await
        .ok()
        .flatten()
        .and_then(|v| {
            v.get("content")
                .and_then(|c| c.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_default();

    let meta_key = edgequake_storage::kv_keys::doc_metadata(&document_id);
    let page_count = state
        .storage
        .kv_storage
        .get_by_id(&meta_key)
        .await
        .ok()
        .flatten()
        .and_then(|v| {
            v.get("page_count")
                .and_then(|p| p.as_u64())
                .map(|n| n as u32)
        })
        .unwrap_or_else(|| {
            edgequake_pdf::page_numbers_from_markdown(&markdown)
                .into_iter()
                .max()
                .unwrap_or(1) as u32
        });

    Ok(Json(derive_page_health(
        &document_id,
        page_count,
        &markdown,
        &[],
        &[],
        &[],
    )))
}

/// POST `/api/v1/documents/{document_id}/pages/reprocess`
#[utoipa::path(
    post,
    path = "/api/v1/documents/{document_id}/pages/reprocess",
    tag = "Documents",
    params(("document_id" = String, Path, description = "Document UUID")),
    request_body = ReprocessPagesRequest,
    responses(
        (status = 200, description = "Dry-run plan", body = ReprocessPagesResponse),
        (status = 202, description = "Reprocess accepted", body = ReprocessPagesResponse),
        (status = 400, description = "Invalid pages or stages"),
        (status = 409, description = "Active task already running on document"),
        (status = 422, description = "Not a PDF / missing markers / no vision")
    )
)]
pub async fn reprocess_pages(
    State(state): State<AppState>,
    tenant: TenantContext,
    Path(document_id): Path<String>,
    Json(body): Json<ReprocessPagesRequest>,
) -> ApiResult<Response> {
    let workspace_id = tenant
        .workspace_id
        .as_deref()
        .ok_or_else(|| ApiError::ValidationError("workspace required".into()))?;
    let tenant_id = tenant
        .tenant_id
        .as_deref()
        .ok_or_else(|| ApiError::ValidationError("tenant required".into()))?;

    let pages = parse_pages_field(body.pages.clone(), body.page_numbers.clone())?;

    let stages: Result<Vec<ReprocessStage>, _> = body
        .stages
        .iter()
        .map(|s| ReprocessStage::parse(s))
        .collect();
    let stages = stages.map_err(ApiError::ValidationError)?;

    let meta_key = edgequake_storage::kv_keys::doc_metadata(&document_id);
    let metadata = state
        .storage
        .kv_storage
        .get_by_id(&meta_key)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("document {document_id}")))?;

    let source_type = metadata
        .get("source_type")
        .or_else(|| metadata.get("document_type"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if source_type != "pdf" {
        return Err(ApiError::ValidationError(
            "partial page reprocess requires a PDF document".into(),
        ));
    }

    let page_count = metadata
        .get("page_count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    if page_count == 0 {
        return Err(ApiError::ValidationError(
            "document has no page_count".into(),
        ));
    }

    // Check raw markdown presence for selected pages.
    let mut pages_missing_raw = false;
    if let (Ok(doc), Ok(ws)) = (Uuid::parse_str(&document_id), Uuid::parse_str(workspace_id)) {
        if let Some(store) = state.storage.page_state_storage.as_ref() {
            if let Ok(rows) = store.list_page_states(doc, ws).await {
                for p in &pages {
                    let has = rows.iter().any(|r| {
                        r.page_number as u32 == *p
                            && r.raw_markdown
                                .as_ref()
                                .map(|m| !m.trim().is_empty())
                                .unwrap_or(false)
                    });
                    if !has {
                        pages_missing_raw = true;
                        break;
                    }
                }
            } else {
                pages_missing_raw = true;
            }
        } else {
            pages_missing_raw = true;
        }
    }

    let has_snapshot = state
        .storage
        .kv_storage
        .get_by_id(&format!("{document_id}-extraction-snapshot"))
        .await
        .ok()
        .flatten()
        .is_some();

    // Rough chunk counts from metadata when available.
    let chunk_count = metadata
        .get("chunk_count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;
    let dirty_est = pages.len().max(1);
    let reusable_est = chunk_count.saturating_sub(dirty_est);

    let plan = plan_partial_reprocess(PlanInput {
        pages: pages.clone(),
        stages,
        page_count,
        pages_missing_raw,
        dirty_chunk_count: dirty_est,
        reusable_chunk_count: reusable_est,
        has_extraction_snapshot: has_snapshot,
    })
    .map_err(ApiError::ValidationError)?;

    if body.dry_run {
        return Ok(Json(ReprocessPagesResponse {
            dry_run: true,
            plan,
            track_id: None,
            task_id: None,
        })
        .into_response());
    }

    // Admission: one active task per document.
    let ws_uuid = Uuid::parse_str(workspace_id).ok();
    if has_active_task_for_document(state.tasks.storage.as_ref(), &document_id, ws_uuid)
        .await
        .unwrap_or(true)
    {
        return Err(ApiError::Conflict(
            "Another task is already running on this document".into(),
        ));
    }

    let needs_vision = plan
        .effective_stages
        .iter()
        .any(|s| s == "parse" || s == "figures");

    // SSOT with PDF recovery: metadata → env provider → provider-default model.
    // Leaving vision_model None caused `backend not configured: vision model`
    // after enqueue (retries exhausted) when document metadata omitted the field.
    let vision_provider = crate::services::resolve_pdf_recovery_vision_provider(&metadata);
    let vision_model =
        crate::services::resolve_pdf_recovery_vision_model(&metadata, &vision_provider);
    if needs_vision
        && vision_model
            .as_deref()
            .map(|m| m.trim().is_empty())
            .unwrap_or(true)
    {
        return Err(ApiError::ValidationError(
            "vision model is required for parse/figures page reprocess but could not be resolved \
             (set document vision_model, workspace vision LLM, or EDGEQUAKE_VISION_MODEL)"
                .into(),
        ));
    }

    let pdf_id = metadata
        .get("pdf_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ApiError::ValidationError("document has no pdf_id".into()))?;
    let pdf_uuid =
        Uuid::parse_str(pdf_id).map_err(|_| ApiError::ValidationError("invalid pdf_id".into()))?;

    let page_scope = page_scope_from_plan(&plan).map_err(ApiError::ValidationError)?;

    let pdf_task = edgequake_tasks::PdfProcessingData {
        pdf_id: pdf_uuid,
        tenant_id: Uuid::parse_str(tenant_id)
            .map_err(|_| ApiError::ValidationError("invalid tenant".into()))?,
        workspace_id: Uuid::parse_str(workspace_id)
            .map_err(|_| ApiError::ValidationError("invalid workspace".into()))?,
        enable_vision: needs_vision,
        vision_provider,
        vision_model,
        existing_document_id: Some(document_id.clone()),
        pdf_parser_backend: edgequake_pdf::PdfParserBackend::Vision,
        pdf_parser_backend_explicit: true,
        restart_from_scratch: false,
        reprocess_mode: Some(edgequake_tasks::ReprocessMode::EntitiesOnly),
        multimodal_process_options: None,
        vision_reasoning_effort: None,
        vision_extract: Default::default(),
        page_scope: Some(page_scope),
    };

    let task = Task::new(
        Uuid::parse_str(tenant_id).unwrap_or(Uuid::nil()),
        Uuid::parse_str(workspace_id).unwrap_or(Uuid::nil()),
        TaskType::PdfProcessing,
        serde_json::to_value(&pdf_task).map_err(|e| ApiError::Internal(e.to_string()))?,
    );
    let track_id = task.track_id.clone();
    let task_id = track_id.clone();

    state
        .enqueue_task(task)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    info!(
        document_id = %document_id,
        track_id = %track_id,
        pages = ?plan.pages,
        stages = ?plan.effective_stages,
        "SPEC-151: enqueued page reprocess"
    );

    let body = ReprocessPagesResponse {
        dry_run: false,
        plan,
        track_id: Some(track_id),
        task_id: Some(task_id),
    };
    Ok((StatusCode::ACCEPTED, Json(body)).into_response())
}
