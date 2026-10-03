//! SPEC-158 e2e — organization → tenant binding, suspension, tenant CRUD authz (LAW-158-5/6).
//! Requires `--features postgres` (user store is a stub in non-PG builds).
#![cfg(feature = "postgres")]

mod common;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use common::spec158::*;
use edgequake_api::services::federation::policy::FederationPolicy;
use serde_json::{json, Value};
use tower::ServiceExt;

fn org_policy() -> FederationPolicy {
    FederationPolicy {
        require_org: true,
        ..policy()
    }
}

async fn ok_body(res: axum::response::Response) -> Value {
    let status = res.status();
    let body = json_body(res).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body
}

fn tenant_of(body: &Value) -> String {
    jwt_payload(body["access_token"].as_str().unwrap())["tenant_id"]
        .as_str()
        .unwrap()
        .to_string()
}

async fn denied(res: axum::response::Response, code: &str) {
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let body = json_body(res).await.to_string();
    assert!(body.contains(code), "expected {code} in {body}");
}

// EC-158-11 org_tenant_bind: the org claim selects the tenant; the JWT carries it.
#[tokio::test]
async fn org_claim_binds_session_to_mapped_tenant() {
    let sso = Sso::start(org_policy(), None).await;
    let acme = sso.create_tenant("Acme", "acme").await;
    let body = ok_body(
        sso.login(
            "",
            "u1",
            json!({"email":"u1@acme.test","organization":["acme"]}),
        )
        .await,
    )
    .await;
    assert_eq!(tenant_of(&body), acme.tenant_id.to_string());
    let uid = body["user"]["user_id"].as_str().unwrap().parse().unwrap();
    assert!(sso
        .state
        .workspace_service
        .check_tenant_access(uid, acme.tenant_id)
        .await
        .unwrap());
}

#[tokio::test]
async fn keycloak_map_shaped_org_claim_is_understood() {
    let sso = Sso::start(org_policy(), None).await;
    let acme = sso.create_tenant("Acme", "acme").await;
    let body = ok_body(
        sso.login(
            "",
            "u2",
            json!({"email":"u2@acme.test","organization":{"Acme":{"id":"org-1"}}}),
        )
        .await,
    )
    .await;
    assert_eq!(
        tenant_of(&body),
        acme.tenant_id.to_string(),
        "aliases are case-insensitive"
    );
}

// EC-158-12: unknown / missing org is denied — never defaulted.
#[tokio::test]
async fn unknown_org_is_denied() {
    let sso = Sso::start(org_policy(), None).await;
    sso.create_tenant("Acme", "acme").await;
    denied(
        sso.login(
            "",
            "u3",
            json!({"email":"u3@x.test","organization":["ghost"]}),
        )
        .await,
        "org_unknown",
    )
    .await;
}

#[tokio::test]
async fn missing_org_is_denied_when_required() {
    let sso = Sso::start(org_policy(), None).await;
    denied(
        sso.login("", "u4", json!({"email":"u4@x.test"})).await,
        "org_missing",
    )
    .await;
}

// EC-158-13 multi_org_picker: ambiguity is explicit; selection must be asserted by the token.
#[tokio::test]
async fn multi_org_requires_explicit_selection() {
    let sso = Sso::start(org_policy(), None).await;
    sso.create_tenant("Acme", "acme").await;
    let globex = sso.create_tenant("Globex", "globex").await;
    let claims = json!({"email":"m@x.test","organization":["acme","globex"]});

    denied(
        sso.login("", "m1", claims.clone()).await,
        "org_ambiguous:acme,globex",
    )
    .await;
    let body = ok_body(sso.login("org=globex", "m1", claims.clone()).await).await;
    assert_eq!(tenant_of(&body), globex.tenant_id.to_string());
    denied(sso.login("org=initech", "m1", claims).await, "org_unknown").await;
}

