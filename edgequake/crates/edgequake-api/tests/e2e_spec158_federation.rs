//! SPEC-158 e2e — federation identity, linking, provisioning (LAW-158-2/3/8/12).

//! Requires `--features postgres`: the user store is a stub in non-PG builds.
#![cfg(feature = "postgres")]

mod common;

use axum::http::StatusCode;
use common::spec158::*;
use edgequake_api::services::federation::policy::{FederationPolicy, LinkPolicy};
use edgequake_core::MembershipRole;
use serde_json::json;
use uuid::Uuid;

async fn user_id_of(res: axum::response::Response) -> String {
    assert_eq!(res.status(), StatusCode::OK);
    json_body(res).await["user"]["user_id"]
        .as_str()
        .unwrap()
        .to_string()
}

fn trusting() -> FederationPolicy {
    FederationPolicy {
        trust_email: true,
        link_policy: LinkPolicy::VerifiedEmail,
        ..policy()
    }
}

// EC-158-01 federation_key: identity is (issuer, sub); email changes never fork the account.
#[tokio::test]
async fn federation_key_is_issuer_and_subject() {
    let sso = Sso::start(policy(), None).await;
    let first = user_id_of(sso.login("", "sub-1", json!({"email":"a@corp.test"})).await).await;
    let again = user_id_of(
        sso.login("", "sub-1", json!({"email":"renamed@corp.test"}))
            .await,
    )
    .await;
    assert_eq!(
        first, again,
        "same (iss,sub) must map to the same user after an email change"
    );

    let rec = sso
        .state
        .auth
        .federation
        .find_identity(&sso.mock.uri(), "sub-1")
        .await
        .unwrap();
    assert_eq!(rec.expect("identity row").user_id.to_string(), first);
}

// EC-158-02 verified_email_link: untrusted/unverified email must NOT take over a local account.
#[tokio::test]
async fn unverified_email_never_links_local_account() {
    let sso = Sso::start(policy(), None).await;
    sso.create_local_user("alice", "alice@corp.test", "SecurePass123!")
        .await;
    let res = sso
        .login(
            "",
            "evil-sub",
            json!({"email":"alice@corp.test","email_verified":true}),
        )
        .await;
    assert_eq!(
        res.status(),
        StatusCode::CONFLICT,
        "default policy never links by email"
    );
    assert!(json_body(res)
        .await
        .to_string()
        .contains("account_exists_unlinked"));
}

#[tokio::test]
async fn verified_email_links_only_when_provider_trusted_and_verified() {
    let sso = Sso::start(trusting(), None).await;
    let local = sso
        .create_local_user("bob", "bob@corp.test", "SecurePass123!")
        .await;

    let unverified = sso
        .login(
            "",
            "b-1",
            json!({"email":"bob@corp.test","email_verified":false}),
        )
        .await;
    assert_eq!(
        unverified.status(),
        StatusCode::CONFLICT,
        "email_verified=false must not link"
    );

    let linked = user_id_of(
        sso.login(
            "",
            "b-1",
            json!({"email":"bob@corp.test","email_verified":true}),
        )
        .await,
    )
    .await;
    assert_eq!(
        linked, local,
        "verified + trusted ⇒ link to the existing user"
    );
}

// EC-158-03: a provider that is not trusted never links even with email_verified=true.
#[tokio::test]
async fn trust_email_off_blocks_link_even_when_link_policy_allows() {
    let p = FederationPolicy {
        trust_email: false,
        link_policy: LinkPolicy::VerifiedEmail,
        ..policy()
    };
    let sso = Sso::start(p, None).await;
    sso.create_local_user("carol", "carol@corp.test", "SecurePass123!")
        .await;
    let res = sso
        .login(
            "",
            "c-1",
            json!({"email":"carol@corp.test","email_verified":true}),
        )
        .await;
    assert_eq!(res.status(), StatusCode::CONFLICT);
}

// EC-158-03: an IdP that omits email must not collide with a real mailbox.
#[tokio::test]
async fn omitted_email_does_not_match_local_account() {
    let sso = Sso::start(policy(), None).await;
    let local = sso
        .create_local_user("alice", "a@corp.test", "SecurePass123!")
        .await;
    let created_res = sso.login("", "no-mail", json!({})).await;
    let created_body = json_body(created_res).await;
    let created = created_body["user"]["user_id"].as_str().unwrap();
    assert_ne!(created, local);
    let email = created_body["user"]["email"].as_str().unwrap_or_default();
    assert!(
        email.ends_with(".sso.invalid"),
        "synthetic mailbox, not a@corp.test: {email}"
    );
    let again = user_id_of(sso.login("", "no-mail-2", json!({})).await).await;
    assert_ne!(again, local);
}

