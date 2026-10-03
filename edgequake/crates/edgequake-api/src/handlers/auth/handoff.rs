//! SPA session handoff (SPEC-158 LAW-158-4): redeem the single-use `code` the OIDC callback put
//! in the redirect URL. The refresh token already travels in the HttpOnly `eq_refresh` cookie;
//! this endpoint only mints the short-lived access token (never persisted, never in a URL).

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::ApiError;
use crate::handlers::auth_types::UserInfo;
use crate::services::federation::handoff::redeem_handoff;
use crate::services::federation::scope_guard::revalidate_scope;
use crate::services::login_tokens::{mint_access_token, SessionScope};
use crate::state::{AppState, PostgresRuntime};

use axum::extract::FromRef;

use super::get_record_by_id;

#[derive(Debug, Deserialize, ToSchema)]
pub struct HandoffRequest {
    /// One-time code from the SSO redirect (`?code=…`).
    pub code: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct HandoffResponse {
    pub access_token: String,
    /// Always `Bearer`.
    pub token_type: String,
    pub expires_in: i64,
    pub user: UserInfo,
    pub tenant_id: String,
    pub workspace_id: Option<String>,
    /// Validated same-origin path to navigate to (never an absolute URL).
    pub redirect_after: Option<String>,
}

/// POST /api/v1/auth/handoff — redeem a single-use SSO handoff code.
#[utoipa::path(
    post,
    path = "/api/v1/auth/handoff",
    tag = "Authentication",
    request_body = HandoffRequest,
    responses(
        (status = 200, description = "Access token for the SSO session", body = HandoffResponse),
        (status = 401, description = "Unknown, expired or already redeemed code"),
        (status = 403, description = "Tenant suspended or membership revoked")
    )
)]
pub async fn redeem_sso_handoff(
    State(state): State<AppState>,
    Json(request): Json<HandoffRequest>,
) -> Result<Json<HandoffResponse>, ApiError> {
    let invalid = || ApiError::auth_unauthorized("handoff", "code_invalid", None);
    let record = redeem_handoff(&state.auth.federation, request.code.trim())
        .await?
        .ok_or_else(invalid)?;

    let session = state
        .auth
        .federation
        .get_session(record.family_id)
        .await?
        .filter(|s| s.revoked_at.is_none())
        .ok_or_else(invalid)?;

    let pg = PostgresRuntime::from_ref(&state);
    let user = get_record_by_id(
        &state.storage,
        Some(&pg),
        &state.security,
        state.operational_stores.identity.as_deref(),
        &record.user_id.to_string(),
    )
    .await?
    .filter(|u| u.is_active)
    .ok_or_else(invalid)?;

    let scope = SessionScope {
        tenant_id: session.tenant_id,
        workspace_id: session.workspace_id,
    };
    let role = revalidate_scope(
        state.workspace_service.as_ref(),
        record.user_id,
        &scope,
        &user.role,
    )
    .await?;
    let user_uuid =
        Uuid::parse_str(&user.user_id).map_err(|_| ApiError::Internal("invalid user id".into()))?;
    let (access_token, expires_in) = mint_access_token(&state.auth, user_uuid, role, &scope)?;
    crate::services::federation::access_jti::remember_access_token(
        state.auth.federation.as_ref(),
        session.family_id,
        &access_token,
    )
    .await?;

    Ok(Json(HandoffResponse {
        access_token,
        token_type: "Bearer".into(),
        expires_in,
        user: UserInfo::from(&user),
        tenant_id: scope.tenant_id.to_string(),
        workspace_id: scope.workspace_id.map(|w| w.to_string()),
        redirect_after: record.redirect_after,
    }))
}
