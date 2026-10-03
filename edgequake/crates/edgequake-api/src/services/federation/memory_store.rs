//! In-memory [`FederationStore`] (tests and PG-less harnesses). Process-local only —
//! production multi-replica deployments use the PostgreSQL adapter (LAW-158-11).

use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::error::ApiError;

use super::store::{
    FederatedIdentityRecord, FederatedSession, FederationStore, HandoffRecord, LinkOutcome,
    LoginAttempt, SessionSelector, StoredProvider,
};

#[derive(Default)]
struct State {
    identities: HashMap<(String, String), FederatedIdentityRecord>,
    attempts: HashMap<String, LoginAttempt>,
    handoffs: HashMap<String, HandoffRecord>,
    sessions: HashMap<Uuid, FederatedSession>,
    /// jti → (family, expiry). Back-channel logout denylists every live row.
    access_jti: HashMap<String, (Uuid, DateTime<Utc>)>,
    logout_jti: HashMap<(String, String), DateTime<Utc>>,
    providers: HashMap<String, StoredProvider>,
}

#[derive(Default)]
pub struct MemoryFederationStore {
    state: Mutex<State>,
}

impl MemoryFederationStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl FederationStore for MemoryFederationStore {
    async fn find_identity(
        &self,
        issuer: &str,
        subject: &str,
    ) -> Result<Option<FederatedIdentityRecord>, ApiError> {
        let s = self.state.lock().await;
        Ok(s.identities.get(&(issuer.into(), subject.into())).cloned())
    }

    async fn link_identity(
        &self,
        record: &FederatedIdentityRecord,
    ) -> Result<LinkOutcome, ApiError> {
        let mut s = self.state.lock().await;
        let key = (record.issuer.clone(), record.subject.clone());
        if let Some(existing) = s.identities.get(&key) {
            return Ok(LinkOutcome::AlreadyLinked(Box::new(existing.clone())));
        }
        s.identities.insert(key, record.clone());
        Ok(LinkOutcome::Created)
    }

    async fn touch_identity(&self, issuer: &str, subject: &str) -> Result<(), ApiError> {
        let mut s = self.state.lock().await;
        if let Some(r) = s.identities.get_mut(&(issuer.into(), subject.into())) {
            r.last_login_at = Utc::now();
        }
        Ok(())
    }

    async fn put_login_attempt(&self, attempt: &LoginAttempt) -> Result<(), ApiError> {
        self.state
            .lock()
            .await
            .attempts
            .insert(attempt.state.clone(), attempt.clone());
        Ok(())
    }

    async fn take_login_attempt(&self, state: &str) -> Result<Option<LoginAttempt>, ApiError> {
        let taken = self.state.lock().await.attempts.remove(state);
        Ok(taken.filter(|a| a.expires_at > Utc::now()))
    }

    async fn put_handoff(&self, record: &HandoffRecord) -> Result<(), ApiError> {
        self.state
            .lock()
            .await
            .handoffs
            .insert(record.code_hash.clone(), record.clone());
        Ok(())
    }

    async fn take_handoff(&self, code_hash: &str) -> Result<Option<HandoffRecord>, ApiError> {
        let taken = self.state.lock().await.handoffs.remove(code_hash);
        Ok(taken.filter(|h| h.expires_at > Utc::now()))
    }

    async fn put_session(&self, session: &FederatedSession) -> Result<(), ApiError> {
        self.state
            .lock()
            .await
            .sessions
            .insert(session.family_id, session.clone());
        Ok(())
    }

    async fn get_session(&self, family_id: Uuid) -> Result<Option<FederatedSession>, ApiError> {
        Ok(self.state.lock().await.sessions.get(&family_id).cloned())
    }

    async fn sessions_for(
        &self,
        selector: &SessionSelector,
    ) -> Result<Vec<FederatedSession>, ApiError> {
        let s = self.state.lock().await;
        Ok(s.sessions
            .values()
            .filter(|x| x.revoked_at.is_none())
            .filter(|x| match selector {
                SessionSelector::Sid { issuer, sid } => {
                    &x.issuer == issuer && x.idp_sid.as_deref() == Some(sid.as_str())
                }
                SessionSelector::Subject { issuer, subject } => {
                    &x.issuer == issuer && &x.subject == subject
                }
            })
            .cloned()
            .collect())
    }

    async fn mark_session_revoked(&self, family_id: Uuid) -> Result<(), ApiError> {
        if let Some(x) = self.state.lock().await.sessions.get_mut(&family_id) {
            x.revoked_at = Some(Utc::now());
        }
        Ok(())
    }

