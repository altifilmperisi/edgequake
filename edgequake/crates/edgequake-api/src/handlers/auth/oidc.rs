//! OpenID Connect login handlers (SPEC-027 phase 54, SPEC-158 enterprise federation).
//!
//! `GET /auth/oidc/login`    — start (provider / org / redirect hints, durable pending state)
//! `GET /auth/oidc/callback` — verify, resolve user + tenant + membership, issue session
//!
//! Browser flows (`EDGEQUAKE_OIDC_SUCCESS_REDIRECT_URL` set) never carry tokens in a URL: the
//! callback sets the HttpOnly `eq_refresh` cookie and redirects with an opaque single-use `code`
//! the SPA redeems at `POST /auth/handoff` (LAW-158-4).

use axum::{
    extract::{FromRef, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
    Json,
};
use chrono::{Duration, Utc};
use serde::Deserialize;
use tracing::info;

use edgequake_audit::{AuditEventType, AuditResult};

use crate::error::ApiError;
use crate::handlers::auth_types::{LoginResponse, UserInfo};
use crate::services::federation::handoff::{create_handoff, NewHandoff};
use crate::services::federation::redirect::safe_redirect_path;
use crate::services::federation::resolver::{resolve_federated_login, FederatedLogin};
use crate::services::federation::store::{FederatedSession, LoginAttempt};
use crate::services::login_tokens::{issue_session, IssuedSession, SessionContext};
use crate::services::oidc_flow::{OidcFlowService, OidcPendingSession, OidcServiceError};
use crate::services::record_compliance_event_runtime;
use crate::state::{AppState, ComplianceRuntime, PostgresRuntime};

use super::refresh_cookie::{cookie_secure_from_headers, set_refresh_cookie_header};

const LOGIN_ATTEMPT_TTL_MINUTES: i64 = 10;

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct OidcLoginQuery {
    /// Provider slug (default: the configured default provider).
    pub provider: Option<String>,
    /// Organization alias hint (Keycloak `organization:{alias}`).
    pub org: Option<String>,
    /// Same-origin path to land on after login (validated, LAW-158-4).
    pub redirect: Option<String>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct OidcCallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    /// IdP-reported error (`access_denied`, …).
    pub error: Option<String>,
}

/// GET /api/v1/auth/oidc/login — redirect to IdP authorization endpoint (PKCE).
#[utoipa::path(
    get,
    path = "/api/v1/auth/oidc/login",
    tag = "Authentication",
    params(
        ("provider" = Option<String>, Query, description = "Provider slug"),
        ("org" = Option<String>, Query, description = "Organization alias hint"),
        ("redirect" = Option<String>, Query, description = "Same-origin path after login")
    ),
    responses(
        (status = 303, description = "Redirect to OIDC provider authorization URL"),
        (status = 503, description = "OIDC not enabled or provider unknown")
    )
)]
pub async fn oidc_login(
    State(state): State<AppState>,
    Query(query): Query<OidcLoginQuery>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let Some(service) = state.auth.provider_or_default(query.provider.as_deref()) else {
        let mapped = map_oidc_service_error(OidcServiceError::NotConfigured);
        if let Some(response) = spa_sso_unavailable(&headers, None, &mapped) {
            return Ok(response);
        }
        return Err(mapped);
    };
    let org = query
        .org
        .as_deref()
        .map(str::trim)
        .filter(|o| !o.is_empty());
    let start = match service.begin_login(org).await {
        Ok(start) => start,
        Err(err) => {
            let mapped = map_oidc_service_error(err);
            if let Some(response) = spa_sso_unavailable(
                &headers,
                service.config().success_redirect_url.as_deref(),
                &mapped,
            ) {
                return Ok(response);
            }
            return Err(mapped);
        }
    };

    state
        .auth
        .federation
        .put_login_attempt(&LoginAttempt {
            state: start.pending.csrf_token.clone(),
            provider_slug: service.slug().to_string(),
            pkce_verifier: start.pending.pkce_verifier.clone(),
            nonce: start.pending.nonce.clone(),
            organization_hint: org.map(|o| o.to_ascii_lowercase()),
            redirect_after: query
                .redirect
                .as_deref()
                .map(|r| safe_redirect_path(Some(r)))
                .filter(|r| r != "/"),
            expires_at: Utc::now() + Duration::minutes(LOGIN_ATTEMPT_TTL_MINUTES),
        })
        .await?;
    Ok(Redirect::to(&start.authorization_url).into_response())
}

