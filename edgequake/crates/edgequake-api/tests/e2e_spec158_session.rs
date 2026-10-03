//! SPEC-158 e2e — handoff, back-channel logout, JWKS rotation, provider discovery/admin,
//! startup gates (LAW-158-4/7/9/10). Requires `--features postgres`.
#![cfg(feature = "postgres")]

mod common;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use common::rsa_fixtures::*;
use common::spec158::*;
use edgequake_api::services::federation::policy::FederationPolicy;
use edgequake_api::startup_security::StartupSecurityOutcome;
use edgequake_api::startup_sso::{enforce_sso_security, validate_sso_startup, SsoStartupFacts};
use edgequake_auth::AuthConfig;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

const BCL_EVENT: &str = "http://schemas.openid.net/event/backchannel-logout";

async fn handoff_code(
    sso: &Sso,
    subject: &str,
    extra: Value,
) -> (String, axum::response::Response) {
    let res = sso.login("", subject, extra).await;
    assert_eq!(res.status(), StatusCode::SEE_OTHER);
    (query_param(&location(&res), "code").expect("code"), res)
}

// EC-158-04 handoff_no_token_url: no credential ever appears in the redirect URL.
#[tokio::test]
async fn handoff_redirect_carries_only_an_opaque_code() {
    let sso = Sso::start(policy(), Some(SUCCESS_URL)).await;
    let (code, res) = handoff_code(&sso, "h1", json!({"email":"h@x.test"})).await;
    let loc = location(&res);
    assert!(loc.starts_with(SUCCESS_URL));
    for forbidden in ["access_token", "refresh_token", "id_token", "eyJ"] {
        assert!(!loc.contains(forbidden), "{forbidden} leaked into {loc}");
    }
    let cookie = set_cookie(&res).expect("refresh cookie");
    assert!(cookie.contains("HttpOnly"), "{cookie}");

    let redeemed = sso
        .post_json("/api/v1/auth/handoff", json!({ "code": code }), None)
        .await;
    assert_eq!(redeemed.status(), StatusCode::OK);
    let body = json_body(redeemed).await;
    assert_eq!(body["token_type"], "Bearer");
    assert!(body["access_token"].as_str().unwrap().starts_with("eyJ"));
    assert!(
        body.get("refresh_token").is_none(),
        "refresh token must stay in the cookie"
    );
    assert_eq!(body["user"]["email"], "h@x.test");
}

#[tokio::test]
async fn handoff_code_is_single_use_and_garbage_rejected() {
    let sso = Sso::start(policy(), Some(SUCCESS_URL)).await;
    let (code, _) = handoff_code(&sso, "h2", json!({"email":"h2@x.test"})).await;
    let first = sso
        .post_json("/api/v1/auth/handoff", json!({ "code": &code }), None)
        .await;
    assert_eq!(first.status(), StatusCode::OK);
    let replay = sso
        .post_json("/api/v1/auth/handoff", json!({ "code": &code }), None)
        .await;
    assert_eq!(replay.status(), StatusCode::UNAUTHORIZED);
    for bad in ["", "nope", &"x".repeat(1000)] {
        let res = sso
            .post_json("/api/v1/auth/handoff", json!({ "code": bad }), None)
            .await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }
}

// EC-158-22: the token the SPA actually holds (handoff redeem) is denylisted too.
#[tokio::test]
async fn backchannel_logout_denylists_handoff_access_token() {
    let mut sso = Sso::start(policy(), Some(SUCCESS_URL)).await;
    let (code, _) = handoff_code(
        &sso,
        "h-bc",
        json!({"email":"hbc@x.test","sid":"sid-handoff"}),
    )
    .await;
    let redeemed = sso
        .post_json("/api/v1/auth/handoff", json!({ "code": code }), None)
        .await;
    assert_eq!(redeemed.status(), StatusCode::OK);
    let access = json_body(redeemed).await["access_token"]
        .as_str()
        .unwrap()
        .to_string();
    sso.enforce_auth();
    assert_eq!(
        sso.request("GET", "/api/v1/auth/me", &access)
            .await
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        post_logout(&sso, &logout_token(&sso, json!({"sid":"sid-handoff"})))
            .await
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        sso.request("GET", "/api/v1/auth/me", &access)
            .await
            .status(),
        StatusCode::UNAUTHORIZED,
        "handoff access jti must be denylisted"
    );
}

