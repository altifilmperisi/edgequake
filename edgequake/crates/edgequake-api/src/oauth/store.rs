//! OAuth AS persistence: clients/codes in memory; refresh grants hashed (+ optional PG).

use sha2::{Digest, Sha256};

use crate::error::ApiError;
use crate::state::{AppState, StorageRuntime};

#[cfg(feature = "postgres")]
use super::types::OAuthRefreshStatus;
use super::types::{
    OAuthAuthorizationCode, OAuthClientRegistration, OAuthRefreshGrant, TakeRefreshOutcome,
};

pub fn hash_refresh_token(token: &str) -> String {
    format!("{:x}", Sha256::digest(token.as_bytes()))
}

pub async fn store_client(
    storage: &StorageRuntime,
    client: OAuthClientRegistration,
) -> Result<(), ApiError> {
    crate::services::auth_memory_store::store_oauth_client(&storage.auth_memory, client).await
}

pub async fn get_client(
    storage: &StorageRuntime,
    client_id: &str,
) -> Result<Option<OAuthClientRegistration>, ApiError> {
    crate::services::auth_memory_store::get_oauth_client(&storage.auth_memory, client_id).await
}

pub async fn store_code(
    storage: &StorageRuntime,
    code: OAuthAuthorizationCode,
) -> Result<(), ApiError> {
    crate::services::auth_memory_store::store_oauth_code(&storage.auth_memory, code).await
}

pub async fn take_code(
    storage: &StorageRuntime,
    code: &str,
) -> Result<Option<OAuthAuthorizationCode>, ApiError> {
    crate::services::auth_memory_store::take_oauth_code(&storage.auth_memory, code).await
}

pub async fn store_refresh(state: &AppState, grant: OAuthRefreshGrant) -> Result<(), ApiError> {
    #[cfg(feature = "postgres")]
    if let Some(pool) = state.pg_pool.as_ref() {
        return store_refresh_pg(pool, &grant).await;
    }
    crate::services::auth_memory_store::store_oauth_refresh(&state.storage.auth_memory, grant).await
}

pub async fn take_refresh(state: &AppState, token: &str) -> Result<TakeRefreshOutcome, ApiError> {
    #[cfg(feature = "postgres")]
    if let Some(pool) = state.pg_pool.as_ref() {
        return take_refresh_pg(pool, token).await;
    }
    crate::services::auth_memory_store::take_oauth_refresh(&state.storage.auth_memory, token).await
}

/// Revoke a refresh token (and its family). Returns true if a grant was found.
pub async fn revoke_refresh(state: &AppState, token: &str) -> Result<bool, ApiError> {
    #[cfg(feature = "postgres")]
    if let Some(pool) = state.pg_pool.as_ref() {
        return revoke_refresh_pg(pool, token).await;
    }
    crate::services::auth_memory_store::revoke_oauth_refresh(&state.storage.auth_memory, token)
        .await
}

/// Revoke every grant in a family (refresh-token reuse).
pub async fn revoke_refresh_family(
    state: &AppState,
    family_id: uuid::Uuid,
) -> Result<(), ApiError> {
    #[cfg(feature = "postgres")]
    if let Some(pool) = state.pg_pool.as_ref() {
        return revoke_family_pg(pool, family_id).await;
    }
    crate::services::auth_memory_store::revoke_oauth_refresh_family(
        &state.storage.auth_memory,
        family_id,
    )
    .await
}

#[cfg(feature = "postgres")]
async fn store_refresh_pg(pool: &sqlx::PgPool, grant: &OAuthRefreshGrant) -> Result<(), ApiError> {
    let token_hash = hash_refresh_token(&grant.token);
    sqlx::query(
        r#"
        INSERT INTO oauth_refresh_grants (
            token_hash, family_id, client_id, resource, scope, user_id, role,
            tenant_id, workspace_id, status, expires_at, created_at, updated_at
        ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,'active',$10,NOW(),NOW())
        "#,
    )
    .bind(&token_hash)
    .bind(grant.family_id)
    .bind(&grant.client_id)
    .bind(&grant.resource)
    .bind(&grant.scope)
    .bind(&grant.user_id)
    .bind(&grant.role)
    .bind(&grant.tenant_id)
    .bind(&grant.workspace_id)
    .bind(grant.expires_at)
    .execute(pool)
    .await
    .map_err(|e| ApiError::Internal(format!("oauth refresh store: {e}")))?;
    Ok(())
}