/// GET /api/v1/auth/oidc/callback — complete OIDC flow and issue an EdgeQuake session.
#[utoipa::path(
    get,
    path = "/api/v1/auth/oidc/callback",
    tag = "Authentication",
    params(
        ("code" = Option<String>, Query, description = "Authorization code from IdP"),
        ("state" = Option<String>, Query, description = "CSRF state from login redirect"),
        ("error" = Option<String>, Query, description = "IdP error code")
    ),
    responses(
        (status = 200, description = "Login successful (JSON tokens; no success redirect configured)", body = LoginResponse),
        (status = 303, description = "Redirect to the SPA with an opaque single-use `code` (never tokens)"),
        (status = 401, description = "State mismatch, replayed or expired pending session"),
        (status = 403, description = "Tenant / provider policy denial"),
        (status = 409, description = "Email belongs to an unlinked local account"),
        (status = 503, description = "OIDC not enabled")
    )
)]
pub async fn oidc_callback(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OidcCallbackQuery>,
) -> Result<Response, ApiError> {
    if !state.auth.sso_active() {
        return Err(map_oidc_service_error(OidcServiceError::NotConfigured));
    }
    let state_param = query
        .state
        .as_deref()
        .ok_or_else(|| ApiError::auth_unauthorized("oidc_callback", "state_missing", None))?;
    // Atomic single-use take: replay / expiry / multi-replica safe (EC-158-24, EC-158-31).
    let attempt = state
        .auth
        .federation
        .take_login_attempt(state_param)
        .await?
        .ok_or_else(|| ApiError::auth_unauthorized("oidc_callback", "state_expired", None))?;
    let Some(provider) = state.auth.provider(&attempt.provider_slug) else {
        return Err(map_oidc_service_error(OidcServiceError::NotConfigured));
    };
    let success_url = provider.config().success_redirect_url.clone();

    let outcome = complete_callback(&state, &provider, &attempt, &query).await;
    match (outcome, success_url) {
        (Ok(done), Some(url)) => handoff_redirect(&state, &headers, &provider, &url, done).await,
        (Ok(done), None) => Ok(json_login(done)),
        (Err(error), Some(url)) => Ok(error_redirect(&url, &error)),
        (Err(error), None) => Err(error),
    }
}

struct CallbackDone {
    login: FederatedLogin,
    session: IssuedSession,
    redirect_after: Option<String>,
}

async fn complete_callback(
    state: &AppState,
    provider: &OidcFlowService,
    attempt: &LoginAttempt,
    query: &OidcCallbackQuery,
) -> Result<CallbackDone, ApiError> {
    if let Some(idp_error) = query.error.as_deref() {
        audit(state, "sso_login", AuditResult::Failure, None);
        return Err(ApiError::auth_unauthorized(
            "oidc_callback",
            "idp_error",
            Some(idp_error),
        ));
    }
    let code = query
        .code
        .as_deref()
        .ok_or_else(|| ApiError::auth_unauthorized("oidc_callback", "code_missing", None))?;
    let pending = OidcPendingSession {
        csrf_token: attempt.state.clone(),
        pkce_verifier: attempt.pkce_verifier.clone(),
        nonce: attempt.nonce.clone(),
    };

    let result = finish_login(state, provider, attempt, code, &pending).await;
    match &result {
        Ok(done) => audit(
            state,
            "sso_login",
            AuditResult::Success,
            Some(done.login.record.user_id.clone()),
        ),
        Err(_) => audit(state, "sso_login", AuditResult::Failure, None),
    }
    result
}

