//! SPEC-158 — SSO principal on SPEC-154 surfaces (MCP audience, API-key scopes, WebSocket).
#![cfg(feature = "postgres")]

mod common;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use common::spec028_mcp::{mcp_tools_call_bearer, tools_call_body};
use common::spec158::*;
use serde_json::json;
use sha2::{Digest, Sha256};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tower::ServiceExt;

const MCP_RESOURCE: &str = "http://127.0.0.1:8080/mcp";
const REDIRECT_URI: &str = "http://127.0.0.1:54321/callback";

async fn sso_access() -> (Sso, String) {
    let mut sso = Sso::start(policy(), None).await;
    sso.enforce_auth();
    let login = sso
        .login("", "surf-1", json!({"email":"surf@corp.test"}))
        .await;
    assert_eq!(login.status(), StatusCode::OK);
    let access = json_body(login).await["access_token"]
        .as_str()
        .unwrap()
        .to_string();
    (sso, access)
}

async fn mcp_token_for(sso: &Sso, bearer: &str) -> String {
    let register = sso
        .app()
        .oneshot(
            Request::post("/oauth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "redirect_uris": [REDIRECT_URI],
                        "client_name": "spec158-mcp",
                        "token_endpoint_auth_method": "none"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(register.status(), StatusCode::CREATED);
    let client_id = json_body(register).await["client_id"]
        .as_str()
        .unwrap()
        .to_string();

    let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let challenge = URL_SAFE_NO_PAD.encode(hasher.finalize());

    let approve = sso
        .app()
        .oneshot(
            Request::post("/oauth/authorize")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
                .body(Body::from(format!(
                    "client_id={}&redirect_uri={}&scope=edgequake:read%20edgequake:query&state=s1&code_challenge={}&code_challenge_method=S256&resource={}&approve=1",
                    urlencoding::encode(&client_id),
                    urlencoding::encode(REDIRECT_URI),
                    urlencoding::encode(&challenge),
                    urlencoding::encode(MCP_RESOURCE),
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(approve.status(), StatusCode::SEE_OTHER);
    let loc = location(&approve);
    let code = query_param(&loc, "code").expect("authorization code");

    let token_resp = sso
        .app()
        .oneshot(
            Request::post("/oauth/token")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(Body::from(format!(
                    "grant_type=authorization_code&code={}&redirect_uri={}&client_id={}&code_verifier={}&resource={}",
                    urlencoding::encode(&code),
                    urlencoding::encode(REDIRECT_URI),
                    urlencoding::encode(&client_id),
                    urlencoding::encode(verifier),
                    urlencoding::encode(MCP_RESOURCE),
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(token_resp.status(), StatusCode::OK);
    json_body(token_resp).await["access_token"]
        .as_str()
        .unwrap()
        .to_string()
}

// EC-158-35: MCP-audience token is rejected on REST and accepted on MCP.
#[tokio::test]
async fn sso_user_mcp_token_is_audience_bound() {
    let (sso, bearer) = sso_access().await;
    let mcp = mcp_token_for(&sso, &bearer).await;
    assert_eq!(
        sso.request("GET", "/api/v1/auth/me", &mcp).await.status(),
        StatusCode::UNAUTHORIZED,
        "MCP audience must not work on REST"
    );
    let (status, body) = mcp_tools_call_bearer(
        &sso.app(),
        "/mcp",
        &mcp,
        "edgequake_search",
        json!({ "query": "sso mcp", "mode": "naive" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

// EC-158-36: SSO user API key with read/query cannot write.
#[tokio::test]
async fn sso_user_read_query_api_key_cannot_write() {
    let (sso, bearer) = sso_access().await;
    let created = sso
        .post_json(
            "/api/v1/api-keys",
            json!({ "name": "ro", "scopes": ["read", "query"] }),
            Some(&bearer),
        )
        .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let key = json_body(created).await["api_key"]
        .as_str()
        .unwrap()
        .to_string();
    let ingest = sso
        .app()
        .oneshot(
            Request::post("/mcp")
                .header(header::AUTHORIZATION, format!("Bearer {key}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    tools_call_body("eq_ingest", json!({"text": "nope"})).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(ingest.status(), StatusCode::FORBIDDEN);
}

// EC-158-37: WebSocket upgrade with Authorization succeeds; `?token=` is rejected.
#[tokio::test]
async fn sso_access_token_upgrades_websocket_header_only() {
    let (mut sso, bearer) = sso_access().await;
    sso.state.security.cors_origins = Some(vec!["https://app.example.com".into()]);
    sso.state.security.cors_fail_closed = true;
    let app = sso.app();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve");
    });
    tokio::task::yield_now().await;

    let mut query = format!(
        "ws://{addr}/ws/pipeline/progress?token={}",
        urlencoding::encode(&bearer)
    )
    .into_client_request()
    .unwrap();
    query
        .headers_mut()
        .insert("Origin", "https://app.example.com".parse().unwrap());
    assert!(
        tokio_tungstenite::connect_async(query).await.is_err(),
        "?token= must be rejected"
    );

    let mut headered = format!("ws://{addr}/ws/pipeline/progress")
        .into_client_request()
        .unwrap();
    headered
        .headers_mut()
        .insert("Authorization", format!("Bearer {bearer}").parse().unwrap());
    headered
        .headers_mut()
        .insert("Origin", "https://app.example.com".parse().unwrap());
    let (mut ws, response) = tokio_tungstenite::connect_async(headered)
        .await
        .expect("Authorization upgrade");
    assert_eq!(
        response.status(),
        axum::http::StatusCode::SWITCHING_PROTOCOLS
    );
    let _ = futures_util::StreamExt::next(&mut ws).await;
}
