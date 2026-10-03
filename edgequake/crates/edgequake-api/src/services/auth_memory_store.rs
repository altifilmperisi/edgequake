//! In-memory auth storage for tests without PostgreSQL (SPEC-027 phase 55).
//!
//! Authentication data **never** uses KV `auth:*` keys. Production SSOT is PostgreSQL;
//! this store is the sole non-PG fallback for identity and sessions (OIDC pending state moved to
//! `services/federation` — SPEC-158 LAW-158-11).

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::error::ApiError;
use crate::handlers::auth::{ApiKeyRecord, RefreshTokenRecord, UserRecord};
use crate::oauth::store::hash_refresh_token;
use crate::oauth::types::{
    OAuthAuthorizationCode, OAuthClientRegistration, OAuthRefreshGrant, OAuthRefreshStatus,
    TakeRefreshOutcome,
};

#[derive(Default)]
struct AuthMemoryState {
    users: HashMap<String, UserRecord>,
    username_index: HashMap<String, String>,
    email_index: HashMap<String, String>,
    refresh_tokens: HashMap<String, RefreshTokenRecord>,
    api_keys: HashMap<String, ApiKeyRecord>,
    oauth_clients: HashMap<String, OAuthClientRegistration>,
    oauth_codes: HashMap<String, OAuthAuthorizationCode>,
    oauth_refresh: HashMap<String, OAuthRefreshGrant>,
}

/// Process-local auth artifacts (test harness only — not KV).
#[derive(Clone, Default)]
pub struct AuthMemoryStore {
    inner: Arc<RwLock<AuthMemoryState>>,
}

impl AuthMemoryStore {
    pub fn new() -> Self {
        Self::default()
    }
}

// ── Users ───────────────────────────────────────────────────────────────────

pub(crate) async fn get_user_record_by_id(
    store: &AuthMemoryStore,
    user_id: &str,
) -> Result<Option<UserRecord>, ApiError> {
    Ok(store.inner.read().await.users.get(user_id).cloned())
}

pub(crate) async fn find_user_record_by_login(
    store: &AuthMemoryStore,
    login: &str,
) -> Result<Option<UserRecord>, ApiError> {
    let state = store.inner.read().await;
    let login_lower = login.to_ascii_lowercase();
    if let Some(user_id) = state.username_index.get(&login_lower) {
        return Ok(state.users.get(user_id).cloned());
    }
    if let Some(user_id) = state.email_index.get(&login_lower) {
        return Ok(state.users.get(user_id).cloned());
    }
    Ok(None)
}

pub(crate) async fn persist_user_record(
    store: &AuthMemoryStore,
    record: &UserRecord,
) -> Result<(), ApiError> {
    let mut state = store.inner.write().await;
    let username_key = record.username.to_ascii_lowercase();
    let email_key = record.email.to_ascii_lowercase();
    state
        .username_index
        .insert(username_key, record.user_id.clone());
    state.email_index.insert(email_key, record.user_id.clone());
    state.users.insert(record.user_id.clone(), record.clone());
    Ok(())
}

pub(crate) async fn delete_user_record(
    store: &AuthMemoryStore,
    record: &UserRecord,
) -> Result<(), ApiError> {
    let mut state = store.inner.write().await;
    state.users.remove(&record.user_id);
    state
        .username_index
        .remove(&record.username.to_ascii_lowercase());
    state.email_index.remove(&record.email.to_ascii_lowercase());
    Ok(())
}

pub(crate) async fn list_user_records(
    store: &AuthMemoryStore,
) -> Result<Vec<UserRecord>, ApiError> {
    Ok(store.inner.read().await.users.values().cloned().collect())
}

pub(crate) async fn update_email_index(
    store: &AuthMemoryStore,
    user_id: &str,
    old_email: &str,
    new_email: &str,
) -> Result<(), ApiError> {
    let mut state = store.inner.write().await;
    state.email_index.remove(&old_email.to_ascii_lowercase());
    state
        .email_index
        .insert(new_email.to_ascii_lowercase(), user_id.to_string());
    Ok(())
}

// ── Refresh tokens ──────────────────────────────────────────────────────────