#[tokio::test]
async fn fixed_tenant_slug_pins_every_login() {
    let sso = Sso::start(
        FederationPolicy {
            fixed_tenant_slug: Some("acme".into()),
            ..policy()
        },
        None,
    )
    .await;
    let acme = sso.create_tenant("Acme", "acme").await;
    let body = ok_body(sso.login("", "f1", json!({"email":"f@x.test"})).await).await;
    assert_eq!(tenant_of(&body), acme.tenant_id.to_string());
}

// EC-158-27: tenant capacity.
#[tokio::test]
async fn tenant_capacity_blocks_new_members() {
    let sso = Sso::start(org_policy(), None).await;
    let mut acme = sso.create_tenant("Acme", "acme").await;
    acme.max_users = 1;
    sso.state
        .workspace_service
        .update_tenant(acme)
        .await
        .unwrap();
    let claims = |e: &str| json!({"email": e, "organization": ["acme"]});
    ok_body(sso.login("", "c1", claims("c1@x.test")).await).await;
    denied(sso.login("", "c2", claims("c2@x.test")).await, "max_users").await;
    ok_body(sso.login("", "c1", claims("c1@x.test")).await).await; // existing member still signs in
}

// EC-158-29 tenant_suspended: at login and at refresh.
#[tokio::test]
async fn suspended_tenant_blocks_login_and_refresh() {
    let sso = Sso::start(org_policy(), None).await;
    let mut acme = sso.create_tenant("Acme", "acme").await;
    let claims = json!({"email":"s@x.test","organization":["acme"]});
    let body = ok_body(sso.login("", "s1", claims.clone()).await).await;
    let refresh = body["refresh_token"].as_str().unwrap().to_string();

    acme.is_active = false;
    sso.state
        .workspace_service
        .update_tenant(acme)
        .await
        .unwrap();

    denied(sso.login("", "s1", claims).await, "tenant_suspended").await;
    let res = sso
        .post_json(
            "/api/v1/auth/refresh",
            json!({ "refresh_token": refresh }),
            None,
        )
        .await;
    denied(res, "tenant_suspended").await;
}

// Refresh keeps the SSO tenant scope (no silent fallback to the default tenant).
#[tokio::test]
async fn refresh_preserves_tenant_scope() {
    let sso = Sso::start(org_policy(), None).await;
    let acme = sso.create_tenant("Acme", "acme").await;
    let body = ok_body(
        sso.login(
            "",
            "r1",
            json!({"email":"r@x.test","organization":["acme"]}),
        )
        .await,
    )
    .await;
    let refresh = body["refresh_token"].as_str().unwrap();
    let res = sso
        .post_json(
            "/api/v1/auth/refresh",
            json!({ "refresh_token": refresh }),
            None,
        )
        .await;
    let renewed = ok_body(res).await;
    assert_eq!(tenant_of(&renewed), acme.tenant_id.to_string());
}

// EC-158-34: revoked membership stops renewal immediately.
#[tokio::test]
async fn revoked_membership_blocks_refresh() {
    let sso = Sso::start(org_policy(), None).await;
    let acme = sso.create_tenant("Acme", "acme").await;
    let body = ok_body(
        sso.login(
            "",
            "v1",
            json!({"email":"v@x.test","organization":["acme"]}),
        )
        .await,
    )
    .await;
    let uid = body["user"]["user_id"].as_str().unwrap().parse().unwrap();
    for m in sso
        .state
        .workspace_service
        .get_user_memberships(uid)
        .await
        .unwrap()
    {
        sso.state
            .workspace_service
            .remove_membership(m.membership_id)
            .await
            .unwrap();
    }
    let res = sso
        .post_json(
            "/api/v1/auth/refresh",
            json!({ "refresh_token": body["refresh_token"] }),
            None,
        )
        .await;
    denied(res, "membership_revoked").await;
    let _ = acme;
}

