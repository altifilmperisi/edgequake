//! OAuth 2.0 Dynamic Client Registration (RFC 7591) — legacy clients.

use axum::{extract::State, http::StatusCode, Json};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::ApiError;
use crate::state::AppState;

use super::cimd::validate_redirect_uri;
use super::store;
use super::types::OAuthClientRegistration;

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct RegisterRequest {
    pub redirect_uris: Vec<String>,
    #[serde(default)]
    pub client_name: Option<String>,
    #[serde(default)]
    pub grant_types: Option<Vec<String>>,
    #[serde(default)]
    pub response_types: Option<Vec<String>>,
    #[serde(default)]
    pub token_endpoint_auth_method: Option<String>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct RegisterResponse {
    pub client_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_name: Option<String>,
    pub redirect_uris: Vec<String>,
    pub grant_types: Vec<String>,
    pub response_types: Vec<String>,
    pub token_endpoint_auth_method: String,
}

/// `POST /oauth/register`
pub async fn register_post(
    State(state): State<AppState>,
    Json(body): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<RegisterResponse>), ApiError> {
    if body.redirect_uris.is_empty() {
        return Err(ApiError::BadRequest("redirect_uris required".into()));
    }
    for uri in &body.redirect_uris {
        validate_redirect_uri(uri)?;
    }

    let client_id = format!("eq_oauth_{}", Uuid::new_v4().simple());
    let grant_types = body
        .grant_types
        .unwrap_or_else(|| vec!["authorization_code".into(), "refresh_token".into()]);
    let response_types = body.response_types.unwrap_or_else(|| vec!["code".into()]);
    let token_endpoint_auth_method = body
        .token_endpoint_auth_method
        .unwrap_or_else(|| "none".into());

    let registration = OAuthClientRegistration {
        client_id: client_id.clone(),
        client_name: body.client_name.clone(),
        redirect_uris: body.redirect_uris.clone(),
        grant_types: grant_types.clone(),
        response_types: response_types.clone(),
        token_endpoint_auth_method: Some(token_endpoint_auth_method.clone()),
        is_cimd: false,
        created_at: Utc::now(),
    };
    store::store_client(&state.storage, registration).await?;

    Ok((
        StatusCode::CREATED,
        Json(RegisterResponse {
            client_id,
            client_name: body.client_name,
            redirect_uris: body.redirect_uris,
            grant_types,
            response_types,
            token_endpoint_auth_method,
        }),
    ))
}
