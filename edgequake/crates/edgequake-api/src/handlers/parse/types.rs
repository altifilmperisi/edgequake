//! SPEC-094 request/response DTOs.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Canonical PDF parser backend wire values (OpenAPI / Swagger enum).
///
/// Runtime also accepts aliases (`llm`, `edge-parse`, `edgeparse_ocr`, …) via
/// [`edgequake_pdf::PdfParserBackend::from_env_str`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum PdfParserBackendName {
    #[serde(rename = "vision")]
    Vision,
    #[serde(rename = "edgeparse")]
    EdgeParse,
    /// EdgeParse with Tesseract raster-table OCR.
    #[serde(rename = "edgeparse-ocr")]
    EdgeParseOcr,
    /// SPEC-123 / SPEC-038 auto-routing (starts as Vision).
    #[serde(rename = "auto")]
    Auto,
}

/// Workspace/tenant update wire values including clear sentinel `none`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum PdfParserBackendUpdateName {
    #[serde(rename = "vision")]
    Vision,
    #[serde(rename = "edgeparse")]
    EdgeParse,
    #[serde(rename = "edgeparse-ocr")]
    EdgeParseOcr,
    #[serde(rename = "auto")]
    Auto,
    /// Clear override (inherit tenant / env / default).
    #[serde(rename = "none")]
    Clear,
}

/// Per-request parse options (multipart JSON `options` or query params).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct ParseOptions {
    /// PDF parser backend. Default: server `EDGEQUAKE_PDF_PARSER_BACKEND` or vision.
    ///
    /// - `vision` — VLM page OCR
    /// - `edgeparse` — CPU born-digital extraction (no Tesseract)
    /// - `edgeparse-ocr` — EdgeParse + Tesseract raster-table OCR
    /// - `auto` — Vision start with SPEC-038 EdgeParse fast-path when dense
    #[serde(default)]
    #[schema(value_type = Option<PdfParserBackendName>, example = "edgeparse-ocr")]
    pub backend: Option<String>,
    /// Vision provider id (e.g. `"ollama"`, `"openai"`).
    #[serde(default)]
    pub provider: Option<String>,
    /// Vision model id.
    #[serde(default)]
    pub model: Option<String>,
    /// Render DPI (72–400). Default 150.
    #[serde(default)]
    pub dpi: Option<u32>,
    /// Vision concurrency (1–16).
    #[serde(default)]
    pub concurrency: Option<usize>,
    /// Page selection string (`"1-10"`, `"5"`, `"1,3,5"`, `"all"`).
    #[serde(default)]
    pub pages: Option<String>,
    /// Table extraction method (forwarded to converter).
    #[serde(default)]
    pub table_method: Option<String>,
    /// When true, write page assets under a temp dir (cleaned after response).
    #[serde(default)]
    pub emit_assets: Option<bool>,
    /// When false, vision failures do not fall back to EdgeParse.
    #[serde(default)]
    pub allow_fallback: Option<bool>,
    /// Include per-page timing rows in the response.
    #[serde(default)]
    pub include_page_timings: Option<bool>,
    /// Force async job even under sync ceiling.
    #[serde(default, rename = "async")]
    pub force_async: Option<bool>,
}

impl ParseOptions {
    pub fn emit_assets(&self) -> bool {
        self.emit_assets.unwrap_or(false)
    }

    pub fn allow_fallback(&self) -> bool {
        self.allow_fallback.unwrap_or(true)
    }

    pub fn include_page_timings(&self) -> bool {
        self.include_page_timings.unwrap_or(false)
    }

    pub fn dpi_or_default(&self) -> u32 {
        self.dpi.unwrap_or(150).clamp(72, 400)
    }
}

/// Timing / cost metrics for a parse request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct ParseMetrics {
    pub total_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub render_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ocr_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assemble_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pages_per_second: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_cost_usd: Option<f64>,
}

/// Per-page timing row.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PageTiming {
    pub page: u32,
    pub ms: u64,
    pub chars: u64,
}

/// Successful parse response (sync or completed async job).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ParseResponse {
    pub markdown: String,
    pub backend: String,
    pub backend_effective: String,
    pub fallback_applied: bool,
    pub page_count: u32,
    pub metrics: ParseMetrics,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_timings: Option<Vec<PageTiming>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
    pub request_id: String,
}

/// 202 Accepted body for async parse jobs.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ParseAsyncAccepted {
    pub job_id: String,
    pub status: String,
    pub request_id: String,
}

/// Poll response for async jobs.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ParseJobStatusResponse {
    pub job_id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<ParseResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ParseJobErrorBody>,
    pub request_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ParseJobErrorBody {
    pub code: String,
    pub message: String,
}

/// Capability discovery for `/parse/backends`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ParseBackendsResponse {
    pub backends: Vec<ParseBackendInfo>,
    pub limits: ParseLimitsInfo,
    pub default_backend: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ParseBackendInfo {
    pub name: String,
    pub available: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub providers: Vec<ParseProviderInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ParseProviderInfo {
    pub name: String,
    pub available: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub models: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ParseLimitsInfo {
    pub sync_max_pages: u32,
    pub sync_max_bytes: u64,
    pub async_max_pages: u32,
    pub async_max_bytes: u64,
    pub max_concurrency: u32,
    pub dpi_min: u32,
    pub dpi_max: u32,
}
