//! Single-use SPA handoff (SPEC-158 LAW-158-4 / EC-158-10, EC-158-29).
//!
//! The callback never puts tokens in a URL. It redirects with an opaque one-time `code`; the SPA
//! redeems it over `POST /api/v1/auth/handoff`. Only the SHA-256 of the code is stored, and no
//! bearer material is persisted — the access token is minted at redeem time.

use base64::Engine;
use chrono::{Duration, Utc};
use rand::RngCore;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::error::ApiError;
use crate::services::login_tokens::SessionScope;

use super::store::{HandoffRecord, SharedFederationStore};

pub const HANDOFF_TTL_SECS: i64 = 90;

/// 256-bit URL-safe random code.
fn random_code() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

pub fn hash_code(code: &str) -> String {
    hex::encode(Sha256::digest(code.as_bytes()))
}

pub struct NewHandoff<'a> {
    pub user_id: Uuid,
    pub family_id: Uuid,
    pub provider_slug: &'a str,
    pub scope: SessionScope,
    pub redirect_after: Option<String>,
}

/// Persist a handoff and return the one-time code (shown to the browser exactly once).
pub async fn create_handoff(
    store: &SharedFederationStore,
    new: NewHandoff<'_>,
) -> Result<String, ApiError> {
    let code = random_code();
    store
        .put_handoff(&HandoffRecord {
            code_hash: hash_code(&code),
            user_id: new.user_id,
            family_id: new.family_id,
            provider_slug: new.provider_slug.to_string(),
            tenant_id: new.scope.tenant_id,
            workspace_id: new.scope.workspace_id,
            redirect_after: new.redirect_after,
            expires_at: Utc::now() + Duration::seconds(HANDOFF_TTL_SECS),
        })
        .await?;
    Ok(code)
}

/// Atomically consume a handoff. `None` for unknown / replayed / expired codes.
pub async fn redeem_handoff(
    store: &SharedFederationStore,
    code: &str,
) -> Result<Option<HandoffRecord>, ApiError> {
    if code.is_empty() || code.len() > 256 {
        return Ok(None);
    }
    store.take_handoff(&hash_code(code)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::federation::memory_store::MemoryFederationStore;
    use std::sync::Arc;

    fn store() -> SharedFederationStore {
        Arc::new(MemoryFederationStore::new())
    }

    fn new_handoff() -> NewHandoff<'static> {
        NewHandoff {
            user_id: Uuid::new_v4(),
            family_id: Uuid::new_v4(),
            provider_slug: "p",
            scope: SessionScope::default_scope(),
            redirect_after: Some("/documents".into()),
        }
    }

    #[tokio::test]
    async fn code_redeems_once_only() {
        let s = store();
        let code = create_handoff(&s, new_handoff()).await.unwrap();
        assert!(code.len() >= 43, "256-bit code");
        assert!(redeem_handoff(&s, &code).await.unwrap().is_some());
        assert!(
            redeem_handoff(&s, &code).await.unwrap().is_none(),
            "replay must fail"
        );
    }

    #[tokio::test]
    async fn unknown_or_oversized_codes_fail() {
        let s = store();
        assert!(redeem_handoff(&s, "nope").await.unwrap().is_none());
        assert!(redeem_handoff(&s, &"x".repeat(1000))
            .await
            .unwrap()
            .is_none());
        assert!(redeem_handoff(&s, "").await.unwrap().is_none());
    }

    #[test]
    fn stored_value_is_a_hash_not_the_code() {
        assert_ne!(hash_code("abc"), "abc");
        assert_eq!(hash_code("abc").len(), 64);
    }
}
