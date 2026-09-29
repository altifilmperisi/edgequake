//! OAuth 2.1 authorization endpoint (PKCE + resource indicator).

use axum::{
    extract::{Form, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
};
use chrono::{Duration, Utc};
use serde::Deserialize;
use uuid::Uuid;

use edgequake_auth::Claims;

use crate::error::ApiError;
use crate::mcp::config::McpPublicConfig;
use crate::state::AppState;

use super::cimd::{redirect_uri_allowed, resolve_client};
use super::scopes::normalize_requested_scopes;
use super::store;
use super::types::OAuthAuthorizationCode;

const AUTH_COOKIE: &str = "edgequake_access_token";
const CODE_TTL_SECS: i64 = 600;

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct AuthorizeQuery {
    pub response_type: Option<String>,
    pub client_id: Option<String>,
    pub redirect_uri: Option<String>,
    pub scope: Option<String>,
    pub state: Option<String>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
    pub resource: Option<String>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct ConsentForm {
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: String,
    pub state: Option<String>,
    pub code_challenge: String,
    pub code_challenge_method: String,
    pub resource: String,
    pub approve: Option<String>,
}

/// `GET /oauth/authorize`
pub async fn authorize_get(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<AuthorizeQuery>,
) -> Result<Response, ApiError> {
    let validated = validate_authorize_request(&state, &headers, &q).await?;
    let Some(session) = session_from_cookie(&state, &headers)? else {
        let login = login_redirect(&headers, &q)?;
        return Ok(login.into_response());
    };

    Ok(consent_html(&validated, &session.sub).into_response())
}

/// `POST /oauth/authorize` — consent approval.
pub async fn authorize_post(
    State(state): State<AppState>,
    headers: HeaderMap,
    Form(form): Form<ConsentForm>,
) -> Result<Response, ApiError> {
    let cfg = McpPublicConfig::resolve(&headers);
    if form.resource != cfg.resource_url {
        return Err(ApiError::BadRequest("resource mismatch".into()));
    }
    if form.code_challenge_method != "S256" {
        return Err(ApiError::BadRequest(
            "code_challenge_method must be S256".into(),
        ));
    }

    let client = resolve_client(&state.storage, &form.client_id).await?;
    if !redirect_uri_allowed(&client, &form.redirect_uri) {
        return Err(ApiError::BadRequest("redirect_uri not registered".into()));
    }

    let Some(session) = session_from_cookie(&state, &headers)? else {
        return Err(ApiError::unauthorized());
    };

    if form.approve.as_deref() != Some("1") && form.approve.as_deref() != Some("true") {
        return Ok(error_redirect(
            &form.redirect_uri,
            "access_denied",
            form.state.as_deref(),
            &cfg.authorization_server,
        ));
    }

    let scope = normalize_requested_scopes(Some(&form.scope));
    let code = mint_code();
    let record = OAuthAuthorizationCode {
        code: code.clone(),
        client_id: form.client_id,
        redirect_uri: form.redirect_uri.clone(),
        code_challenge: form.code_challenge,
        code_challenge_method: form.code_challenge_method,
        resource: form.resource,
        scope,
        user_id: session.sub,
        role: session.role,
        tenant_id: session.tenant_id,
        workspace_id: session.workspace_id,
        expires_at: Utc::now() + Duration::seconds(CODE_TTL_SECS),
    };
    store::store_code(&state.storage, record).await?;

    let mut url = url::Url::parse(&form.redirect_uri)
        .map_err(|e| ApiError::Internal(format!("bad redirect_uri: {e}")))?;
    {
        let mut qp = url.query_pairs_mut();
        qp.append_pair("code", &code);
        if let Some(state) = form.state.as_deref() {
            qp.append_pair("state", state);
        }
        qp.append_pair("iss", cfg.authorization_server.trim_end_matches('/'));
    }
    Ok(Redirect::to(url.as_str()).into_response())
}

struct ValidatedAuthorize {
    client_id: String,
    redirect_uri: String,
    scope: String,
    state: Option<String>,
    code_challenge: String,
    code_challenge_method: String,
    resource: String,
    client_name: String,
}

async fn validate_authorize_request(
    state: &AppState,
    headers: &HeaderMap,
    q: &AuthorizeQuery,
) -> Result<ValidatedAuthorize, ApiError> {
    if q.response_type.as_deref() != Some("code") {
        return Err(ApiError::BadRequest("response_type must be code".into()));
    }
    let client_id = q
        .client_id
        .as_deref()
        .ok_or_else(|| ApiError::BadRequest("client_id required".into()))?;
    let redirect_uri = q
        .redirect_uri
        .as_deref()
        .ok_or_else(|| ApiError::BadRequest("redirect_uri required".into()))?;
    let code_challenge = q
        .code_challenge
        .as_deref()
        .ok_or_else(|| ApiError::BadRequest("code_challenge required".into()))?;
    let method = q.code_challenge_method.as_deref().unwrap_or("");
    if method != "S256" {
        return Err(ApiError::BadRequest(
            "code_challenge_method must be S256".into(),
        ));
    }
    let cfg = McpPublicConfig::resolve(headers);
    let resource = q
        .resource
        .as_deref()
        .ok_or_else(|| ApiError::BadRequest("resource required (RFC 8707)".into()))?;
    if resource != cfg.resource_url {
        return Err(ApiError::BadRequest(format!(
            "resource must be {}",
            cfg.resource_url
        )));
    }

    let client = resolve_client(&state.storage, client_id).await?;
    if !redirect_uri_allowed(&client, redirect_uri) {
        return Err(ApiError::BadRequest("redirect_uri not registered".into()));
    }

    Ok(ValidatedAuthorize {
        client_id: client_id.to_string(),
        redirect_uri: redirect_uri.to_string(),
        scope: normalize_requested_scopes(q.scope.as_deref()),
        state: q.state.clone(),
        code_challenge: code_challenge.to_string(),
        code_challenge_method: method.to_string(),
        resource: resource.to_string(),
        client_name: client
            .client_name
            .unwrap_or_else(|| client.client_id.clone()),
    })
}

fn session_from_cookie(state: &AppState, headers: &HeaderMap) -> Result<Option<Claims>, ApiError> {
    let Some(cookie_header) = headers.get(header::COOKIE).and_then(|v| v.to_str().ok()) else {
        return Ok(None);
    };
    let token = cookie_header.split(';').find_map(|part| {
        let part = part.trim();
        part.strip_prefix(&format!("{AUTH_COOKIE}=")).map(|v| {
            // Cookie may be URL-encoded by the WebUI.
            urlencoding::decode(v)
                .map(|c| c.into_owned())
                .unwrap_or_else(|_| v.to_string())
        })
    });
    let Some(token) = token.filter(|t| !t.is_empty()) else {
        return Ok(None);
    };
    match state.auth.jwt.verify_token(&token) {
        Ok(claims) => Ok(Some(claims)),
        Err(_) => Ok(None),
    }
}

fn login_redirect(headers: &HeaderMap, q: &AuthorizeQuery) -> Result<Redirect, ApiError> {
    let cfg = McpPublicConfig::resolve(headers);
    let mut authorize = url::Url::parse(&format!("{}/oauth/authorize", cfg.public_base_url()))
        .map_err(|e| ApiError::Internal(format!("authorize url: {e}")))?;
    {
        let mut qp = authorize.query_pairs_mut();
        if let Some(v) = &q.response_type {
            qp.append_pair("response_type", v);
        }
        if let Some(v) = &q.client_id {
            qp.append_pair("client_id", v);
        }
        if let Some(v) = &q.redirect_uri {
            qp.append_pair("redirect_uri", v);
        }
        if let Some(v) = &q.scope {
            qp.append_pair("scope", v);
        }
        if let Some(v) = &q.state {
            qp.append_pair("state", v);
        }
        if let Some(v) = &q.code_challenge {
            qp.append_pair("code_challenge", v);
        }
        if let Some(v) = &q.code_challenge_method {
            qp.append_pair("code_challenge_method", v);
        }
        if let Some(v) = &q.resource {
            qp.append_pair("resource", v);
        }
    }
    let path_and_query = format!(
        "{}{}",
        authorize.path(),
        authorize
            .query()
            .map(|q| format!("?{q}"))
            .unwrap_or_default()
    );
    let login = format!(
        "{}/login?redirect={}",
        cfg.public_base_url(),
        urlencoding::encode(&path_and_query)
    );
    Ok(Redirect::to(&login))
}

fn error_redirect(redirect_uri: &str, error: &str, state: Option<&str>, iss: &str) -> Response {
    let Ok(mut url) = url::Url::parse(redirect_uri) else {
        return (StatusCode::BAD_REQUEST, "invalid redirect_uri").into_response();
    };
    {
        let mut qp = url.query_pairs_mut();
        qp.append_pair("error", error);
        if let Some(state) = state {
            qp.append_pair("state", state);
        }
        qp.append_pair("iss", iss.trim_end_matches('/'));
    }
    Redirect::to(url.as_str()).into_response()
}

fn consent_html(v: &ValidatedAuthorize, user_sub: &str) -> Html<String> {
    let state_field = v
        .state
        .as_deref()
        .map(|s| {
            format!(
                r#"<input type="hidden" name="state" value="{}" />"#,
                html_escape(s)
            )
        })
        .unwrap_or_default();
    Html(format!(
        r#"<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8"/><title>Authorize EdgeQuake MCP</title>
<style>
body{{font-family:system-ui,sans-serif;max-width:420px;margin:3rem auto;padding:0 1rem;color:#111}}
card{{display:block;border:1px solid #ddd;border-radius:12px;padding:1.5rem}}
h1{{font-size:1.25rem;margin:0 0 .5rem}}
p{{color:#444;line-height:1.45}}
.scopes{{background:#f6f6f8;border-radius:8px;padding:.75rem;font-family:ui-monospace,monospace;font-size:.85rem}}
button{{margin-top:1rem;margin-right:.5rem;padding:.6rem 1rem;border-radius:8px;border:0;cursor:pointer}}
.approve{{background:#111;color:#fff}}
.deny{{background:#eee;color:#111}}
</style></head><body>
<form method="post" action="/oauth/authorize">
<div class="card">
<h1>Authorize MCP access</h1>
<p><strong>{client}</strong> wants to access EdgeQuake MCP as user <code>{user}</code>.</p>
<p class="scopes">{scope}</p>
<input type="hidden" name="client_id" value="{client_id}" />
<input type="hidden" name="redirect_uri" value="{redirect}" />
<input type="hidden" name="scope" value="{scope}" />
{state_field}
<input type="hidden" name="code_challenge" value="{challenge}" />
<input type="hidden" name="code_challenge_method" value="{method}" />
<input type="hidden" name="resource" value="{resource}" />
<button class="approve" type="submit" name="approve" value="1">Allow</button>
<button class="deny" type="submit" name="approve" value="0">Deny</button>
</div></form></body></html>"#,
        client = html_escape(&v.client_name),
        user = html_escape(user_sub),
        scope = html_escape(&v.scope),
        client_id = html_escape(&v.client_id),
        redirect = html_escape(&v.redirect_uri),
        challenge = html_escape(&v.code_challenge),
        method = html_escape(&v.code_challenge_method),
        resource = html_escape(&v.resource),
        state_field = state_field,
    ))
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn mint_code() -> String {
    format!("eqc_{}", Uuid::new_v4().simple())
}
