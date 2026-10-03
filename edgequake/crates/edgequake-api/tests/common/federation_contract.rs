//! One behavioural contract for every `FederationStore` backend (memory + PostgreSQL) — DRY.

use std::sync::Arc;

use chrono::{Duration, Utc};
use edgequake_api::services::federation::store::{
    FederatedIdentityRecord, FederatedSession, FederationStore, HandoffRecord, LinkOutcome,
    LoginAttempt, SessionSelector, StoredProvider,
};
use serde_json::json;
use uuid::Uuid;

pub type Store = Arc<dyn FederationStore>;

fn identity(user_id: Uuid, subject: &str) -> FederatedIdentityRecord {
    FederatedIdentityRecord {
        federated_id: Uuid::new_v4(),
        user_id,
        provider_slug: "idp".into(),
        issuer: "https://idp.test".into(),
        subject: subject.into(),
        email_at_link: Some("a@x.test".into()),
        email_verified_at_link: true,
        linked_at: Utc::now(),
        last_login_at: Utc::now(),
    }
}

fn attempt(state: &str, ttl_secs: i64) -> LoginAttempt {
    LoginAttempt {
        state: state.into(),
        provider_slug: "idp".into(),
        pkce_verifier: "pkce".into(),
        nonce: "nonce".into(),
        organization_hint: Some("acme".into()),
        redirect_after: Some("/documents".into()),
        expires_at: Utc::now() + Duration::seconds(ttl_secs),
    }
}

fn handoff(hash: &str, ttl_secs: i64) -> HandoffRecord {
    HandoffRecord {
        code_hash: hash.into(),
        user_id: Uuid::new_v4(),
        family_id: Uuid::new_v4(),
        provider_slug: "idp".into(),
        tenant_id: Uuid::new_v4(),
        workspace_id: None,
        redirect_after: None,
        expires_at: Utc::now() + Duration::seconds(ttl_secs),
    }
}

fn session(user_id: Uuid, sid: Option<&str>, subject: &str) -> FederatedSession {
    FederatedSession {
        family_id: Uuid::new_v4(),
        user_id,
        provider_slug: "idp".into(),
        issuer: "https://idp.test".into(),
        subject: subject.into(),
        idp_sid: sid.map(str::to_string),
        tenant_id: Uuid::new_v4(),
        workspace_id: Some(Uuid::new_v4()),
        created_at: Utc::now(),
        revoked_at: None,
    }
}

fn provider(slug: &str) -> StoredProvider {
    StoredProvider {
        slug: slug.into(),
        kind: "keycloak".into(),
        display_name: "Acme".into(),
        issuer: "https://kc.test/realms/a".into(),
        client_id: "c".into(),
        client_secret_ref: Some("ACME_SECRET".into()),
        redirect_uri: "https://eq.test/cb".into(),
        scopes: vec!["profile".into()],
        trust_email: false,
        link_policy: "never".into(),
        jit_enabled: true,
        role_claim: "realm_access.roles".into(),
        role_map: json!({"eq-admin": "admin"}),
        max_role: "admin".into(),
        enabled: true,
        metadata: json!({"require_org": true}),
    }
}

/// `user_id` must satisfy any FK the backend enforces (PG: row in `users`).
pub async fn run(store: Store, user_id: Uuid) {
    identities(&store, user_id).await;
    login_attempts(&store).await;
    handoffs(&store).await;
    sessions(&store, user_id).await;
    logout_jti(&store).await;
    providers(&store).await;
    store.purge_expired().await.expect("purge");
}

async fn identities(store: &Store, user_id: Uuid) {
    let first = identity(user_id, "sub-1");
    assert!(matches!(
        store.link_identity(&first).await.unwrap(),
        LinkOutcome::Created
    ));
    // UNIQUE(issuer, subject): a second insert for the same pair never creates a duplicate.
    let dup = identity(user_id, "sub-1");
    match store.link_identity(&dup).await.unwrap() {
        LinkOutcome::AlreadyLinked(existing) => {
            assert_eq!(existing.federated_id, first.federated_id)
        }
        LinkOutcome::Created => panic!("duplicate (issuer, subject) was created"),
    }
    let found = store
        .find_identity("https://idp.test", "sub-1")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.user_id, user_id);
    assert!(store
        .find_identity("https://other.test", "sub-1")
        .await
        .unwrap()
        .is_none());
    store
        .touch_identity("https://idp.test", "sub-1")
        .await
        .unwrap();
}

