//! SSO provider discovery for the login page + admin management (SPEC-158 LAW-158-7).

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Serialize;
use utoipa::ToSchema;

use crate::error::ApiError;
use crate::handlers::auth::ApiRequireAdmin;
use crate::services::federation::registry::ProviderSummary;
use crate::services::federation::store::StoredProvider;
use crate::state::AppState;

#[derive(Debug, Serialize, ToSchema)]
pub struct SsoProvidersResponse {
    pub providers: Vec<ProviderSummary>,
}

/// GET /api/v1/auth/sso/providers — public, secret-free list for the login page.
#[utoipa::path(
    get,
    path = "/api/v1/auth/sso/providers",
    tag = "Authentication",
    responses((status = 200, description = "Configured SSO providers", body = SsoProvidersResponse))
)]
pub async fn list_sso_providers(State(state): State<AppState>) -> Json<SsoProvidersResponse> {
    let providers = state
        .auth
        .all_providers()
        .iter()
        .filter(|p| p.config().is_runtime_builtin())
        .map(|p| ProviderSummary::of(p))
        .collect();
    Json(SsoProvidersResponse { providers })
}

/// GET /api/v1/admin/identity-providers — persisted provider definitions (admin).
#[utoipa::path(
    get,
    path = "/api/v1/admin/identity-providers",
    tag = "Authentication",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Stored identity providers", body = [StoredProvider]),
              (status = 403, description = "Admin role required"))
)]
pub async fn admin_list_identity_providers(
    State(state): State<AppState>,
    _admin: ApiRequireAdmin,
) -> Result<Json<Vec<StoredProvider>>, ApiError> {
    Ok(Json(state.auth.federation.list_providers().await?))
}

/// PUT /api/v1/admin/identity-providers/{slug} — upsert a provider and reload the registry.
#[utoipa::path(
    put,
    path = "/api/v1/admin/identity-providers/{slug}",
    tag = "Authentication",
    security(("bearer_auth" = [])),
    request_body = StoredProvider,
    params(("slug" = String, Path, description = "Provider slug")),
    responses((status = 200, description = "Saved", body = StoredProvider),
              (status = 400, description = "Invalid provider definition"),
              (status = 403, description = "Admin role required"))
)]
pub async fn admin_upsert_identity_provider(
    State(state): State<AppState>,
    _admin: ApiRequireAdmin,
    Path(slug): Path<String>,
    Json(mut provider): Json<StoredProvider>,
) -> Result<Json<StoredProvider>, ApiError> {
    provider.slug = slug.trim().to_ascii_lowercase();
    validate_provider(&provider)?;
    state.auth.federation.upsert_provider(&provider).await?;
    state.auth.reload_providers().await?;
    Ok(Json(provider))
}

/// DELETE /api/v1/admin/identity-providers/{slug}
#[utoipa::path(
    delete,
    path = "/api/v1/admin/identity-providers/{slug}",
    tag = "Authentication",
    security(("bearer_auth" = [])),
    params(("slug" = String, Path, description = "Provider slug")),
    responses((status = 204, description = "Deleted"), (status = 404, description = "Unknown provider"),
              (status = 403, description = "Admin role required"))
)]
pub async fn admin_delete_identity_provider(
    State(state): State<AppState>,
    _admin: ApiRequireAdmin,
    Path(slug): Path<String>,
) -> Result<StatusCode, ApiError> {
    if !state.auth.federation.delete_provider(&slug).await? {
        return Err(ApiError::NotFound(format!("identity provider {slug}")));
    }
    state.auth.reload_providers().await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Reject definitions that could never work or that weaken trust (LAW-158-3).
fn validate_provider(p: &StoredProvider) -> Result<(), ApiError> {
    let slug_ok = !p.slug.is_empty()
        && p.slug.len() <= 40
        && p.slug
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !slug_ok {
        return Err(ApiError::BadRequest(
            "slug must be 1-40 chars [a-z0-9_-]".into(),
        ));
    }
    if !p.issuer.starts_with("https://") && !p.issuer.starts_with("http://localhost") {
        return Err(ApiError::BadRequest(
            "issuer must be https (or http://localhost)".into(),
        ));
    }
    if p.client_id.trim().is_empty() || !p.redirect_uri.starts_with("http") {
        return Err(ApiError::BadRequest(
            "client_id and an http(s) redirect_uri are required".into(),
        ));
    }
    if !matches!(p.link_policy.as_str(), "never" | "verified_email") {
        return Err(ApiError::BadRequest(
            "link_policy must be never | verified_email".into(),
        ));
    }
    if p.max_role
        .parse::<edgequake_core::MembershipRole>()
        .is_err()
    {
        return Err(ApiError::BadRequest(
            "max_role must be readonly|member|admin|owner".into(),
        ));
    }
    Ok(())
}
