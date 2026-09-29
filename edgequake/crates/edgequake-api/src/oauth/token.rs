//! OAuth 2.1 token endpoint (authorization_code + refresh_token).

use axum::{
    extract::State,
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Form, Json,
};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use edgequake_auth::{Claims, Role};

use crate::error::ApiError;
use crate::mcp::config::McpPublicConfig;
use crate::state::AppState;

use super::pkce::verify_s256;
use super::store;
use super::types::{OAuthRefreshGrant, OAuthRefreshStatus, TakeRefreshOutcome};

/// MCP access tokens are short-lived; clients use refresh_token rotation.
pub const ACCESS_TTL_SECS: i64 = 900;
const REFRESH_TTL_SECS: i64 = 30 * 24 * 3600;

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct TokenForm {
    pub grant_type: String,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub redirect_uri: Option<String>,
    #[serde(default)]
    pub client_id: Option<String>,
    #[serde(default)]
    pub code_verifier: Option<String>,
    #[serde(default)]
    pub resource: Option<String>,
    #[serde(default)]
    pub refresh_token: Option<String>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: &'static str,
    pub expires_in: i64,
    pub scope: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
}

fn oauth_error(status: StatusCode, error: &str, description: &str) -> Response {
    let mut res = (
        status,
        Json(json!({
            "error": error,
            "error_description": description,
        })),
    )
        .into_response();
    res.headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    res.headers_mut()
        .insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
    res
}

/// `POST /oauth/token`
pub async fn token_post(
    State(state): State<AppState>,
    headers: HeaderMap,
    Form(form): Form<TokenForm>,
) -> Response {
    match form.grant_type.as_str() {
        "authorization_code" => match exchange_code(&state, &headers, &form).await {
            Ok(body) => token_ok(body),
            Err(resp) => resp,
        },
        "refresh_token" => match refresh_grant(&state, &headers, &form).await {
            Ok(body) => token_ok(body),
            Err(resp) => resp,
        },
        _ => oauth_error(
            StatusCode::BAD_REQUEST,
            "unsupported_grant_type",
            "supported: authorization_code, refresh_token",
        ),
    }
}

fn token_ok(body: TokenResponse) -> Response {
    let mut res = (StatusCode::OK, Json(body)).into_response();
    res.headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    res.headers_mut()
        .insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
    res
}

async fn exchange_code(
    state: &AppState,
    headers: &HeaderMap,
    form: &TokenForm,
) -> Result<TokenResponse, Response> {
    let code = form
        .code
        .as_deref()
        .ok_or_else(|| oauth_error(StatusCode::BAD_REQUEST, "invalid_request", "code required"))?;
    let redirect_uri = form.redirect_uri.as_deref().ok_or_else(|| {
        oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "redirect_uri required",
        )
    })?;
    let client_id = form.client_id.as_deref().ok_or_else(|| {
        oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "client_id required",
        )
    })?;
    let verifier = form.code_verifier.as_deref().ok_or_else(|| {
        oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "code_verifier required",
        )
    })?;

    let record = store::take_code(&state.storage, code)
        .await
        .map_err(api_err_to_oauth)?
        .ok_or_else(|| {
            oauth_error(
                StatusCode::BAD_REQUEST,
                "invalid_grant",
                "invalid or expired code",
            )
        })?;

    if record.expires_at < Utc::now() {
        return Err(oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_grant",
            "authorization code expired",
        ));
    }
    if record.client_id != client_id {
        return Err(oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_grant",
            "client_id mismatch",
        ));
    }
    if record.redirect_uri != redirect_uri {
        return Err(oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_grant",
            "redirect_uri mismatch",
        ));
    }
    if record.code_challenge_method != "S256" || !verify_s256(verifier, &record.code_challenge) {
        return Err(oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_grant",
            "PKCE verification failed",
        ));
    }

    let cfg = McpPublicConfig::resolve(headers);
    let resource = form.resource.as_deref().unwrap_or(&record.resource);
    if resource != record.resource || resource != cfg.resource_url {
        return Err(oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_grant",
            "resource mismatch",
        ));
    }

    issue_tokens(
        state,
        &cfg,
        client_id,
        resource,
        &record.scope,
        &record.user_id,
        &record.role,
        record.tenant_id.as_deref(),
        record.workspace_id.as_deref(),
        Uuid::new_v4(),
    )
    .await
    .map_err(api_err_to_oauth)
}