pub(crate) async fn persist_refresh_token(
    store: &AuthMemoryStore,
    record: &RefreshTokenRecord,
) -> Result<(), ApiError> {
    store
        .inner
        .write()
        .await
        .refresh_tokens
        .insert(record.token.clone(), record.clone());
    Ok(())
}

pub(crate) async fn load_refresh_token(
    store: &AuthMemoryStore,
    token: &str,
) -> Result<Option<RefreshTokenRecord>, ApiError> {
    Ok(store.inner.read().await.refresh_tokens.get(token).cloned())
}

/// Outcome of consuming a web-session refresh token (SPEC-154 Wave 4).
#[derive(Debug)]
pub(crate) enum TakeWebRefreshOutcome {
    Consumed(Box<RefreshTokenRecord>),
    ReuseDetected {
        #[allow(dead_code)]
        family_id: uuid::Uuid,
    },
    Invalid,
}

pub(crate) async fn revoke_refresh_token(
    store: &AuthMemoryStore,
    token: &str,
) -> Result<bool, ApiError> {
    let mut state = store.inner.write().await;
    let Some(record) = state.refresh_tokens.get(token).cloned() else {
        return Ok(false);
    };
    revoke_family_web_locked(&mut state, record.family_id);
    Ok(true)
}

/// Revoke every token in a rotation family (SPEC-158 back-channel logout).
pub(crate) async fn revoke_refresh_family(
    store: &AuthMemoryStore,
    family_id: uuid::Uuid,
) -> Result<(), ApiError> {
    revoke_family_web_locked(&mut *store.inner.write().await, family_id);
    Ok(())
}

/// Consume an active web refresh token (rotate); reuse revokes the family.
pub(crate) async fn take_web_refresh(
    store: &AuthMemoryStore,
    token: &str,
) -> Result<TakeWebRefreshOutcome, ApiError> {
    let mut state = store.inner.write().await;
    let Some(record) = state.refresh_tokens.get_mut(token) else {
        return Ok(TakeWebRefreshOutcome::Invalid);
    };
    if record.expires_at < chrono::Utc::now() {
        return Ok(TakeWebRefreshOutcome::Invalid);
    }
    if record.is_active() {
        record.status = "rotated".to_string();
        record.revoked = true;
        let consumed = record.clone();
        return Ok(TakeWebRefreshOutcome::Consumed(Box::new(consumed)));
    }
    let family_id = record.family_id;
    revoke_family_web_locked(&mut state, family_id);
    Ok(TakeWebRefreshOutcome::ReuseDetected { family_id })
}

fn revoke_family_web_locked(state: &mut AuthMemoryState, family_id: uuid::Uuid) {
    for record in state.refresh_tokens.values_mut() {
        if record.family_id == family_id {
            record.status = "revoked".to_string();
            record.revoked = true;
        }
    }
}

// ── API keys ────────────────────────────────────────────────────────────────

pub(crate) async fn persist_api_key(
    store: &AuthMemoryStore,
    record: &ApiKeyRecord,
) -> Result<(), ApiError> {
    store
        .inner
        .write()
        .await
        .api_keys
        .insert(record.key_id.clone(), record.clone());
    Ok(())
}

pub(crate) async fn list_api_keys_for_user(
    store: &AuthMemoryStore,
    user_id: &str,
) -> Result<Vec<ApiKeyRecord>, ApiError> {
    let mut records: Vec<ApiKeyRecord> = store
        .inner
        .read()
        .await
        .api_keys
        .values()
        .filter(|r| r.user_id == user_id)
        .cloned()
        .collect();
    records.sort_by_key(|b| std::cmp::Reverse(b.created_at));
    Ok(records)
}

pub(crate) async fn find_active_api_keys_by_prefix(
    store: &AuthMemoryStore,
    prefix: &str,
) -> Result<Vec<ApiKeyRecord>, ApiError> {
    Ok(store
        .inner
        .read()
        .await
        .api_keys
        .values()
        .filter(|r| r.is_active && r.prefix == prefix)
        .cloned()
        .collect())
}