async fn finish_login(
    state: &AppState,
    provider: &OidcFlowService,
    attempt: &LoginAttempt,
    code: &str,
    pending: &OidcPendingSession,
) -> Result<CallbackDone, ApiError> {
    let identity = provider
        .complete_login(code, &attempt.state, pending)
        .await
        .map_err(map_oidc_service_error)?;
    let claims = identity.claims;

    let mut login = resolve_federated_login(
        state,
        provider,
        &claims,
        attempt.organization_hint.as_deref(),
    )
    .await?;

    let pg = PostgresRuntime::from_ref(state);
    crate::services::login_lockout::record_successful_login(
        &state.storage,
        Some(&pg),
        &state.security,
        state.operational_stores.identity.as_deref(),
        &mut login.record,
    )
    .await?;

    let ctx = SessionContext {
        auth: &state.auth,
        storage: &state.storage,
        pg: &pg,
        security: &state.security,
        stores: &state.operational_stores,
    };
    let session = issue_session(
        &ctx,
        &login.record.user_id,
        login.session_role.clone(),
        &login.tenant.scope(),
    )
    .await?;
    state
        .auth
        .federation
        .put_session(&FederatedSession {
            family_id: session.family_id,
            user_id: uuid::Uuid::parse_str(&login.record.user_id)
                .map_err(|_| ApiError::Internal("invalid user id".into()))?,
            provider_slug: provider.slug().to_string(),
            issuer: claims.issuer.clone(),
            subject: claims.subject.clone(),
            idp_sid: claims.session_id.clone(),
            tenant_id: login.tenant.tenant_id,
            workspace_id: login.tenant.workspace_id,
            created_at: Utc::now(),
            revoked_at: None,
        })
        .await?;
    crate::services::federation::access_jti::remember_access_token(
        state.auth.federation.as_ref(),
        session.family_id,
        &session.access_token,
    )
    .await?;

    info!(
        user_id = %login.record.user_id,
        tenant = %login.tenant.slug,
        provider = %provider.slug(),
        origin = ?login.origin,
        membership_role = ?login.membership_role,
        "SSO login successful"
    );
    Ok(CallbackDone {
        login,
        session,
        redirect_after: attempt.redirect_after.clone(),
    })
}

/// JSON mode (no SPA redirect configured): API / test clients receive the session directly.
fn json_login(done: CallbackDone) -> Response {
    let body = LoginResponse {
        access_token: done.session.access_token,
        token_type: "Bearer".to_string(),
        expires_in: done.session.expires_in,
        refresh_token: Some(done.session.refresh_token),
        user: UserInfo::from(&done.login.record),
    };
    (StatusCode::OK, Json(body)).into_response()
}

/// Browser mode: cookie + opaque code only — **no token ever appears in the URL**.
async fn handoff_redirect(
    state: &AppState,
    headers: &HeaderMap,
    provider: &OidcFlowService,
    success_url: &str,
    done: CallbackDone,
) -> Result<Response, ApiError> {
    let user_id = uuid::Uuid::parse_str(&done.login.record.user_id)
        .map_err(|_| ApiError::Internal("invalid user id".into()))?;
    let code = create_handoff(
        &state.auth.federation,
        NewHandoff {
            user_id,
            family_id: done.session.family_id,
            provider_slug: provider.slug(),
            scope: done.login.tenant.scope(),
            redirect_after: done.redirect_after,
        },
    )
    .await?;
    let mut url = url::Url::parse(success_url)
        .map_err(|e| ApiError::Internal(format!("invalid success redirect: {e}")))?;
    url.query_pairs_mut().append_pair("code", &code);

    let mut response = Redirect::to(url.as_str()).into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        set_refresh_cookie_header(
            &done.session.refresh_token,
            cookie_secure_from_headers(headers),
        ),
    );
    Ok(response)
}

