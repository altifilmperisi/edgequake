//! Map a minted EdgeQuake access token onto its refresh family (EC-158-22).
//!
//! Back-channel logout never sees the bearer. The `jti` has to be stored when
//! the token is minted, then copied onto the SPEC-154 denylist at logout.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::error::ApiError;

use super::store::FederationStore;

/// Persist the `jti` of an access token this process just signed.
pub async fn remember_access_token(
    store: &dyn FederationStore,
    family_id: Uuid,
    access_token: &str,
) -> Result<(), ApiError> {
    let (jti, expires_at) = minted_jti(access_token)?;
    store.remember_access_jti(family_id, &jti, expires_at).await
}

fn minted_jti(token: &str) -> Result<(String, DateTime<Utc>), ApiError> {
    let payload = token
        .split('.')
        .nth(1)
        .ok_or_else(|| ApiError::Internal("access token has no payload".into()))?;
    use base64::Engine;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(payload))
        .map_err(|e| ApiError::Internal(format!("access token payload: {e}")))?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|e| ApiError::Internal(format!("access token payload: {e}")))?;
    let jti = value
        .get("jti")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ApiError::Internal("access token has no jti".into()))?
        .to_string();
    let exp = value
        .get("exp")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| ApiError::Internal("access token has no exp".into()))?;
    Ok((jti, crate::services::jti_denylist::exp_claim_to_utc(exp)))
}