    async fn remember_access_jti(
        &self,
        family_id: Uuid,
        jti: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), ApiError> {
        self.state
            .lock()
            .await
            .access_jti
            .insert(jti.to_string(), (family_id, expires_at));
        Ok(())
    }

    async fn access_jtis(&self, family_id: Uuid) -> Result<Vec<(String, DateTime<Utc>)>, ApiError> {
        let now = Utc::now();
        Ok(self
            .state
            .lock()
            .await
            .access_jti
            .iter()
            .filter(|(_, (family, exp))| *family == family_id && *exp > now)
            .map(|(jti, (_, exp))| (jti.clone(), *exp))
            .collect())
    }

    async fn record_logout_jti(
        &self,
        issuer: &str,
        jti: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<bool, ApiError> {
        let mut s = self.state.lock().await;
        let key = (issuer.to_string(), jti.to_string());
        if s.logout_jti.get(&key).is_some_and(|exp| *exp > Utc::now()) {
            return Ok(false);
        }
        s.logout_jti.insert(key, expires_at);
        Ok(true)
    }

    async fn list_providers(&self) -> Result<Vec<StoredProvider>, ApiError> {
        let mut all: Vec<_> = self
            .state
            .lock()
            .await
            .providers
            .values()
            .cloned()
            .collect();
        all.sort_by(|a, b| a.slug.cmp(&b.slug));
        Ok(all)
    }

    async fn upsert_provider(&self, provider: &StoredProvider) -> Result<(), ApiError> {
        self.state
            .lock()
            .await
            .providers
            .insert(provider.slug.clone(), provider.clone());
        Ok(())
    }

    async fn delete_provider(&self, slug: &str) -> Result<bool, ApiError> {
        Ok(self.state.lock().await.providers.remove(slug).is_some())
    }

    async fn purge_expired(&self) -> Result<u64, ApiError> {
        let now = Utc::now();
        let mut s = self.state.lock().await;
        let before = s.attempts.len() + s.handoffs.len() + s.logout_jti.len() + s.access_jti.len();
        s.attempts.retain(|_, a| a.expires_at > now);
        s.handoffs.retain(|_, h| h.expires_at > now);
        s.logout_jti.retain(|_, exp| *exp > now);
        s.access_jti.retain(|_, (_, exp)| *exp > now);
        let after = s.attempts.len() + s.handoffs.len() + s.logout_jti.len() + s.access_jti.len();
        Ok((before - after) as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn attempt(state: &str, ttl_secs: i64) -> LoginAttempt {
        LoginAttempt {
            state: state.into(),
            provider_slug: "p".into(),
            pkce_verifier: "v".into(),
            nonce: "n".into(),
            organization_hint: None,
            redirect_after: None,
            expires_at: Utc::now() + Duration::seconds(ttl_secs),
        }
    }

    #[tokio::test]
    async fn login_attempt_is_single_use_and_expires() {
        let store = MemoryFederationStore::new();
        store.put_login_attempt(&attempt("s1", 60)).await.unwrap();
        assert!(store.take_login_attempt("s1").await.unwrap().is_some());
        assert!(store.take_login_attempt("s1").await.unwrap().is_none());
        store.put_login_attempt(&attempt("s2", -1)).await.unwrap();
        assert!(store.take_login_attempt("s2").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn identity_key_is_issuer_and_subject() {
        let store = MemoryFederationStore::new();
        let mk = |issuer: &str| FederatedIdentityRecord {
            federated_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            provider_slug: "p".into(),
            issuer: issuer.into(),
            subject: "same-sub".into(),
            email_at_link: None,
            email_verified_at_link: false,
            linked_at: Utc::now(),
            last_login_at: Utc::now(),
        };
        let a = mk("https://a");
        let b = mk("https://b");
        assert_eq!(store.link_identity(&a).await.unwrap(), LinkOutcome::Created);
        assert_eq!(store.link_identity(&b).await.unwrap(), LinkOutcome::Created);
        assert!(matches!(
            store.link_identity(&mk("https://a")).await.unwrap(),
            LinkOutcome::AlreadyLinked(_)
        ));
        assert_eq!(
            store
                .find_identity("https://a", "same-sub")
                .await
                .unwrap()
                .unwrap()
                .user_id,
            a.user_id
        );
    }

    #[tokio::test]
    async fn logout_jti_replay_is_detected() {
        let store = MemoryFederationStore::new();
        let exp = Utc::now() + Duration::minutes(5);
        assert!(store.record_logout_jti("i", "j", exp).await.unwrap());
        assert!(!store.record_logout_jti("i", "j", exp).await.unwrap());
    }
}