// EC-158-16: discovery failure sends the SPA `sso_unavailable`; password still works.
#[tokio::test]
async fn unreachable_idp_redirects_and_password_still_works() {
    let mut sso = Sso::start(policy(), Some(SUCCESS_URL)).await;
    sso.create_local_user("breakglass", "bg@local.test", "SecurePass123!")
        .await;
    sso.with_dead_issuer();
    let res = sso.get("/api/v1/auth/oidc/login").await;
    assert_eq!(res.status(), StatusCode::SEE_OTHER);
    let loc = location(&res);
    assert!(
        loc.contains("error=sso_unavailable"),
        "browser must land on the SPA with a machine code: {loc}"
    );
    sso.enforce_auth();
    assert_eq!(
        sso.password_login("breakglass", "SecurePass123!")
            .await
            .status(),
        StatusCode::OK
    );
}

// EC-158-18 jit_membership: provisioning creates exactly one membership in the default tenant.
#[tokio::test]
async fn jit_provisions_user_and_single_sso_membership() {
    let sso = Sso::start(policy(), None).await;
    let uid: Uuid = user_id_of(sso.login("", "j-1", json!({"email":"j@corp.test"})).await)
        .await
        .parse()
        .unwrap();
    sso.login("", "j-1", json!({"email":"j@corp.test"})).await;
    let memberships = sso
        .state
        .workspace_service
        .get_user_memberships(uid)
        .await
        .unwrap();
    assert_eq!(memberships.len(), 1, "membership uniqueness (EC-158-19)");
    assert_eq!(
        memberships[0]
            .metadata
            .get("source")
            .and_then(|v| v.as_str()),
        Some("sso")
    );
}

#[tokio::test]
async fn jit_disabled_denies_unknown_users() {
    let sso = Sso::start(
        FederationPolicy {
            jit_enabled: false,
            ..policy()
        },
        None,
    )
    .await;
    let res = sso
        .login("", "nope", json!({"email":"nope@corp.test"}))
        .await;
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    assert!(json_body(res).await.to_string().contains("jit_disabled"));
}

// EC-158-20 role mapping is capped; IdP can never mint owner unless explicitly allowed.
#[tokio::test]
async fn idp_roles_map_and_are_capped() {
    let mut p = policy();
    p.role_claim = "roles".into();
    p.role_map.insert("eq-owner".into(), MembershipRole::Owner);
    p.role_map.insert("eq-admin".into(), MembershipRole::Admin);
    p.max_role = MembershipRole::Admin;
    let sso = Sso::start(p, None).await;
    let uid: Uuid = user_id_of(
        sso.login(
            "",
            "r-1",
            json!({"email":"r@corp.test","roles":["eq-owner"]}),
        )
        .await,
    )
    .await
    .parse()
    .unwrap();
    let m = sso
        .state
        .workspace_service
        .get_user_memberships(uid)
        .await
        .unwrap();
    assert_eq!(
        m[0].role,
        MembershipRole::Admin,
        "Owner request capped at max_role"
    );
}

// EC-158-22 role_resync: a downgrade at the IdP applies on the next login.
#[tokio::test]
async fn role_downgrade_resyncs_on_next_login() {
    let mut p = policy();
    p.role_claim = "roles".into();
    p.role_map.insert("eq-admin".into(), MembershipRole::Admin);
    let sso = Sso::start(p, None).await;
    let uid: Uuid = user_id_of(
        sso.login(
            "",
            "d-1",
            json!({"email":"d@corp.test","roles":["eq-admin"]}),
        )
        .await,
    )
    .await
    .parse()
    .unwrap();
    assert_eq!(
        sso.state
            .workspace_service
            .get_user_memberships(uid)
            .await
            .unwrap()[0]
            .role,
        MembershipRole::Admin
    );
    sso.login("", "d-1", json!({"email":"d@corp.test","roles":[]}))
        .await;
    let role = sso
        .state
        .workspace_service
        .get_user_memberships(uid)
        .await
        .unwrap()[0]
        .role;
    assert_eq!(
        role,
        MembershipRole::Member,
        "no mapped role ⇒ default_role"
    );
}