async fn refresh_grant(
    state: &AppState,
    headers: &HeaderMap,
    form: &TokenForm,
) -> Result<TokenResponse, Response> {
    let refresh = form.refresh_token.as_deref().ok_or_else(|| {
        oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "refresh_token required",
        )
    })?;
    let client_id = form.client_id.as_deref().ok_or_else(|| {
        oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "client_id required",
        )
    })?;

    let outcome = store::take_refresh(state, refresh)
        .await
        .map_err(api_err_to_oauth)?;

    let grant = match outcome {
        TakeRefreshOutcome::Consumed(g) => *g,
        TakeRefreshOutcome::Invalid => {
            return Err(oauth_error(
                StatusCode::BAD_REQUEST,
                "invalid_grant",
                "invalid refresh_token",
            ));
        }
        TakeRefreshOutcome::ReuseDetected { family_id } => {
            let _ = family_id;
            return Err(oauth_error(
                StatusCode::BAD_REQUEST,
                "invalid_grant",
                "refresh_token reuse detected; authorization revoked",
            ));
        }
    };

    if grant.expires_at < Utc::now() {
        let _ = store::revoke_refresh_family(state, grant.family_id).await;
        return Err(oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_grant",
            "refresh_token expired",
        ));
    }
    if grant.client_id != client_id {
        return Err(oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_grant",
            "client_id mismatch",
        ));
    }

    let cfg = McpPublicConfig::resolve(headers);
    let resource = form.resource.as_deref().unwrap_or(&grant.resource);
    if resource != grant.resource || resource != cfg.resource_url {
        return Err(oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_grant",
            "resource mismatch",
        ));
    }

    issue_tokens(
        state,
        &cfg,
        client_id,
        resource,
        &grant.scope,
        &grant.user_id,
        &grant.role,
        grant.tenant_id.as_deref(),
        grant.workspace_id.as_deref(),
        grant.family_id,
    )
    .await
    .map_err(api_err_to_oauth)
}

fn api_err_to_oauth(err: ApiError) -> Response {
    match err {
        ApiError::BadRequest(msg) => oauth_error(StatusCode::BAD_REQUEST, "invalid_request", &msg),
        other => oauth_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "server_error",
            &other.to_string(),
        ),
    }
}

#[allow(clippy::too_many_arguments)]
async fn issue_tokens(
    state: &AppState,
    cfg: &McpPublicConfig,
    client_id: &str,
    resource: &str,
    scope: &str,
    user_id: &str,
    role: &str,
    tenant_id: Option<&str>,
    workspace_id: Option<&str>,
    family_id: Uuid,
) -> Result<TokenResponse, ApiError> {
    let user_uuid = Uuid::parse_str(user_id)
        .map_err(|_| ApiError::Internal("invalid user_id in oauth grant".into()))?;
    let role = Role::try_parse(role).map_err(ApiError::Internal)?;

    let role_str = role.as_str().to_string();
    let mut claims = Claims::new(user_uuid, role, ACCESS_TTL_SECS)
        .with_audience(vec![resource.to_string()])
        .with_scope(scope.to_string())
        .with_issuer(cfg.authorization_server.trim_end_matches('/'));
    if let Some(t) = tenant_id {
        claims = claims.with_tenant_id(t);
    }
    if let Some(w) = workspace_id {
        claims = claims.with_workspace_id(w);
    }

    let access_token = state
        .auth
        .jwt
        .generate_token_with_claims(claims)
        .map_err(|e| ApiError::Internal(format!("token sign failed: {e}")))?;

    let refresh_token = format!("eqr_{}", Uuid::new_v4());
    store::store_refresh(
        state,
        OAuthRefreshGrant {
            token: refresh_token.clone(),
            family_id,
            client_id: client_id.to_string(),
            resource: resource.to_string(),
            scope: scope.to_string(),
            user_id: user_id.to_string(),
            role: role_str,
            tenant_id: tenant_id.map(|s| s.to_string()),
            workspace_id: workspace_id.map(|s| s.to_string()),
            expires_at: Utc::now() + Duration::seconds(REFRESH_TTL_SECS),
            status: OAuthRefreshStatus::Active,
        },
    )
    .await?;

    Ok(TokenResponse {
        access_token,
        token_type: "Bearer",
        expires_in: ACCESS_TTL_SECS,
        scope: scope.to_string(),
        refresh_token: Some(refresh_token),
    })
}
