//! Durable JWT `jti` denylist (SPEC-154 Wave 4 / LAW-154-8).

use chrono::{DateTime, TimeZone, Utc};
use edgequake_auth::JwtService;

use crate::error::ApiError;
use crate::services::OptionalPgPool;
use crate::state::AppState;

/// Persist a revoked jti until access-token expiry (best-effort when PG available).
#[allow(dead_code)] // Available for AppState-centric call sites / future admin revoke.
pub(crate) async fn revoke_jti_durable(
    state: &AppState,
    jti: &str,
    expires_at: DateTime<Utc>,
    reason: &str,
) -> Result<(), ApiError> {
    revoke_jti_parts(
        &state.auth.jwt,
        state.pg_pool.as_ref(),
        jti,
        expires_at,
        reason,
    )
    .await
}

/// Core revoke used by handlers that already hold JwtService + optional pool.
pub(crate) async fn revoke_jti_parts(
    jwt: &JwtService,
    pool: OptionalPgPool<'_>,
    jti: &str,
    expires_at: DateTime<Utc>,
    reason: &str,
) -> Result<(), ApiError> {
    jwt.revoke_jti(jti);

    #[cfg(feature = "postgres")]
    {
        if let Some(pool) = pool {
            let _ = sqlx::query(
                r#"
                INSERT INTO jwt_jti_denylist (jti, expires_at, revoked_at, reason)
                VALUES ($1, $2, NOW(), $3)
                ON CONFLICT (jti) DO UPDATE
                  SET expires_at = EXCLUDED.expires_at,
                      revoked_at = NOW(),
                      reason = EXCLUDED.reason
                "#,
            )
            .bind(jti)
            .bind(expires_at)
            .bind(reason)
            .execute(pool)
            .await;
        }
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (pool, expires_at, reason);
    }
    Ok(())
}

/// True when jti is revoked in-process or in durable store.
pub(crate) async fn is_jti_revoked_durable(state: &AppState, jti: &str) -> bool {
    is_jti_revoked_parts(&state.auth.jwt, state.pg_pool.as_ref(), jti).await
}

pub(crate) async fn is_jti_revoked_parts(
    jwt: &JwtService,
    pool: OptionalPgPool<'_>,
    jti: &str,
) -> bool {
    if jwt.is_jti_revoked(jti) {
        return true;
    }

    #[cfg(feature = "postgres")]
    {
        if let Some(pool) = pool {
            let found = sqlx::query_scalar::<_, bool>(
                r#"
                SELECT EXISTS(
                  SELECT 1 FROM jwt_jti_denylist
                  WHERE jti = $1 AND expires_at > NOW()
                )
                "#,
            )
            .bind(jti)
            .fetch_one(pool)
            .await
            .unwrap_or(false);
            if found {
                jwt.revoke_jti(jti);
                return true;
            }
        }
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (pool, jti);
    }
    false
}

/// Convert JWT `exp` claim (unix seconds) to UTC datetime.
pub(crate) fn exp_claim_to_utc(exp: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(exp, 0)
        .single()
        .unwrap_or_else(Utc::now)
}