/// Failure in browser mode: return to the SPA with a short machine code (no details leaked).
fn error_redirect(success_url: &str, error: &ApiError) -> Response {
    let Ok(mut url) = url::Url::parse(success_url) else {
        return StatusCode::BAD_GATEWAY.into_response();
    };
    url.query_pairs_mut()
        .append_pair("error", &error_code(error));
    Redirect::to(url.as_str()).into_response()
}

/// Same-origin Next proxy: a relative callback keeps the user on the UI host.
const RELATIVE_SSO_CALLBACK: &str = "/auth/callback";

fn prefers_html_navigation(headers: &HeaderMap) -> bool {
    let accept = headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if accept.contains("text/html") {
        return true;
    }
    headers.get("sec-fetch-mode").and_then(|v| v.to_str().ok()) == Some("navigate")
}

/// Browser SSO start must not dump a JSON 503; API clients still get 503.
fn spa_sso_unavailable(
    headers: &HeaderMap,
    success_url: Option<&str>,
    error: &ApiError,
) -> Option<Response> {
    if let Some(url) = success_url.filter(|u| !u.is_empty()) {
        return Some(error_redirect(url, error));
    }
    if prefers_html_navigation(headers) {
        return Some(
            Redirect::to(&format!(
                "{RELATIVE_SSO_CALLBACK}?error={}",
                error_code(error)
            ))
            .into_response(),
        );
    }
    None
}

/// Stable, non-sensitive machine code for an error (also drives the SPA message).
pub(crate) fn error_code(error: &ApiError) -> String {
    match error {
        ApiError::Forbidden(Some(reason)) => reason.clone(),
        ApiError::Conflict(code) => code.clone(),
        ApiError::AccountLocked => "account_locked".into(),
        ApiError::Unauthorized(_) => "unauthorized".into(),
        ApiError::ServiceUnavailable { .. } => "sso_unavailable".into(),
        _ => "login_failed".into(),
    }
}

fn audit(state: &AppState, action: &str, result: AuditResult, user_id: Option<String>) {
    record_compliance_event_runtime(
        &ComplianceRuntime::from_ref(state),
        "default",
        AuditEventType::Authentication,
        action,
        result,
        None,
        user_id,
        None,
    );
}

pub fn map_oidc_service_error(err: OidcServiceError) -> ApiError {
    match err {
        OidcServiceError::NotConfigured => ApiError::ServiceUnavailable {
            message: "OIDC is not enabled. Set EDGEQUAKE_OIDC_ENABLED=true and OIDC env vars."
                .to_string(),
            retry_after_secs: 0,
        },
        OidcServiceError::StateMismatch => {
            ApiError::auth_unauthorized("oidc_callback", "state_mismatch", None)
        }
        OidcServiceError::Unavailable(_) => ApiError::ServiceUnavailable {
            message: "The identity provider is unreachable; use password sign-in or retry."
                .to_string(),
            retry_after_secs: 5,
        },
        OidcServiceError::Rejected(_) => {
            ApiError::auth_unauthorized("oidc_callback", "idp_rejected", None)
        }
        OidcServiceError::Provider(msg) => ApiError::Internal(format!("oidc: {msg}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_codes_are_stable_and_non_sensitive() {
        assert_eq!(
            error_code(&ApiError::forbidden_reason("org_unknown")),
            "org_unknown"
        );
        assert_eq!(
            error_code(&ApiError::Conflict("account_exists_unlinked".into())),
            "account_exists_unlinked"
        );
        assert_eq!(
            error_code(&ApiError::Internal("secret db detail".into())),
            "login_failed"
        );
        assert_eq!(
            error_code(&ApiError::ServiceUnavailable {
                message: "The identity provider is unreachable; use password sign-in or retry."
                    .into(),
                retry_after_secs: 5,
            }),
            "sso_unavailable"
        );
    }

    #[test]
    fn html_accept_is_treated_as_browser_navigation() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::ACCEPT,
            "text/html,application/xhtml+xml".parse().unwrap(),
        );
        assert!(prefers_html_navigation(&headers));
        assert!(!prefers_html_navigation(&HeaderMap::new()));
    }
}
