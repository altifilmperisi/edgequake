//! Authentication session handlers: login, refresh, logout, get_me.
//!
//! @implements FEAT0802 (JWT Token Support)
//! @implements FEAT0804 (JWT login with access and refresh tokens)
//! @implements FEAT0805 (Token refresh without re-authentication)

use axum::{
    extract::State,
    http::{
        header::{AUTHORIZATION, SET_COOKIE},
        HeaderMap, StatusCode,
    },
    response::{IntoResponse, Response},
    Json,
};
use chrono::{Duration, Utc};
use tracing::info;
use uuid::Uuid;

use edgequake_audit::{AuditEventType, AuditResult};
use edgequake_auth::Role;

use crate::error::ApiError;
use crate::handlers::auth::ApiAuthenticated;
use crate::services::record_compliance_event_runtime;
use crate::state::{
    ApiSecurityConfig, AuthRuntime, ComplianceRuntime, OperationalStores, PostgresRuntime,
    StorageRuntime,
};

use super::refresh_cookie::{
    clear_refresh_cookie_header, cookie_secure_from_headers, refresh_token_for_json_body,
    resolve_refresh_token, set_refresh_cookie_header,
};
use super::{
    find_user_by_login, get_record_by_id, get_user_by_id, RefreshTokenRecord, RequestAuthContext,
};
pub use crate::handlers::auth_types::{
    GetMeResponse, LoginRequest, LoginResponse, RefreshTokenRequest, RefreshTokenResponse, UserInfo,
};

/// Login endpoint.
///
/// POST /api/v1/auth/login
#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    tag = "Authentication",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = LoginResponse),
        (status = 401, description = "Invalid credentials"),
        (status = 423, description = "Account locked")
    )
)]
pub async fn login(
    State(auth): State<AuthRuntime>,
    State(storage): State<StorageRuntime>,
    State(pg_runtime): State<PostgresRuntime>,
    State(security): State<ApiSecurityConfig>,
    State(compliance): State<ComplianceRuntime>,
    State(stores): State<OperationalStores>,
    headers: HeaderMap,
    Json(request): Json<LoginRequest>,
) -> Result<Response, ApiError> {
    info!("Login attempt for user: {}", request.username);

    let user =
        find_user_by_login(&storage, Some(&pg_runtime), &security, &request.username).await?;

    let user = match user {
        Some(u) => u,
        None => {
            record_compliance_event_runtime(
                &compliance,
                "default",
                AuditEventType::Authentication,
                "login",
                AuditResult::Failure,
                None,
                None,
                None,
            );
            return Err(ApiError::auth_unauthorized(
                "login",
                "user_not_found",
                Some(&request.username),
            ));
        }
    };

    let mut record = get_record_by_id(
        &storage,
        Some(&pg_runtime),
        &security,
        stores.identity.as_deref(),
        &user.user_id,
    )
    .await?
    .ok_or_else(|| ApiError::Internal("User record missing after lookup".into()))?;

    crate::services::login_lockout::ensure_login_allowed(&record)?;

    if !record.is_active {
        return Err(ApiError::forbidden_reason("account_inactive"));
    }

    let password_valid = auth
        .password
        .verify_password(&request.password, &record.password_hash)
        .map_err(|e| ApiError::Internal(format!("password_verify failed: {e}")))?;

    if !password_valid {
        record_compliance_event_runtime(
            &compliance,
            "default",
            AuditEventType::Authentication,
            "login",
            AuditResult::Failure,
            None,
            Some(record.user_id.clone()),
            None,
        );
        crate::services::login_lockout::record_failed_login(
            &storage,
            Some(&pg_runtime),
            &security,
            stores.identity.as_deref(),
            &auth.config,
            &mut record,
        )
        .await?;
        return Err(ApiError::auth_unauthorized(
            "login",
            "invalid_password",
            Some(&request.username),
        ));
    }

    crate::services::login_lockout::record_successful_login(
        &storage,
        Some(&pg_runtime),
        &security,
        stores.identity.as_deref(),
        &mut record,
    )
    .await?;

    let user_uuid = Uuid::parse_str(&record.user_id)
        .map_err(|_| ApiError::Internal("Invalid user ID format".to_string()))?;

    let expiry_seconds = auth.jwt.expiry_duration().as_secs() as i64;
    let claims = crate::services::identity_storage::access_token_claims(
        user_uuid,
        Role::parse(&record.role),
        expiry_seconds,
    );
    let access_token = auth
        .jwt
        .generate_token_with_claims(claims)
        .map_err(|e| ApiError::Internal(format!("token_generation failed: {e}")))?;

    let refresh_token = Uuid::new_v4().to_string();
    let refresh_expiry = Utc::now() + Duration::days(30);

    let refresh_record = RefreshTokenRecord {
        token: refresh_token.clone(),
        user_id: record.user_id.clone(),
        family_id: Uuid::new_v4(),
        status: "active".to_string(),
        created_at: Utc::now(),
        expires_at: refresh_expiry,
        revoked: false,
    };

    crate::services::session_storage::persist_refresh_token(
        &storage,
        Some(&pg_runtime),
        &security,
        stores.sessions.as_deref(),
        &refresh_record,
    )
    .await?;

    info!("Login successful for user: {}", record.username);

    let expires_in = expiry_seconds;

    record_compliance_event_runtime(
        &compliance,
        "default",
        AuditEventType::Authentication,
        "login",
        AuditResult::Success,
        None,
        Some(record.user_id.clone()),
        None,
    );

    Ok(with_refresh_cookie(
        LoginResponse {
            access_token,
            token_type: "Bearer".to_string(),
            expires_in,
            refresh_token: refresh_token_for_json_body(&headers, &refresh_token),
            user: UserInfo::from(&record),
        },
        &refresh_token,
        cookie_secure_from_headers(&headers),
    ))
}