// EC-158-23 last_owner_guard: sole owner is never demoted by an IdP resync.
#[tokio::test]
async fn last_owner_is_not_demoted_by_resync() {
    let mut p = policy();
    p.role_claim = "roles".into();
    p.role_map.insert("eq-owner".into(), MembershipRole::Owner);
    p.max_role = MembershipRole::Owner;
    let sso = Sso::start(p, None).await;
    let uid: Uuid = user_id_of(
        sso.login(
            "",
            "o-1",
            json!({"email":"o@corp.test","roles":["eq-owner"]}),
        )
        .await,
    )
    .await
    .parse()
    .unwrap();
    sso.login("", "o-1", json!({"email":"o@corp.test","roles":[]}))
        .await;
    let role = sso
        .state
        .workspace_service
        .get_user_memberships(uid)
        .await
        .unwrap()[0]
        .role;
    assert_eq!(role, MembershipRole::Owner, "last owner must be preserved");
}

// EC-158-17 google_hd: hosted-domain allow-list.
#[tokio::test]
async fn google_hosted_domain_allow_list() {
    let p = FederationPolicy {
        allowed_hosted_domains: vec!["corp.test".into()],
        ..policy()
    };
    let sso = Sso::start(p, None).await;
    let ok = sso
        .login("", "g-1", json!({"email":"g@corp.test","hd":"corp.test"}))
        .await;
    assert_eq!(ok.status(), StatusCode::OK);
    let bad = sso
        .login("", "g-2", json!({"email":"g@gmail.com","hd":"gmail.com"}))
        .await;
    assert_eq!(bad.status(), StatusCode::FORBIDDEN);
    let missing = sso.login("", "g-3", json!({"email":"g3@gmail.com"})).await;
    assert_eq!(
        missing.status(),
        StatusCode::FORBIDDEN,
        "no hd claim must not bypass the allow-list"
    );
    let body = json_body(missing).await.to_string();
    assert!(body.contains("hd_mismatch"), "{body}");
}

// EC-158-16 entra_guest: only listed Entra tenants (tid) may sign in.
#[tokio::test]
async fn entra_tenant_allow_list() {
    let p = FederationPolicy {
        allowed_idp_tenant_ids: vec!["tid-home".into()],
        ..policy()
    };
    let sso = Sso::start(p, None).await;
    assert_eq!(
        sso.login("", "e-1", json!({"email":"e@corp.test","tid":"tid-home"}))
            .await
            .status(),
        StatusCode::OK
    );
    let guest = sso
        .login(
            "",
            "e-2",
            json!({"email":"e2@guest.test","tid":"tid-guest"}),
        )
        .await;
    assert_eq!(guest.status(), StatusCode::FORBIDDEN);
    let body = json_body(guest).await.to_string();
    assert!(body.contains("idp_tenant_not_allowed"), "{body}");
    let create = sso
        .post_json(
            "/api/v1/users",
            json!({
                "username": "e2-guest-local",
                "email": "e2@guest.test",
                "password": "SecurePass123!"
            }),
            None,
        )
        .await;
    assert_eq!(
        create.status(),
        StatusCode::CREATED,
        "denied guest must not leave a user row"
    );
}

// EC-158-26 username collisions get a numeric suffix instead of failing or merging.
#[tokio::test]
async fn username_collision_gets_suffix() {
    let sso = Sso::start(policy(), None).await;
    sso.create_local_user("dave", "dave-local@corp.test", "SecurePass123!")
        .await;
    let res = sso
        .login(
            "",
            "dv-1",
            json!({"email":"dave@other.test","preferred_username":"dave"}),
        )
        .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(json_body(res).await["user"]["username"], "dave-2");
}

// EC-158-24 state_replay: a login attempt is single-use.
#[tokio::test]
async fn state_is_single_use() {
    let sso = Sso::start(policy(), None).await;
    let (st, nonce) = sso.begin("").await;
    sso.arm(sso.id_token(&nonce, "s-1", json!({"email":"s@corp.test"})));
    assert_eq!(sso.callback(&st).await.status(), StatusCode::OK);
    assert_eq!(
        sso.callback(&st).await.status(),
        StatusCode::UNAUTHORIZED,
        "replayed state"
    );
}