async fn login_attempts(store: &Store) {
    store
        .put_login_attempt(&attempt("state-1", 60))
        .await
        .unwrap();
    let got = store
        .take_login_attempt("state-1")
        .await
        .unwrap()
        .expect("first take");
    assert_eq!(got.organization_hint.as_deref(), Some("acme"));
    assert!(
        store.take_login_attempt("state-1").await.unwrap().is_none(),
        "replay"
    );

    store
        .put_login_attempt(&attempt("state-expired", -5))
        .await
        .unwrap();
    assert!(
        store
            .take_login_attempt("state-expired")
            .await
            .unwrap()
            .is_none(),
        "expired"
    );

    // Concurrent redemption: exactly one winner (multi-replica safety).
    store
        .put_login_attempt(&attempt("state-race", 60))
        .await
        .unwrap();
    let (a, b) = tokio::join!(
        store.take_login_attempt("state-race"),
        store.take_login_attempt("state-race")
    );
    assert_eq!([a.unwrap(), b.unwrap()].iter().flatten().count(), 1, "race");
}

async fn handoffs(store: &Store) {
    store.put_handoff(&handoff("hash-1", 60)).await.unwrap();
    assert!(store.take_handoff("hash-1").await.unwrap().is_some());
    assert!(
        store.take_handoff("hash-1").await.unwrap().is_none(),
        "replay"
    );
    store.put_handoff(&handoff("hash-old", -1)).await.unwrap();
    assert!(
        store.take_handoff("hash-old").await.unwrap().is_none(),
        "expired"
    );
    store.put_handoff(&handoff("hash-race", 60)).await.unwrap();
    let (a, b) = tokio::join!(
        store.take_handoff("hash-race"),
        store.take_handoff("hash-race")
    );
    assert_eq!([a.unwrap(), b.unwrap()].iter().flatten().count(), 1, "race");
}

async fn sessions(store: &Store, user_id: Uuid) {
    let by_sid = session(user_id, Some("sid-1"), "sub-1");
    let by_sub = session(user_id, None, "sub-2");
    store.put_session(&by_sid).await.unwrap();
    store.put_session(&by_sub).await.unwrap();

    let sid = SessionSelector::Sid {
        issuer: "https://idp.test".into(),
        sid: "sid-1".into(),
    };
    let hits = store.sessions_for(&sid).await.unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].family_id, by_sid.family_id);

    let sub = SessionSelector::Subject {
        issuer: "https://idp.test".into(),
        subject: "sub-2".into(),
    };
    assert_eq!(store.sessions_for(&sub).await.unwrap().len(), 1);

    let live_until = Utc::now() + Duration::minutes(5);
    store
        .remember_access_jti(by_sub.family_id, "jti-live", live_until)
        .await
        .unwrap();
    store
        .remember_access_jti(
            by_sub.family_id,
            "jti-expired",
            Utc::now() - Duration::seconds(1),
        )
        .await
        .unwrap();
    let live = store.access_jtis(by_sub.family_id).await.unwrap();
    assert_eq!(live.len(), 1, "expired access jti is not mapped");
    assert_eq!(live[0].0, "jti-live");

    store.mark_session_revoked(by_sid.family_id).await.unwrap();
    assert!(
        store.sessions_for(&sid).await.unwrap().is_empty(),
        "revoked sessions are not re-selected"
    );
    let reloaded = store.get_session(by_sid.family_id).await.unwrap().unwrap();
    assert!(reloaded.revoked_at.is_some());
    assert!(store.get_session(Uuid::new_v4()).await.unwrap().is_none());
}

async fn logout_jti(store: &Store) {
    let exp = Utc::now() + Duration::minutes(5);
    assert!(store
        .record_logout_jti("https://idp.test", "jti-1", exp)
        .await
        .unwrap());
    assert!(
        !store
            .record_logout_jti("https://idp.test", "jti-1", exp)
            .await
            .unwrap(),
        "replay"
    );
    assert!(
        store
            .record_logout_jti("https://other.test", "jti-1", exp)
            .await
            .unwrap(),
        "per-issuer"
    );
}

async fn providers(store: &Store) {
    store.upsert_provider(&provider("acme")).await.unwrap();
    let mut changed = provider("acme");
    changed.display_name = "Acme Corp".into();
    store.upsert_provider(&changed).await.unwrap();
    let all = store.list_providers().await.unwrap();
    assert_eq!(
        all.iter().filter(|p| p.slug == "acme").count(),
        1,
        "upsert, not insert"
    );
    assert_eq!(
        all.iter().find(|p| p.slug == "acme").unwrap().display_name,
        "Acme Corp"
    );
    assert_eq!(
        all.iter().find(|p| p.slug == "acme").unwrap().metadata["require_org"],
        true
    );
    assert!(store.delete_provider("acme").await.unwrap());
    assert!(!store.delete_provider("acme").await.unwrap());
}
