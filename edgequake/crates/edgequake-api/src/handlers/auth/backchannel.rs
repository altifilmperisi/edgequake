//! OIDC Back-Channel Logout endpoint (SPEC-158 LAW-158-10). Logic: `services/federation/backchannel`.

use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Form, Json,
};
use serde::Deserialize;
use serde_json::json;

use crate::services::federation::backchannel::{process_logout_token, LogoutError};
use crate::state::AppState;

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct BackchannelLogoutForm {
    /// Signed `logout_token` JWT (`application/x-www-form-urlencoded`, OIDC BCL §2.5).
    pub logout_token: String,
}

/// POST /api/v1/auth/oidc/backchannel-logout
#[utoipa::path(
    post,
    path = "/api/v1/auth/oidc/backchannel-logout",
    tag = "Authentication",
    request_body(content = BackchannelLogoutForm, content_type = "application/x-www-form-urlencoded"),
    responses(
        (status = 200, description = "Logout token accepted; matching sessions revoked"),
        (status = 400, description = "Invalid, unverifiable or replayed logout token")
    )
)]
pub async fn oidc_backchannel_logout(
    State(state): State<AppState>,
    Form(form): Form<BackchannelLogoutForm>,
) -> Response {
    let no_store = [(header::CACHE_CONTROL, "no-store")];
    match process_logout_token(&state, &form.logout_token).await {
        Ok(outcome) => {
            tracing::info!(
                revoked = outcome.revoked_sessions,
                "back-channel logout accepted"
            );
            (StatusCode::OK, no_store).into_response()
        }
        Err(LogoutError::Invalid(code)) => {
            tracing::warn!(reason = code, "back-channel logout rejected");
            (
                StatusCode::BAD_REQUEST,
                no_store,
                Json(json!({ "error": "invalid_request", "error_description": code })),
            )
                .into_response()
        }
        Err(LogoutError::Internal(msg)) => {
            tracing::error!(error = %msg, "back-channel logout failed");
            (StatusCode::INTERNAL_SERVER_ERROR, no_store).into_response()
        }
    }
}