/// Refresh access token.
///
/// POST /api/v1/auth/refresh
#[utoipa::path(
    post,
    path = "/api/v1/auth/refresh",
    tag = "Authentication",
    request_body = RefreshTokenRequest,
    responses(
        (status = 200, description = "Token refreshed", body = RefreshTokenResponse),
        (status = 401, description = "Invalid or expired refresh token")
    )
)]
pub async fn refresh_token(
    State(auth): State<AuthRuntime>,
    State(storage): State<StorageRuntime>,
    State(pg_runtime): State<PostgresRuntime>,
    State(security): State<ApiSecurityConfig>,
    State(stores): State<OperationalStores>,
    headers: HeaderMap,
    Json(request): Json<RefreshTokenRequest>,
) -> Result<Response, ApiError> {
    use crate::services::auth_memory_store::TakeWebRefreshOutcome;

    let presented = resolve_refresh_token(request.refresh_token.as_deref(), &headers)
        .ok_or_else(|| ApiError::auth_unauthorized("refresh", "token_missing", None))?;

    let outcome = crate::services::session_storage::take_web_refresh(
        &storage,
        Some(&pg_runtime),
        &security,
        stores.sessions.as_deref(),
        &presented,
    )
    .await?;

    let record = match outcome {
        TakeWebRefreshOutcome::Consumed(r) => *r,
        TakeWebRefreshOutcome::Invalid => {
            return Err(ApiError::auth_unauthorized(
                "refresh",
                "token_invalid",
                None,
            ));
        }
        TakeWebRefreshOutcome::ReuseDetected { family_id: _ } => {
            return Err(ApiError::auth_unauthorized("refresh", "token_reuse", None));
        }
    };

    let user = get_user_by_id(
        &storage,
        Some(&pg_runtime),
        &security,
        stores.identity.as_deref(),
        &record.user_id,
    )
    .await?
    .ok_or(ApiError::auth_unauthorized(
        "refresh",
        "user_not_found",
        None,
    ))?;

    let user_uuid = Uuid::parse_str(&user.user_id)
        .map_err(|_| ApiError::Internal("Invalid user ID format".to_string()))?;

    // SPEC-154 Wave 4: rotate — issue successor in the same family.
    let new_refresh = Uuid::new_v4().to_string();
    let refresh_expiry = Utc::now() + Duration::days(30);
    let refresh_record = RefreshTokenRecord {
        token: new_refresh.clone(),
        user_id: record.user_id.clone(),
        family_id: record.family_id,
        status: "active".to_string(),
        created_at: Utc::now(),
        expires_at: refresh_expiry,
        revoked: false,
    };
    crate::services::session_storage::persist_refresh_token(
        &storage,
        Some(&pg_runtime),
        &security,
        stores.sessions.as_deref(),
        &refresh_record,
    )
    .await?;

    let expires_in = auth.jwt.expiry_duration().as_secs() as i64;
    let claims =
        crate::services::identity_storage::access_token_claims(user_uuid, user.role, expires_in);
    let access_token = auth
        .jwt
        .generate_token_with_claims(claims)
        .map_err(|e| ApiError::Internal(format!("Token generation error: {}", e)))?;

    Ok(with_refresh_cookie(
        RefreshTokenResponse {
            access_token,
            token_type: "Bearer".to_string(),
            expires_in,
            refresh_token: refresh_token_for_json_body(&headers, &new_refresh),
        },
        &new_refresh,
        cookie_secure_from_headers(&headers),
    ))
}

