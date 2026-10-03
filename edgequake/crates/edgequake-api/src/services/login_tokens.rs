//! Shared session issuer (SPEC-158 DRY): password login, SSO callback, handoff redeem and
//! refresh rotation all mint access + refresh tokens through this single module.

use chrono::{Duration, Utc};
use uuid::Uuid;

use edgequake_auth::Role;

use crate::error::ApiError;
use crate::handlers::auth::RefreshTokenRecord;
use crate::state::{
    ApiSecurityConfig, AuthRuntime, OperationalStores, PostgresRuntime, StorageRuntime,
};

/// Tenant/workspace bound into the access token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionScope {
    pub tenant_id: Uuid,
    pub workspace_id: Option<Uuid>,
}

impl SessionScope {
    /// Legacy single-tenant scope (password login, anonymous, API keys).
    pub fn default_scope() -> Self {
        let (tenant_id, workspace_id) = crate::services::identity_storage::default_identity_scope();
        Self {
            tenant_id,
            workspace_id: Some(workspace_id),
        }
    }
}

/// Borrowed runtime handles needed to issue a session (ISP — no full `AppState`).
pub struct SessionContext<'a> {
    pub auth: &'a AuthRuntime,
    pub storage: &'a StorageRuntime,
    pub pg: &'a PostgresRuntime,
    pub security: &'a ApiSecurityConfig,
    pub stores: &'a OperationalStores,
}

#[derive(Debug, Clone)]
pub struct IssuedSession {
    pub access_token: String,
    pub expires_in: i64,
    pub refresh_token: String,
    pub family_id: Uuid,
}

const REFRESH_TTL_DAYS: i64 = 30;

/// Sign an access JWT for `user_id` bound to `scope`.
pub fn mint_access_token(
    auth: &AuthRuntime,
    user_id: Uuid,
    role: Role,
    scope: &SessionScope,
) -> Result<(String, i64), ApiError> {
    let expires_in = auth.jwt.expiry_duration().as_secs() as i64;
    let claims = crate::services::identity_storage::access_token_claims_scoped(
        user_id,
        role,
        expires_in,
        scope.tenant_id,
        scope.workspace_id,
    );
    let token = auth
        .jwt
        .generate_token_with_claims(claims)
        .map_err(|e| ApiError::Internal(format!("token_generation failed: {e}")))?;
    Ok((token, expires_in))
}

/// Persist a fresh refresh token in `family_id`; returns the opaque token string.
pub(crate) async fn persist_new_refresh(
    ctx: &SessionContext<'_>,
    user_id: &str,
    family_id: Uuid,
) -> Result<String, ApiError> {
    let token = Uuid::new_v4().to_string();
    let record = RefreshTokenRecord {
        token: token.clone(),
        user_id: user_id.to_string(),
        family_id,
        status: "active".to_string(),
        created_at: Utc::now(),
        expires_at: Utc::now() + Duration::days(REFRESH_TTL_DAYS),
        revoked: false,
    };
    crate::services::session_storage::persist_refresh_token(
        ctx.storage,
        Some(ctx.pg),
        ctx.security,
        ctx.stores.sessions.as_deref(),
        &record,
    )
    .await?;
    Ok(token)
}

/// Start a brand-new session (new refresh family).
pub(crate) async fn issue_session(
    ctx: &SessionContext<'_>,
    user_id: &str,
    role: Role,
    scope: &SessionScope,
) -> Result<IssuedSession, ApiError> {
    let user_uuid = Uuid::parse_str(user_id)
        .map_err(|_| ApiError::Internal("Invalid user ID format".into()))?;
    let (access_token, expires_in) = mint_access_token(ctx.auth, user_uuid, role, scope)?;
    let family_id = Uuid::new_v4();
    let refresh_token = persist_new_refresh(ctx, user_id, family_id).await?;
    Ok(IssuedSession {
        access_token,
        expires_in,
        refresh_token,
        family_id,
    })
}
