//! GET /api/v1/parse/backends — capability discovery.

use axum::extract::State;
use axum::Json;

use edgequake_pdf::PdfParserBackend;

use super::service::ParseLimits;
use super::types::{ParseBackendInfo, ParseBackendsResponse, ParseLimitsInfo, ParseProviderInfo};
use crate::error::ApiResult;
use crate::provider_catalog::build_available_providers_response;
use crate::state::AppState;

/// List parse backends, reachable vision providers/models, and ceilings.
#[utoipa::path(
    get,
    path = "/api/v1/parse/backends",
    responses(
        (status = 200, description = "Available parse backends", body = ParseBackendsResponse)
    ),
    tag = "Parse"
)]
pub async fn list_parse_backends(
    State(state): State<AppState>,
) -> ApiResult<Json<ParseBackendsResponse>> {
    let active_llm = state.query.llm_provider.name();
    let active_embedding = state.query.embedding_provider.name();
    let catalog = build_available_providers_response(
        state.query.models_config.as_ref(),
        active_llm,
        active_embedding,
    );

    let mut vision_providers = Vec::new();
    for p in &catalog.llm_providers {
        let mut models = Vec::new();
        if !p.default_models.chat_model.is_empty() {
            models.push(p.default_models.chat_model.clone());
        }
        if let Some(cfg) = state
            .query
            .models_config
            .providers
            .iter()
            .find(|c| c.name == p.id)
        {
            for m in &cfg.models {
                if !models.iter().any(|existing| existing == &m.name) {
                    models.push(m.name.clone());
                }
            }
        }
        vision_providers.push(ParseProviderInfo {
            name: p.id.clone(),
            available: p.available,
            models,
        });
    }

    let default_backend =
        edgequake_pdf::resolve_pdf_parser_choice(None, None, None, PdfParserBackend::from_env())
            .runtime_backend
            .as_str()
            .to_string();

    let limits = ParseLimits::from_env();
    let max_concurrency = state.parse_jobs.max_concurrent() as u32;

    Ok(Json(ParseBackendsResponse {
        backends: vec![
            ParseBackendInfo {
                name: "vision".into(),
                available: vision_providers.iter().any(|p| p.available),
                providers: vision_providers,
            },
            ParseBackendInfo {
                name: "edgeparse".into(),
                available: true,
                providers: vec![],
            },
            ParseBackendInfo {
                name: "edgeparse-ocr".into(),
                available: true,
                providers: vec![],
            },
        ],
        limits: ParseLimitsInfo {
            sync_max_pages: limits.sync_max_pages,
            sync_max_bytes: limits.sync_max_bytes,
            async_max_pages: limits.async_max_pages,
            async_max_bytes: limits.async_max_bytes,
            max_concurrency,
            dpi_min: 72,
            dpi_max: 400,
        },
        default_backend,
    }))
}
