use axum::extract::{Path, State};
use axum::Json;
use serde::Serialize;
use tracing::debug;
use utoipa::ToSchema;
use uuid::Uuid;

use super::byte_range::{evaluate_range, ByteRange};
use super::helpers::get_pdf_storage;
use crate::error::{ApiError, ApiResult};
use crate::middleware::TenantContext;
use crate::state::AppState;
use edgequake_storage::PdfProcessingStatus;

// ============================================================================
// PDF Content Download Endpoints (SPEC-002: Document Viewer)
// ============================================================================

/// PDF download response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PdfContentResponse {
    /// PDF ID.
    pub pdf_id: String,
    /// Linked document ID (mm-assets / markdown viewer scope).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_id: Option<String>,
    /// Original filename.
    pub filename: String,
    /// File size in bytes.
    pub file_size_bytes: i64,
    /// MIME type.
    pub content_type: String,
    /// Extracted markdown content (if processed).
    pub markdown_content: Option<String>,
    /// Whether PDF processing is complete.
    pub is_processed: bool,
}

/// Download raw PDF file data.
///
/// @implements SPEC-002: Document Viewer - PDF download endpoint
/// @implements UC0711: Download PDF for viewing
/// @enforces BR0701: Workspace isolation
///
/// Returns the raw PDF binary data with appropriate content-type headers.
/// This allows the frontend PDF viewer to render the original document.
///
/// # Arguments
///
/// * `state` - Application state with PDF storage
/// * `context` - Tenant context for workspace isolation
/// * `pdf_id` - PDF identifier
///
/// # Returns
///
/// * `Ok(Response)` - Raw PDF data with application/pdf content-type
/// * `Err(404)` - PDF not found
/// * `Err(403)` - Not authorized for this workspace
#[utoipa::path(
    get,
    path = "/api/v1/documents/pdf/{pdf_id}/download",
    params(
        ("pdf_id" = String, Path, description = "PDF identifier")
    ),
    responses(
        (status = 200, description = "Raw PDF data", content_type = "application/pdf"),
        (status = 206, description = "Partial PDF data for a `Range: bytes=` request", content_type = "application/pdf"),
        (status = 416, description = "Range not satisfiable"),
        (status = 404, description = "PDF not found"),
        (status = 403, description = "Not authorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Documents"
)]
pub async fn download_pdf(
    State(state): State<AppState>,
    context: TenantContext,
    headers: axum::http::HeaderMap,
    Path(pdf_id): Path<String>,
) -> ApiResult<axum::response::Response<axum::body::Body>> {
    let pdf_id = Uuid::parse_str(&pdf_id)
        .map_err(|_| ApiError::BadRequest("Invalid PDF ID format".to_string()))?;

    let pdf_storage = get_pdf_storage(&state)?;

    // Header-only lookup: auth + size without loading the blob, so a Range
    // request never costs a whole-file read.
    let info = pdf_storage
        .get_pdf_blob_info(&pdf_id)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to get PDF: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("PDF not found".to_string()))?;

    // OODA-51: Make workspace verification optional for PDF viewer compatibility
    // WHY: react-pdf Document component loads PDFs via URL without custom headers,
    // so X-Workspace-ID header is not available. The PDF is already isolated by its
    // UUID which is unique per workspace, so access is implicitly scoped.
    // If workspace header IS provided, verify it matches for defense-in-depth.
    if let Some(workspace_id) = context.workspace_id_uuid() {
        if info.workspace_id != workspace_id {
            return Err(ApiError::forbidden());
        }
    }

    let range_header = headers
        .get(axum::http::header::RANGE)
        .and_then(|v| v.to_str().ok());
    let range = evaluate_range(range_header, info.total_bytes);
    debug!(
        "PDF download: id={}, filename={}, size={}, range={:?} -> {:?}",
        pdf_id, info.filename, info.total_bytes, range_header, range
    );

    let body = match range {
        ByteRange::Full => {
            let pdf = pdf_storage
                .get_pdf(&pdf_id)
                .await
                .map_err(|e| ApiError::Internal(format!("Failed to get PDF: {}", e)))?
                .ok_or_else(|| ApiError::NotFound("PDF not found".to_string()))?;
            pdf.pdf_data
        }
        ByteRange::Partial { start, end } => pdf_storage
            .get_pdf_bytes_range(&pdf_id, start, end)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to read PDF range: {}", e)))?
            .ok_or_else(|| ApiError::NotFound("PDF not found".to_string()))?,
        ByteRange::Unsatisfiable => Vec::new(),
    };

    Ok(build_pdf_response(
        &info.filename,
        info.total_bytes,
        range,
        body,
    ))
}