#[cfg(feature = "postgres")]
async fn take_refresh_pg(pool: &sqlx::PgPool, token: &str) -> Result<TakeRefreshOutcome, ApiError> {
    let token_hash = hash_refresh_token(token);
    let row = sqlx::query_as::<
        _,
        (
            uuid::Uuid,
            String,
            String,
            String,
            String,
            String,
            Option<String>,
            Option<String>,
            String,
            chrono::DateTime<chrono::Utc>,
        ),
    >(
        r#"
        SELECT family_id, client_id, resource, scope, user_id, role,
               tenant_id, workspace_id, status, expires_at
        FROM oauth_refresh_grants
        WHERE token_hash = $1
        "#,
    )
    .bind(&token_hash)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::Internal(format!("oauth refresh take: {e}")))?;

    let Some((
        family_id,
        client_id,
        resource,
        scope,
        user_id,
        role,
        tenant_id,
        workspace_id,
        status_s,
        expires_at,
    )) = row
    else {
        return Ok(TakeRefreshOutcome::Invalid);
    };

    let status = OAuthRefreshStatus::parse(&status_s).unwrap_or(OAuthRefreshStatus::Revoked);
    match status {
        OAuthRefreshStatus::Active => {
            let updated = sqlx::query(
                r#"
                UPDATE oauth_refresh_grants
                SET status = 'rotated', updated_at = NOW()
                WHERE token_hash = $1 AND status = 'active'
                "#,
            )
            .bind(&token_hash)
            .execute(pool)
            .await
            .map_err(|e| ApiError::Internal(format!("oauth refresh rotate: {e}")))?;
            if updated.rows_affected() == 0 {
                revoke_family_pg(pool, family_id).await?;
                return Ok(TakeRefreshOutcome::ReuseDetected { family_id });
            }
            Ok(TakeRefreshOutcome::Consumed(Box::new(OAuthRefreshGrant {
                token: String::new(),
                family_id,
                client_id,
                resource,
                scope,
                user_id,
                role,
                tenant_id,
                workspace_id,
                expires_at,
                status: OAuthRefreshStatus::Rotated,
            })))
        }
        OAuthRefreshStatus::Rotated | OAuthRefreshStatus::Revoked => {
            revoke_family_pg(pool, family_id).await?;
            Ok(TakeRefreshOutcome::ReuseDetected { family_id })
        }
    }
}

#[cfg(feature = "postgres")]
async fn revoke_refresh_pg(pool: &sqlx::PgPool, token: &str) -> Result<bool, ApiError> {
    let token_hash = hash_refresh_token(token);
    let family_id: Option<uuid::Uuid> =
        sqlx::query_scalar(r#"SELECT family_id FROM oauth_refresh_grants WHERE token_hash = $1"#)
            .bind(&token_hash)
            .fetch_optional(pool)
            .await
            .map_err(|e| ApiError::Internal(format!("oauth refresh revoke lookup: {e}")))?;
    let Some(family_id) = family_id else {
        return Ok(false);
    };
    revoke_family_pg(pool, family_id).await?;
    Ok(true)
}

#[cfg(feature = "postgres")]
async fn revoke_family_pg(pool: &sqlx::PgPool, family_id: uuid::Uuid) -> Result<(), ApiError> {
    sqlx::query(
        r#"
        UPDATE oauth_refresh_grants
        SET status = 'revoked', updated_at = NOW()
        WHERE family_id = $1 AND status <> 'revoked'
        "#,
    )
    .bind(family_id)
    .execute(pool)
    .await
    .map_err(|e| ApiError::Internal(format!("oauth refresh family revoke: {e}")))?;
    Ok(())
}
