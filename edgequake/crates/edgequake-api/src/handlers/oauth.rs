//! Thin Axum + OpenAPI adapters for the MCP OAuth authorization server.

use axum::{
    extract::{Form, Query, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::Value;

use crate::error::ApiError;
use crate::oauth;
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/.well-known/oauth-authorization-server",
    tag = "MCP",
    responses((status = 200, description = "OAuth 2.0 Authorization Server Metadata (RFC 8414)"))
)]
pub async fn oauth_authorization_server_metadata(
    state: State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    oauth::metadata::authorization_server_metadata(state, headers).await
}

#[utoipa::path(
    get,
    path = "/.well-known/openid-configuration",
    tag = "MCP",
    responses((status = 200, description = "OpenID Connect Discovery (alias of AS metadata)"))
)]
pub async fn openid_configuration(state: State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    oauth::metadata::openid_configuration(state, headers).await
}

#[utoipa::path(
    get,
    path = "/oauth/authorize",
    tag = "MCP",
    responses(
        (status = 200, description = "Consent HTML when session cookie present"),
        (status = 302, description = "Redirect to login or client redirect_uri")
    )
)]
pub async fn oauth_authorize_get(
    state: State<AppState>,
    headers: HeaderMap,
    query: Query<oauth::authorize::AuthorizeQuery>,
) -> Result<Response, ApiError> {
    oauth::authorize::authorize_get(state, headers, query).await
}

#[utoipa::path(
    post,
    path = "/oauth/authorize",
    tag = "MCP",
    responses((status = 302, description = "Redirect with authorization code or error"))
)]
pub async fn oauth_authorize_post(
    state: State<AppState>,
    headers: HeaderMap,
    form: Form<oauth::authorize::ConsentForm>,
) -> Result<Response, ApiError> {
    oauth::authorize::authorize_post(state, headers, form).await
}

#[utoipa::path(
    post,
    path = "/oauth/token",
    tag = "MCP",
    responses(
        (status = 200, description = "Access token (+ refresh token)"),
        (status = 400, description = "OAuth error (invalid_grant / invalid_request)")
    )
)]
pub async fn oauth_token_post(
    state: State<AppState>,
    headers: HeaderMap,
    form: Form<oauth::token::TokenForm>,
) -> Response {
    oauth::token::token_post(state, headers, form).await
}

#[utoipa::path(
    post,
    path = "/oauth/revoke",
    tag = "MCP",
    responses((status = 200, description = "Token revoked (RFC 7009; always 200)"))
)]
pub async fn oauth_revoke_post(
    state: State<AppState>,
    form: Form<oauth::revoke::RevokeForm>,
) -> Response {
    oauth::revoke::revoke_post(state, form).await
}

#[utoipa::path(
    post,
    path = "/oauth/register",
    tag = "MCP",
    responses((status = 201, description = "Dynamically registered OAuth client"))
)]
pub async fn oauth_register_post(
    state: State<AppState>,
    body: Json<oauth::register::RegisterRequest>,
) -> Result<
    (
        axum::http::StatusCode,
        Json<oauth::register::RegisterResponse>,
    ),
    ApiError,
> {
    oauth::register::register_post(state, body).await
}

/// Re-export for OpenAPI schema tooling (unused at runtime).
#[allow(dead_code)]
pub type OAuthMetadataDocument = Value;