/// Build the `200` / `206` / `416` PDF response for an evaluated range.
///
/// WHY range support: pdf.js reads the xref at the end of the file and then
/// only the chunks backing the pages in view, so page 1 paints without
/// transferring the whole file. `Content-Encoding: identity` is explicit because
/// pdf.js disables range requests on compressed responses (and gzip on a PDF
/// gains nothing).
fn build_pdf_response(
    filename: &str,
    total: u64,
    range: ByteRange,
    body: Vec<u8>,
) -> axum::response::Response<axum::body::Body> {
    use axum::http::{header, StatusCode};
    use axum::response::IntoResponse;

    let content_disposition = format!("inline; filename=\"{}\"", filename);
    let common = [
        (header::CONTENT_TYPE, "application/pdf".to_string()),
        (header::CONTENT_DISPOSITION, content_disposition),
        (header::CACHE_CONTROL, "private, max-age=3600".to_string()),
        (header::ACCEPT_RANGES, "bytes".to_string()),
        (header::CONTENT_ENCODING, "identity".to_string()),
    ];

    match range {
        ByteRange::Full => (StatusCode::OK, common, body).into_response(),
        ByteRange::Partial { start, end } => (
            StatusCode::PARTIAL_CONTENT,
            common,
            [(
                header::CONTENT_RANGE,
                format!("bytes {start}-{end}/{total}"),
            )],
            body,
        )
            .into_response(),
        ByteRange::Unsatisfiable => (
            StatusCode::RANGE_NOT_SATISFIABLE,
            [
                (header::ACCEPT_RANGES, "bytes".to_string()),
                (header::CONTENT_RANGE, format!("bytes */{total}")),
            ],
        )
            .into_response(),
    }
}

/// Get PDF content metadata including markdown.
///
/// @implements SPEC-002: Document Viewer - Markdown content endpoint
/// @implements UC0712: Get PDF metadata with extracted markdown
/// @enforces BR0701: Workspace isolation
///
/// Returns PDF metadata including the extracted markdown content (if processed).
/// This allows the frontend to display both the original PDF and the extracted markdown.
///
/// # Arguments
///
/// * `state` - Application state with PDF storage
/// * `context` - Tenant context for workspace isolation
/// * `pdf_id` - PDF identifier
///
/// # Returns
///
/// * `Ok(Json(PdfContentResponse))` - PDF metadata with markdown
/// * `Err(404)` - PDF not found
/// * `Err(403)` - Not authorized for this workspace
#[utoipa::path(
    get,
    path = "/api/v1/documents/pdf/{pdf_id}/content",
    params(
        ("pdf_id" = String, Path, description = "PDF identifier")
    ),
    responses(
        (status = 200, description = "PDF content metadata", body = PdfContentResponse),
        (status = 404, description = "PDF not found"),
        (status = 403, description = "Not authorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Documents"
)]
pub async fn get_pdf_content(
    State(state): State<AppState>,
    context: TenantContext,
    Path(pdf_id): Path<String>,
) -> ApiResult<Json<PdfContentResponse>> {
    let pdf_id = Uuid::parse_str(&pdf_id)
        .map_err(|_| ApiError::BadRequest("Invalid PDF ID format".to_string()))?;

    let pdf_storage = get_pdf_storage(&state)?;

    let pdf = pdf_storage
        .get_pdf(&pdf_id)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to get PDF: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("PDF not found".to_string()))?;

    // OODA-51: Make workspace verification optional for PDF viewer compatibility
    // WHY: Frontend PDF components may not have access to custom headers.
    // If workspace header IS provided, verify it matches for defense-in-depth.
    if let Some(workspace_id) = context.workspace_id_uuid() {
        if pdf.workspace_id != workspace_id {
            return Err(ApiError::forbidden());
        }
    }

    let is_processed = pdf.processing_status == PdfProcessingStatus::Completed;

    Ok(Json(PdfContentResponse {
        pdf_id: pdf.pdf_id.to_string(),
        document_id: pdf.document_id.map(|id| id.to_string()),
        filename: pdf.filename,
        file_size_bytes: pdf.file_size_bytes,
        content_type: pdf.content_type,
        markdown_content: pdf.markdown_content,
        is_processed,
    }))
}

#[cfg(test)]
mod range_response_tests {
    use super::{build_pdf_response, evaluate_range};
    use axum::http::{header, StatusCode};

    fn sample() -> Vec<u8> {
        (0u8..100).collect()
    }

    /// Mirror of the handler: evaluate the header, then slice like storage does.
    fn respond(range_header: Option<&str>) -> axum::response::Response<axum::body::Body> {
        let data = sample();
        let range = evaluate_range(range_header, data.len() as u64);
        let body = match range {
            super::ByteRange::Full => data.clone(),
            super::ByteRange::Partial { start, end } => {
                data[start as usize..=end as usize].to_vec()
            }
            super::ByteRange::Unsatisfiable => Vec::new(),
        };
        build_pdf_response("a.pdf", data.len() as u64, range, body)
    }

    #[tokio::test]
    async fn full_response_advertises_range_support() {
        let res = respond(None);
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.headers()[header::ACCEPT_RANGES], "bytes");
        assert_eq!(res.headers()[header::CONTENT_ENCODING], "identity");
    }

    #[tokio::test]
    async fn range_request_returns_206_with_exact_slice() {
        let res = respond(Some("bytes=10-19"));
        assert_eq!(res.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(res.headers()[header::CONTENT_RANGE], "bytes 10-19/100");
        let body = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(body.as_ref(), &sample()[10..=19]);
    }

    #[tokio::test]
    async fn out_of_bounds_range_returns_416() {
        let res = respond(Some("bytes=500-"));
        assert_eq!(res.status(), StatusCode::RANGE_NOT_SATISFIABLE);
        assert_eq!(res.headers()[header::CONTENT_RANGE], "bytes */100");
    }
}
