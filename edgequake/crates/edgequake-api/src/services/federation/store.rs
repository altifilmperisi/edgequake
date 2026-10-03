//! Federation persistence port (SPEC-158 — Dependency Inversion).
//!
//! One trait, two adapters: [`MemoryFederationStore`](super::memory_store) for tests / no-PG
//! harnesses and `PgFederationStore` (feature `postgres`). Handlers and services depend only on
//! this trait (LAW-158-11: durable, multi-replica-safe state in production).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::ApiError;

/// `(issuer, subject)` → local user. The federation key (LAW-158-3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FederatedIdentityRecord {
    pub federated_id: Uuid,
    pub user_id: Uuid,
    pub provider_slug: String,
    pub issuer: String,
    pub subject: String,
    pub email_at_link: Option<String>,
    pub email_verified_at_link: bool,
    pub linked_at: DateTime<Utc>,
    pub last_login_at: DateTime<Utc>,
}

/// Outcome of inserting an identity under `UNIQUE(issuer, subject)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkOutcome {
    Created,
    /// Lost a race: the winner's record is returned.
    AlreadyLinked(Box<FederatedIdentityRecord>),
}

/// Pending authorization request (state → PKCE/nonce), TTL-bound and single-use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginAttempt {
    pub state: String,
    pub provider_slug: String,
    pub pkce_verifier: String,
    pub nonce: String,
    pub organization_hint: Option<String>,
    pub redirect_after: Option<String>,
    pub expires_at: DateTime<Utc>,
}

/// Single-use SPA handoff (no bearer material is stored — tokens are minted at redeem).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandoffRecord {
    pub code_hash: String,
    pub user_id: Uuid,
    pub family_id: Uuid,
    pub provider_slug: String,
    pub tenant_id: Uuid,
    pub workspace_id: Option<Uuid>,
    pub redirect_after: Option<String>,
    pub expires_at: DateTime<Utc>,
}

/// Refresh-family → tenant scope + IdP session binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FederatedSession {
    pub family_id: Uuid,
    pub user_id: Uuid,
    pub provider_slug: String,
    pub issuer: String,
    pub subject: String,
    pub idp_sid: Option<String>,
    pub tenant_id: Uuid,
    pub workspace_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

/// Which sessions a back-channel logout token targets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionSelector {
    Sid { issuer: String, sid: String },
    Subject { issuer: String, subject: String },
}

/// Persisted provider definition (`identity_providers` row).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct StoredProvider {
    pub slug: String,
    pub kind: String,
    pub display_name: String,
    pub issuer: String,
    pub client_id: String,
    /// Environment variable name holding the client secret (never the secret).
    pub client_secret_ref: Option<String>,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
    pub trust_email: bool,
    pub link_policy: String,
    pub jit_enabled: bool,
    pub role_claim: String,
    pub role_map: serde_json::Value,
    pub max_role: String,
    pub enabled: bool,
    pub metadata: serde_json::Value,
}

#[async_trait]
pub trait FederationStore: Send + Sync {
    // ── identities ──
    async fn find_identity(
        &self,
        issuer: &str,
        subject: &str,
    ) -> Result<Option<FederatedIdentityRecord>, ApiError>;
    async fn link_identity(
        &self,
        record: &FederatedIdentityRecord,
    ) -> Result<LinkOutcome, ApiError>;
    async fn touch_identity(&self, issuer: &str, subject: &str) -> Result<(), ApiError>;

    // ── pending authorization (LAW-158-11) ──
    async fn put_login_attempt(&self, attempt: &LoginAttempt) -> Result<(), ApiError>;
    /// Atomic take: returns `None` when missing, replayed, or expired.
    async fn take_login_attempt(&self, state: &str) -> Result<Option<LoginAttempt>, ApiError>;

    // ── handoff (LAW-158-4) ──
    async fn put_handoff(&self, record: &HandoffRecord) -> Result<(), ApiError>;
    /// Atomic single-use take: `None` when missing, replayed, or expired.
    async fn take_handoff(&self, code_hash: &str) -> Result<Option<HandoffRecord>, ApiError>;

    // ── sessions / back-channel logout (LAW-158-10) ──
    async fn put_session(&self, session: &FederatedSession) -> Result<(), ApiError>;
    async fn get_session(&self, family_id: Uuid) -> Result<Option<FederatedSession>, ApiError>;
    /// Active (non-revoked) sessions matching the selector.
    async fn sessions_for(
        &self,
        selector: &SessionSelector,
    ) -> Result<Vec<FederatedSession>, ApiError>;
    async fn mark_session_revoked(&self, family_id: Uuid) -> Result<(), ApiError>;
    /// Remember an access-token `jti` minted for this refresh family (EC-158-22).
    async fn remember_access_jti(
        &self,
        family_id: Uuid,
        jti: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), ApiError>;
    /// Unexpired access-token jti values for the family.
    async fn access_jtis(&self, family_id: Uuid) -> Result<Vec<(String, DateTime<Utc>)>, ApiError>;
    /// Record a logout-token `jti`; `false` when it was already seen (replay).
    async fn record_logout_jti(
        &self,
        issuer: &str,
        jti: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<bool, ApiError>;

    // ── providers (LAW-158-7) ──
    async fn list_providers(&self) -> Result<Vec<StoredProvider>, ApiError>;
    async fn upsert_provider(&self, provider: &StoredProvider) -> Result<(), ApiError>;
    async fn delete_provider(&self, slug: &str) -> Result<bool, ApiError>;

    /// Delete expired ephemeral rows; returns rows removed.
    async fn purge_expired(&self) -> Result<u64, ApiError>;
}

pub type SharedFederationStore = std::sync::Arc<dyn FederationStore>;