// EC-158-14 tenant_crud_authz (fail-first: before SPEC-158 any caller could do all of this).
#[tokio::test]
async fn tenant_crud_requires_authorization() {
    let mut sso = Sso::start(org_policy(), None).await;
    let admin_token = sso.admin_token();
    let acme = sso.create_tenant("Acme", "acme").await;
    let globex = sso.create_tenant("Globex", "globex").await;
    let member = ok_body(
        sso.login(
            "",
            "t1",
            json!({"email":"t@x.test","organization":["acme"]}),
        )
        .await,
    )
    .await;
    let member_token = member["access_token"].as_str().unwrap().to_string();
    sso.enforce_auth();

    let acme_url = format!("/api/v1/tenants/{}", acme.tenant_id);
    let globex_url = format!("/api/v1/tenants/{}", globex.tenant_id);

    // Anonymous.
    assert_eq!(
        sso.get("/api/v1/tenants").await.status(),
        StatusCode::UNAUTHORIZED
    );

    // Plain member: read own tenant only; cannot create / update / delete.
    assert_eq!(
        sso.request("GET", &acme_url, &member_token).await.status(),
        StatusCode::OK
    );
    assert_eq!(
        sso.request("GET", &globex_url, &member_token)
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        sso.request("PUT", &acme_url, &member_token).await.status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        sso.request("DELETE", &acme_url, &member_token)
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        sso.request("DELETE", &globex_url, &member_token)
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    let create = sso
        .post_json(
            "/api/v1/tenants",
            json!({"name":"Evil","slug":"evil"}),
            Some(&member_token),
        )
        .await;
    assert_eq!(create.status(), StatusCode::FORBIDDEN);

    // List is membership-scoped for members, global for platform admins.
    let list = json_body(sso.request("GET", "/api/v1/tenants", &member_token).await).await;
    assert_eq!(list["total"], 1);
    assert_eq!(list["items"][0]["slug"], "acme");
    let all = json_body(sso.request("GET", "/api/v1/tenants", &admin_token).await).await;
    assert!(all["total"].as_u64().unwrap_or(0) >= 2, "{all}");

    // Platform admin can manage.
    assert_eq!(
        sso.request("GET", &globex_url, &admin_token).await.status(),
        StatusCode::OK
    );
}

// EC-158-12: a valid SSO JWT for tenant A must not be retargeted with X-Tenant-ID B.
#[tokio::test]
async fn sso_jwt_rejects_foreign_tenant_header() {
    let mut sso = Sso::start(org_policy(), None).await;
    let acme = sso.create_tenant("Acme", "acme").await;
    let globex = sso.create_tenant("Globex", "globex").await;
    let body = ok_body(
        sso.login(
            "",
            "hdr",
            json!({"email":"hdr@acme.test","organization":["acme"]}),
        )
        .await,
    )
    .await;
    let token = body["access_token"].as_str().unwrap().to_string();
    sso.enforce_auth();

    let acme_url = format!("/api/v1/tenants/{}", acme.tenant_id);
    let call = |tenant: &str| {
        let token = token.clone();
        let url = acme_url.clone();
        let app = sso.app();
        let header_value = tenant.to_string();
        async move {
            app.oneshot(
                Request::get(&url)
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .header("x-tenant-id", header_value)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap()
        }
    };

    assert_eq!(
        call(&acme.tenant_id.to_string()).await.status(),
        StatusCode::OK
    );
    let spoofed = call(&globex.tenant_id.to_string()).await;
    assert_eq!(spoofed.status(), StatusCode::FORBIDDEN);
    assert!(json_body(spoofed)
        .await
        .to_string()
        .contains("does not match"));
}

// EC-158-10 break_glass_password: local admins keep working while SSO is active.
#[tokio::test]
async fn password_login_still_works_with_sso_active() {
    let mut sso = Sso::start(org_policy(), None).await;
    sso.create_local_user("breakglass", "bg@local.test", "SecurePass123!")
        .await;
    sso.enforce_auth();
    let res = sso.password_login("breakglass", "SecurePass123!").await;
    assert_eq!(res.status(), StatusCode::OK);
}