#[tokio::test]
async fn handoff_is_reachable_without_auth_when_auth_enforced() {
    let mut sso = Sso::start(policy(), Some(SUCCESS_URL)).await;
    sso.enforce_auth();
    let (code, _) = handoff_code(&sso, "h3", json!({"email":"h3@x.test"})).await;
    let res = sso
        .post_json("/api/v1/auth/handoff", json!({ "code": code }), None)
        .await;
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn failed_login_redirects_with_machine_code_only() {
    let p = FederationPolicy {
        require_org: true,
        ..policy()
    };
    let sso = Sso::start(p, Some(SUCCESS_URL)).await;
    let res = sso.login("", "h4", json!({"email":"h4@x.test"})).await;
    assert_eq!(res.status(), StatusCode::SEE_OTHER);
    let loc = location(&res);
    assert_eq!(query_param(&loc, "error").as_deref(), Some("org_missing"));
    assert!(query_param(&loc, "code").is_none());
}

// ── back-channel logout ──

fn logout_token(sso: &Sso, extra: Value) -> String {
    let now = chrono::Utc::now().timestamp();
    let mut claims = json!({
        "iss": sso.mock.uri(), "aud": CLIENT_ID, "iat": now, "exp": now + 120,
        "jti": uuid::Uuid::new_v4().to_string(), "events": { BCL_EVENT: {} },
    });
    claims
        .as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(CLIENT_SECRET.as_bytes()),
    )
    .unwrap()
}

async fn post_logout(sso: &Sso, token: &str) -> axum::response::Response {
    sso.app()
        .oneshot(
            Request::post("/api/v1/auth/oidc/backchannel-logout")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(format!(
                    "logout_token={}",
                    urlencoding::encode(token)
                )))
                .unwrap(),
        )
        .await
        .unwrap()
}

// EC-158-30 backchannel_logout.
#[tokio::test]
async fn backchannel_logout_revokes_sso_session() {
    let mut sso = Sso::start(policy(), None).await;
    let login = sso
        .login("", "bc1", json!({"email":"bc@x.test","sid":"sid-1"}))
        .await;
    let body = json_body(login).await;
    let refresh = body["refresh_token"].as_str().unwrap().to_string();
    let access = body["access_token"].as_str().unwrap().to_string();
    sso.enforce_auth();
    assert_eq!(
        sso.request("GET", "/api/v1/auth/me", &access)
            .await
            .status(),
        StatusCode::OK,
        "mapped access token works before logout"
    );

    let res = post_logout(&sso, &logout_token(&sso, json!({"sid":"sid-1"}))).await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers().get(header::CACHE_CONTROL).unwrap(),
        "no-store"
    );

    let renew = sso
        .post_json(
            "/api/v1/auth/refresh",
            json!({ "refresh_token": refresh }),
            None,
        )
        .await;
    assert_eq!(
        renew.status(),
        StatusCode::UNAUTHORIZED,
        "revoked family must not refresh"
    );
    assert_eq!(
        sso.request("GET", "/api/v1/auth/me", &access)
            .await
            .status(),
        StatusCode::UNAUTHORIZED,
        "mapped access jti must be denylisted"
    );
}

#[tokio::test]
async fn backchannel_logout_by_subject_and_unknown_sid_are_ok() {
    let sso = Sso::start(policy(), None).await;
    sso.login("", "bc2", json!({"email":"bc2@x.test"})).await;
    assert_eq!(
        post_logout(&sso, &logout_token(&sso, json!({"sub":"bc2"})))
            .await
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        post_logout(&sso, &logout_token(&sso, json!({"sid":"never-seen"})))
            .await
            .status(),
        StatusCode::OK
    );
}