pub(crate) async fn revoke_api_key(
    store: &AuthMemoryStore,
    key_id: &str,
    owner_user_id: &str,
) -> Result<Option<ApiKeyRecord>, ApiError> {
    let mut state = store.inner.write().await;
    if let Some(record) = state.api_keys.get_mut(key_id) {
        if record.user_id != owner_user_id {
            return Ok(None);
        }
        record.is_active = false;
        return Ok(Some(record.clone()));
    }
    Ok(None)
}

// ── MCP OAuth AS (ephemeral client / code / refresh) ─────────────────────────

pub(crate) async fn store_oauth_client(
    store: &AuthMemoryStore,
    client: OAuthClientRegistration,
) -> Result<(), ApiError> {
    store
        .inner
        .write()
        .await
        .oauth_clients
        .insert(client.client_id.clone(), client);
    Ok(())
}

pub(crate) async fn get_oauth_client(
    store: &AuthMemoryStore,
    client_id: &str,
) -> Result<Option<OAuthClientRegistration>, ApiError> {
    Ok(store
        .inner
        .read()
        .await
        .oauth_clients
        .get(client_id)
        .cloned())
}

pub(crate) async fn store_oauth_code(
    store: &AuthMemoryStore,
    code: OAuthAuthorizationCode,
) -> Result<(), ApiError> {
    store
        .inner
        .write()
        .await
        .oauth_codes
        .insert(code.code.clone(), code);
    Ok(())
}

pub(crate) async fn take_oauth_code(
    store: &AuthMemoryStore,
    code: &str,
) -> Result<Option<OAuthAuthorizationCode>, ApiError> {
    Ok(store.inner.write().await.oauth_codes.remove(code))
}

pub(crate) async fn store_oauth_refresh(
    store: &AuthMemoryStore,
    mut grant: OAuthRefreshGrant,
) -> Result<(), ApiError> {
    let hash = hash_refresh_token(&grant.token);
    grant.status = OAuthRefreshStatus::Active;
    // Do not keep plaintext in the map.
    grant.token.clear();
    store.inner.write().await.oauth_refresh.insert(hash, grant);
    Ok(())
}

pub(crate) async fn take_oauth_refresh(
    store: &AuthMemoryStore,
    token: &str,
) -> Result<TakeRefreshOutcome, ApiError> {
    let hash = hash_refresh_token(token);
    let mut inner = store.inner.write().await;
    let outcome = {
        let Some(grant) = inner.oauth_refresh.get_mut(&hash) else {
            return Ok(TakeRefreshOutcome::Invalid);
        };
        match grant.status {
            OAuthRefreshStatus::Active => {
                grant.status = OAuthRefreshStatus::Rotated;
                let mut out = grant.clone();
                out.token.clear();
                TakeRefreshOutcome::Consumed(Box::new(out))
            }
            OAuthRefreshStatus::Rotated | OAuthRefreshStatus::Revoked => {
                TakeRefreshOutcome::ReuseDetected {
                    family_id: grant.family_id,
                }
            }
        }
    };
    if let TakeRefreshOutcome::ReuseDetected { family_id } = outcome {
        revoke_family_locked(&mut inner, family_id);
        return Ok(TakeRefreshOutcome::ReuseDetected { family_id });
    }
    Ok(outcome)
}

pub(crate) async fn revoke_oauth_refresh(
    store: &AuthMemoryStore,
    token: &str,
) -> Result<bool, ApiError> {
    let hash = hash_refresh_token(token);
    let mut inner = store.inner.write().await;
    let Some(grant) = inner.oauth_refresh.get(&hash) else {
        return Ok(false);
    };
    let family_id = grant.family_id;
    revoke_family_locked(&mut inner, family_id);
    Ok(true)
}

pub(crate) async fn revoke_oauth_refresh_family(
    store: &AuthMemoryStore,
    family_id: uuid::Uuid,
) -> Result<(), ApiError> {
    let mut inner = store.inner.write().await;
    revoke_family_locked(&mut inner, family_id);
    Ok(())
}

fn revoke_family_locked(inner: &mut AuthMemoryState, family_id: uuid::Uuid) {
    for grant in inner.oauth_refresh.values_mut() {
        if grant.family_id == family_id {
            grant.status = OAuthRefreshStatus::Revoked;
        }
    }
}