#[tokio::test]
async fn nonce_mismatch_is_rejected() {
    let sso = Sso::start(policy(), None).await;
    let (st, _nonce) = sso.begin("").await;
    sso.arm(sso.id_token("attacker-nonce", "n-1", json!({"email":"n@corp.test"})));
    assert_eq!(sso.callback(&st).await.status(), StatusCode::UNAUTHORIZED);
}

// EC-158-25 open_redirect: only same-origin paths survive.
#[tokio::test]
async fn open_redirect_targets_are_neutralised() {
    let sso = Sso::start(policy(), Some(SUCCESS_URL)).await;
    for evil in [
        "https://evil.test/x",
        "//evil.test",
        "/\\evil.test",
        "javascript:alert(1)",
    ] {
        let (st, nonce) = sso
            .begin(&format!("redirect={}", urlencoding::encode(evil)))
            .await;
        sso.arm(sso.id_token(&nonce, "o-1", json!({"email":"o@corp.test"})));
        let res = sso.callback(&st).await;
        assert_eq!(res.status(), StatusCode::SEE_OTHER);
        let code = query_param(&location(&res), "code").expect("handoff code");
        let redeemed = sso
            .post_json("/api/v1/auth/handoff", json!({ "code": code }), None)
            .await;
        assert_eq!(redeemed.status(), StatusCode::OK);
        let body = json_body(redeemed).await;
        assert!(
            body["redirect_after"].is_null(),
            "unsafe redirect {evil:?} must be dropped"
        );
    }
}

#[tokio::test]
async fn safe_same_origin_redirect_is_preserved() {
    let sso = Sso::start(policy(), Some(SUCCESS_URL)).await;
    let (st, nonce) = sso.begin("redirect=%2Fdocuments%3Ftab%3D1").await;
    sso.arm(sso.id_token(&nonce, "p-1", json!({"email":"p@corp.test"})));
    let code = query_param(&location(&sso.callback(&st).await), "code").expect("code");
    let body = json_body(
        sso.post_json("/api/v1/auth/handoff", json!({ "code": code }), None)
            .await,
    )
    .await;
    assert_eq!(body["redirect_after"], "/documents?tab=1");
}

// EC-158-25 github_broker: GitHub emails are unverified and Trust Email stays off,
// so a brokered login must not take over a local account with the same address.
#[tokio::test]
async fn github_broker_unverified_email_does_not_link() {
    let sso = Sso::start(policy(), None).await;
    let local = sso
        .create_local_user("gh-local", "dev@corp.test", "SecurePass123!")
        .await;
    let res = sso
        .login(
            "",
            "github|48213",
            json!({"email":"dev@corp.test","email_verified":false}),
        )
        .await;
    assert_eq!(res.status(), StatusCode::CONFLICT);
    assert!(json_body(res)
        .await
        .to_string()
        .contains("account_exists_unlinked"));

    let created = user_id_of(
        sso.login(
            "",
            "github|99901",
            json!({"email":"new-gh@users.noreply.github.com","email_verified":false}),
        )
        .await,
    )
    .await;
    assert_ne!(created, local, "JIT user is a distinct principal");
}

// EC-158-17 id_token_leeway: 30s of exp/iat skew is accepted; more is rejected.
#[tokio::test]
async fn id_token_clock_skew_within_30s_is_accepted() {
    let sso = Sso::start(policy(), None).await;
    let now = chrono::Utc::now().timestamp();
    assert_eq!(
        sso.login("", "skew-exp", json!({"email":"e@x.test","exp": now - 15}))
            .await
            .status(),
        StatusCode::OK,
        "exp 15s in the past is inside the leeway"
    );
    assert_eq!(
        sso.login("", "skew-iat", json!({"email":"i@x.test","iat": now + 15}))
            .await
            .status(),
        StatusCode::OK,
        "iat 15s in the future is inside the leeway"
    );
}

#[tokio::test]
async fn id_token_clock_skew_beyond_30s_is_rejected() {
    let sso = Sso::start(policy(), None).await;
    let now = chrono::Utc::now().timestamp();
    assert_eq!(
        sso.login(
            "",
            "skew-exp-far",
            json!({"email":"e2@x.test","exp": now - 60})
        )
        .await
        .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        sso.login(
            "",
            "skew-iat-far",
            json!({"email":"i2@x.test","iat": now + 90})
        )
        .await
        .status(),
        StatusCode::UNAUTHORIZED
    );
}