/// Logout endpoint (revoke refresh token).
///
/// POST /api/v1/auth/logout
#[utoipa::path(
    post,
    path = "/api/v1/auth/logout",
    tag = "Authentication",
    request_body = RefreshTokenRequest,
    responses(
        (status = 204, description = "Logout successful"),
        (status = 401, description = "Invalid token")
    )
)]
#[allow(clippy::too_many_arguments)] // Axum extractor fan-in
pub async fn logout(
    State(auth): State<AuthRuntime>,
    State(storage): State<StorageRuntime>,
    State(pg_runtime): State<PostgresRuntime>,
    State(security): State<ApiSecurityConfig>,
    State(compliance): State<ComplianceRuntime>,
    State(stores): State<OperationalStores>,
    headers: HeaderMap,
    Json(request): Json<RefreshTokenRequest>,
) -> Result<Response, ApiError> {
    let presented = resolve_refresh_token(request.refresh_token.as_deref(), &headers);

    let user_id = if let Some(ref token) = presented {
        crate::services::session_storage::load_refresh_token(
            &storage,
            Some(&pg_runtime),
            &security,
            stores.sessions.as_deref(),
            token,
        )
        .await?
        .map(|record| record.user_id)
    } else {
        None
    };

    if let Some(ref token) = presented {
        let _ = crate::services::session_storage::revoke_refresh_token(
            &storage,
            Some(&pg_runtime),
            &security,
            stores.sessions.as_deref(),
            token,
        )
        .await?;
    }

    // SPEC-083 / SPEC-154: durable denylist access-token jti when Bearer is presented.
    if let Some(header) = headers.get(AUTHORIZATION).and_then(|v| v.to_str().ok()) {
        if let Some(token) = header.strip_prefix("Bearer ") {
            let claims = auth
                .jwt
                .verify_token(token)
                .or_else(|_| auth.jwt.decode_unverified(token));
            if let Ok(claims) = claims {
                let expires_at = crate::services::jti_denylist::exp_claim_to_utc(claims.exp);
                let _ = crate::services::jti_denylist::revoke_jti_parts(
                    &auth.jwt,
                    pg_runtime.optional_pg_pool(),
                    &claims.jti,
                    expires_at,
                    "logout",
                )
                .await;
            }
        }
    }

    record_compliance_event_runtime(
        &compliance,
        "default",
        AuditEventType::Authentication,
        "logout",
        AuditResult::Success,
        None,
        user_id,
        None,
    );

    let secure = cookie_secure_from_headers(&headers);
    let mut response = StatusCode::NO_CONTENT.into_response();
    response
        .headers_mut()
        .insert(SET_COOKIE, clear_refresh_cookie_header(secure));
    Ok(response)
}

fn with_refresh_cookie<T: serde::Serialize>(
    body: T,
    refresh_token: &str,
    secure: bool,
) -> Response {
    let mut response = Json(body).into_response();
    response
        .headers_mut()
        .insert(SET_COOKIE, set_refresh_cookie_header(refresh_token, secure));
    response
}

/// Get current user information.
///
/// GET /api/v1/auth/me
#[utoipa::path(
    get,
    path = "/api/v1/auth/me",
    tag = "Authentication",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "User information", body = GetMeResponse),
        (status = 401, description = "Not authenticated")
    )
)]
pub async fn get_me(
    State(storage): State<StorageRuntime>,
    State(pg_runtime): State<PostgresRuntime>,
    State(security): State<ApiSecurityConfig>,
    State(stores): State<OperationalStores>,
    ApiAuthenticated(RequestAuthContext { user_id, .. }): ApiAuthenticated,
) -> Result<Json<GetMeResponse>, ApiError> {
    let user_record = get_record_by_id(
        &storage,
        Some(&pg_runtime),
        &security,
        stores.identity.as_deref(),
        &user_id,
    )
    .await?
    .ok_or_else(|| ApiError::NotFound(format!("User {} not found", user_id)))?;

    if !user_record.is_active {
        return Err(ApiError::forbidden_reason("account_inactive"));
    }

    Ok(Json(GetMeResponse {
        user: UserInfo::from(&user_record),
    }))
}