#[tokio::test]
async fn backchannel_logout_rejects_replay_and_malformed_tokens() {
    let sso = Sso::start(policy(), None).await;
    let token = logout_token(&sso, json!({"sid":"sid-r"}));
    assert_eq!(post_logout(&sso, &token).await.status(), StatusCode::OK);
    assert_eq!(
        post_logout(&sso, &token).await.status(),
        StatusCode::BAD_REQUEST,
        "jti replay"
    );

    let with_nonce = logout_token(&sso, json!({"sid":"s","nonce":"n"}));
    assert_eq!(
        post_logout(&sso, &with_nonce).await.status(),
        StatusCode::BAD_REQUEST
    );
    let wrong_aud = logout_token(&sso, json!({"sid":"s","aud":"someone-else"}));
    assert_eq!(
        post_logout(&sso, &wrong_aud).await.status(),
        StatusCode::BAD_REQUEST
    );
    let no_subject = logout_token(&sso, json!({}));
    assert_eq!(
        post_logout(&sso, &no_subject).await.status(),
        StatusCode::BAD_REQUEST
    );
    let unknown_issuer = logout_token(&sso, json!({"sid":"s","iss":"https://evil.test"}));
    assert_eq!(
        post_logout(&sso, &unknown_issuer).await.status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        post_logout(&sso, "not-a-jwt").await.status(),
        StatusCode::BAD_REQUEST
    );

    let forged = encode(
        &Header::new(Algorithm::HS256),
        &json!({"iss": sso.mock.uri(), "aud": CLIENT_ID, "exp": chrono::Utc::now().timestamp() + 60,
                "jti": "j", "sid": "s", "events": { BCL_EVENT: {} }}),
        &EncodingKey::from_secret(b"wrong-secret"),
    )
    .unwrap();
    assert_eq!(
        post_logout(&sso, &forged).await.status(),
        StatusCode::BAD_REQUEST,
        "bad signature"
    );
}

// ── JWKS rotation (EC-158-15) ──

fn jwk(kid: &str, n: &str) -> Value {
    json!({"kty":"RSA","use":"sig","alg":"RS256","kid":kid,"n":n,"e":RSA_E})
}

#[tokio::test]
async fn jwks_rotation_between_authorize_and_callback_is_survived() {
    let sso = Sso::start_with_alg(
        policy(),
        None,
        "RS256",
        json!({"keys":[jwk(KEY_A_KID, KEY_A_N)]}),
    )
    .await;

    let (st, nonce) = sso.begin("").await;
    sso.arm(sign_rs256(
        KEY_A_KID,
        KEY_A_PEM,
        &sso.mock.uri(),
        &nonce,
        "rk1",
        json!({"email":"rk@x.test"}),
    ));
    assert_eq!(
        sso.callback(&st).await.status(),
        StatusCode::OK,
        "baseline with key A"
    );

    // IdP rotates: only key B is published now; the cached JWKS still has A.
    *sso.jwks.0.lock().unwrap() = json!({"keys":[jwk(KEY_B_KID, KEY_B_N)]});
    let (st, nonce) = sso.begin("").await;
    sso.arm(sign_rs256(
        KEY_B_KID,
        KEY_B_PEM,
        &sso.mock.uri(),
        &nonce,
        "rk1",
        json!({"email":"rk@x.test"}),
    ));
    assert_eq!(
        sso.callback(&st).await.status(),
        StatusCode::OK,
        "forced JWKS refresh must pick up key B"
    );
}

#[tokio::test]
async fn token_signed_by_unpublished_key_is_rejected() {
    let sso = Sso::start_with_alg(
        policy(),
        None,
        "RS256",
        json!({"keys":[jwk(KEY_A_KID, KEY_A_N)]}),
    )
    .await;
    let (st, nonce) = sso.begin("").await;
    sso.arm(sign_rs256(
        KEY_B_KID,
        KEY_B_PEM,
        &sso.mock.uri(),
        &nonce,
        "rk2",
        json!({"email":"x@x.test"}),
    ));
    assert_eq!(sso.callback(&st).await.status(), StatusCode::UNAUTHORIZED);
}

// ── provider discovery + admin ──

