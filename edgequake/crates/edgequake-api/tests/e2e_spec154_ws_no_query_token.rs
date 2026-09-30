//! SPEC-154 Wave 5 — WebSocket rejects `?token=` (EC-154-10 / EC-154-11).

use std::net::SocketAddr;

use axum::http::StatusCode;
use edgequake_api::{AppState, Server, ServerConfig};
use futures_util::StreamExt;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

const ALLOWED_ORIGIN: &str = "https://app.example.com";
const API_KEY: &str = "spec154-ws-key";

fn auth_state() -> AppState {
    let mut state = AppState::test_state();
    state.auth.config.auth_enabled = true;
    state.auth.config.dev_mode = false;
    state.auth.config.api_keys = vec![API_KEY.to_string()];
    state.auth.config.master_api_key = Some(API_KEY.to_string());
    state.security.cors_origins = Some(vec![ALLOWED_ORIGIN.to_string()]);
    state.security.cors_fail_closed = true;
    state
}

fn build_router(state: AppState) -> axum::Router {
    Server::new(
        ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            enable_cors: true,
            enable_compression: false,
            enable_swagger: false,
        },
        state,
    )
    .build_router()
}

async fn bind_server(state: AppState) -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let app = build_router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral");
    let addr = listener.local_addr().expect("local addr");
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve");
    });
    tokio::task::yield_now().await;
    (addr, handle)
}

#[tokio::test]
async fn ec_154_10_ws_query_token_rejected() {
    let (addr, _handle) = bind_server(auth_state()).await;
    let url = format!(
        "ws://{addr}/ws/pipeline/progress?token={}",
        urlencoding::encode(API_KEY)
    );
    let mut request = url.into_client_request().expect("request");
    request
        .headers_mut()
        .insert("Origin", ALLOWED_ORIGIN.parse().unwrap());
    let err = tokio_tungstenite::connect_async(request)
        .await
        .expect_err("query token must fail handshake");
    let msg = err.to_string();
    assert!(
        msg.contains("401") || msg.contains("Unauthorized") || msg.contains("HTTP error"),
        "expected 401 rejection, got: {msg}"
    );
}

#[tokio::test]
async fn ec_154_11_ws_authorization_header_accepted() {
    let (addr, _handle) = bind_server(auth_state()).await;
    let url = format!("ws://{addr}/ws/pipeline/progress");
    let mut request = url.into_client_request().expect("request");
    request.headers_mut().insert(
        "Authorization",
        format!("Bearer {API_KEY}").parse().unwrap(),
    );
    request
        .headers_mut()
        .insert("Origin", ALLOWED_ORIGIN.parse().unwrap());
    let (mut ws, response) = tokio_tungstenite::connect_async(request)
        .await
        .expect("bearer header must upgrade");
    assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);
    // Drain optional Connected frame so the server doesn't see a half-open client.
    let _ = ws.next().await;
}

#[tokio::test]
async fn ec_154_11b_ws_sec_websocket_protocol_accepted() {
    let (addr, _handle) = bind_server(auth_state()).await;
    let url = format!("ws://{addr}/ws/pipeline/progress");
    let mut request = url.into_client_request().expect("request");
    // Browser shape: sentinel + credential (no Authorization header).
    request.headers_mut().insert(
        "Sec-WebSocket-Protocol",
        format!("edgequake.bearer, {API_KEY}").parse().unwrap(),
    );
    request
        .headers_mut()
        .insert("Origin", ALLOWED_ORIGIN.parse().unwrap());
    let (mut ws, response) = tokio_tungstenite::connect_async(request)
        .await
        .expect("Sec-WebSocket-Protocol credential must upgrade");
    assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);
    let selected = response
        .headers()
        .get("Sec-WebSocket-Protocol")
        .and_then(|v| v.to_str().ok());
    assert_eq!(
        selected,
        Some("edgequake.bearer"),
        "server must echo sentinel protocol"
    );
    let _ = ws.next().await;
}
