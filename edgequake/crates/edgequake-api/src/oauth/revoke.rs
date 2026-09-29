//! RFC 7009 Token Revocation for MCP OAuth AS.

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Form,
};
use serde::Deserialize;

use crate::state::AppState;

use super::store;

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct RevokeForm {
    pub token: String,
    #[serde(default)]
    pub token_type_hint: Option<String>,
    #[serde(default)]
    pub client_id: Option<String>,
}

/// `POST /oauth/revoke` — always 200 for unknown tokens (anti-enumeration).
pub async fn revoke_post(State(state): State<AppState>, Form(form): Form<RevokeForm>) -> Response {
    let _ = form.token_type_hint;
    let _ = form.client_id;

    // Refresh tokens are opaque `eqr_*`. Access JWTs are not denylisted here;
    // clients should discard them after revoke. Always return 200.
    if form.token.starts_with("eqr_") {
        let _ = store::revoke_refresh(&state, &form.token).await;
    } else {
        // Best-effort: treat as refresh hash lookup anyway (no error leak).
        let _ = store::revoke_refresh(&state, &form.token).await;
    }

    StatusCode::OK.into_response()
}