#[tokio::test]
async fn provider_list_is_public_and_secret_free() {
    let mut sso = Sso::start(policy(), None).await;
    sso.enforce_auth();
    let res = sso.get("/api/v1/auth/sso/providers").await;
    assert_eq!(res.status(), StatusCode::OK);
    let text = json_body(res).await;
    let s = text.to_string();
    assert!(
        !s.contains(CLIENT_SECRET) && !s.contains("client_secret"),
        "{s}"
    );
    assert_eq!(text["providers"][0]["slug"], "idp");
    assert_eq!(
        text["providers"][0]["login_path"],
        "/api/v1/auth/oidc/login?provider=idp"
    );
}

fn stored(issuer: &str) -> Value {
    json!({
        "slug": "ignored", "kind": "keycloak", "display_name": "Acme SSO", "issuer": issuer,
        "client_id": "c", "client_secret_ref": "ACME_SECRET", "redirect_uri": "https://eq.test/api/v1/auth/oidc/callback",
        "scopes": [], "trust_email": false, "link_policy": "never", "jit_enabled": true,
        "role_claim": "", "role_map": {}, "max_role": "admin", "enabled": true, "metadata": {}
    })
}

async fn admin_call(
    sso: &Sso,
    method: &str,
    uri: &str,
    token: &str,
    body: Value,
) -> axum::response::Response {
    sso.app()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
}

#[tokio::test]
async fn identity_provider_admin_requires_admin_and_validates() {
    let mut sso = Sso::start(policy(), None).await;
    let member = sso
        .state
        .auth
        .jwt
        .generate_token(uuid::Uuid::new_v4(), edgequake_auth::Role::User)
        .unwrap();
    let admin = sso.admin_token();
    sso.enforce_auth();
    let uri = "/api/v1/admin/identity-providers/Acme";

    assert_eq!(
        admin_call(
            &sso,
            "PUT",
            uri,
            &member,
            stored("https://kc.test/realms/a")
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        sso.get("/api/v1/admin/identity-providers").await.status(),
        StatusCode::UNAUTHORIZED
    );
    // http issuer is rejected (LAW-158-3: no cleartext trust anchors).
    assert_eq!(
        admin_call(&sso, "PUT", uri, &admin, stored("http://kc.test/realms/a"))
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );
    let mut bad_policy = stored("https://kc.test/realms/a");
    bad_policy["link_policy"] = json!("always");
    assert_eq!(
        admin_call(&sso, "PUT", uri, &admin, bad_policy)
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        admin_call(
            &sso,
            "DELETE",
            "/api/v1/admin/identity-providers/ghost",
            &admin,
            json!({})
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );
}

// ── startup gates (EC-158-21) ──

#[test]
fn startup_gates_are_fail_closed_for_sso() {
    let mut auth = AuthConfig::new("secure-test-secret-spec158-long-enough-xx");
    auth.auth_enabled = false;
    auth.dev_mode = false;
    let facts = SsoStartupFacts {
        redirect_uris: vec!["https://eq.test/cb".into()],
    };
    let security = enforce_sso_security(Default::default(), &facts, false);
    assert!(security.strict_tenant_bind, "SSO forces tenant bind");
    assert!(matches!(
        validate_sso_startup(&facts, &auth, &security),
        StartupSecurityOutcome::Fatal(_)
    ));
    auth.auth_enabled = true;
    assert_eq!(
        validate_sso_startup(&facts, &auth, &security),
        StartupSecurityOutcome::Ok
    );
}

#[tokio::test]
async fn runtime_facts_reflect_configured_providers() {
    let sso = Sso::start(policy(), None).await;
    let facts = sso.state.auth.startup_facts();
    assert!(facts.is_active());
    assert_eq!(
        facts.redirect_uris,
        vec!["http://localhost/api/v1/auth/oidc/callback".to_string()]
    );
}

async fn pg_pool() -> Option<sqlx::PgPool> {
    let url = std::env::var("DATABASE_URL")
        .ok()
        .filter(|u| !u.is_empty())?;
    std::env::set_var("EDGEQUAKE_MIGRATE_CLI", "1");
    let pool = sqlx::PgPool::connect(&url).await.ok()?;
    edgequake_api::state::migration_bootstrap::run_postgres_migrations(&pool)
        .await
        .ok()?;
    Some(pool)
}

fn bind_pg_federation(state: &mut edgequake_api::AppState, pool: sqlx::PgPool) {
    use std::sync::Arc;
    state.pg_pool = Some(pool.clone());
    state.auth.federation =
        Arc::new(edgequake_api::services::federation::pg_store::PgFederationStore::new(pool));
    state.security.kv_identity_mirror = false;
}

// EC-158-24: pending login lives in Postgres so replica B can finish what replica A started.
#[tokio::test]
async fn pending_login_survives_replica_handoff() {
    let Some(pool) = pg_pool().await else {
        eprintln!("SKIP: no DATABASE_URL");
        return;
    };
    let mut sso = Sso::start(policy(), None).await;
    bind_pg_federation(&mut sso.state, pool.clone());
    sso.state
        .initialize_defaults()
        .await
        .expect("seed default tenant/user");
    let (st, nonce) = sso.begin("").await;
    sso.arm(sso.id_token(
        &nonce,
        &format!("rep-b-{}", Uuid::new_v4()),
        json!({"email": format!("rep-b-{}@corp.test", Uuid::new_v4())}),
    ));

    let mut replica_b = sso.state.clone();
    replica_b.auth.jwt = std::sync::Arc::new(edgequake_auth::JwtService::new(
        replica_b.auth.config.clone(),
    ));
    let res = Sso::router(replica_b)
        .oneshot(
            Request::get(format!(
                "/api/v1/auth/oidc/callback?code=c&state={}",
                urlencoding::encode(&st)
            ))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    let status = res.status();
    let body = json_body(res).await;
    assert_eq!(status, StatusCode::OK, "callback on replica B: {body}");

    let mut empty = sso.state.clone();
    empty.auth.federation =
        std::sync::Arc::new(edgequake_api::services::federation::MemoryFederationStore::new());
    let miss = Sso::router(empty)
        .oneshot(
            Request::get(format!(
                "/api/v1/auth/oidc/callback?code=c&state={}",
                urlencoding::encode(&st)
            ))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(miss.status(), StatusCode::UNAUTHORIZED);
    let body = json_body(miss).await.to_string();
    assert!(
        body.contains("state_expired")
            || body.contains("state_mismatch")
            || body.contains("unauthorized"),
        "{body}"
    );
}

// EC-158-22: logout on A denylists access jti for a fresh JwtService on B.
#[tokio::test]
async fn access_jti_denylist_is_durable_across_processes() {
    let Some(pool) = pg_pool().await else {
        eprintln!("SKIP: no DATABASE_URL");
        return;
    };
    let mut sso = Sso::start(policy(), None).await;
    bind_pg_federation(&mut sso.state, pool.clone());
    sso.state
        .initialize_defaults()
        .await
        .expect("seed default tenant/user");
    let sid = format!("sid-x-{}", Uuid::new_v4());
    let login = sso
        .login(
            "",
            &format!("jti-x-{}", Uuid::new_v4()),
            json!({"email": format!("jti-x-{}@corp.test", Uuid::new_v4()), "sid": sid}),
        )
        .await;
    assert_eq!(login.status(), StatusCode::OK);
    let access = json_body(login).await["access_token"]
        .as_str()
        .unwrap()
        .to_string();
    sso.enforce_auth();
    assert_eq!(
        post_logout(&sso, &logout_token(&sso, json!({"sid": sid})))
            .await
            .status(),
        StatusCode::OK
    );

    let mut replica_b = sso.state.clone();
    replica_b.auth.jwt = std::sync::Arc::new(edgequake_auth::JwtService::new(
        replica_b.auth.config.clone(),
    ));
    replica_b.auth.config.auth_enabled = true;
    replica_b.auth.config.dev_mode = false;
    let me = Sso::router(replica_b)
        .oneshot(
            Request::get("/api/v1/auth/me")
                .header(header::AUTHORIZATION, format!("Bearer {access}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        me.status(),
        StatusCode::UNAUTHORIZED,
        "empty in-process denylist must still read jwt_jti_denylist"
    );
}
