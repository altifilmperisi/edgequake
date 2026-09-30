//! SPEC-149 — Real WebSocket frame exchange for multiplexed progress.
//!
//! ```bash
//! cargo test -p edgequake-api --test e2e_spec149_realtime_progress
//! ```

use std::net::SocketAddr;
use std::time::Duration;

use axum::http::StatusCode;
use edgequake_api::handlers::ProgressEvent;
use edgequake_api::{AppState, Server, ServerConfig};
use edgequake_auth::{Claims, Role};
use edgequake_tasks::{Task, TaskType};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

const ALLOWED_ORIGIN: &str = "https://app.example.com";
const API_KEY: &str = "spec149-test-key";

fn production_auth_state() -> AppState {
    let mut state = AppState::test_state();
    state.auth.config.auth_enabled = true;
    state.auth.config.dev_mode = false;
    state.auth.config.api_keys = vec![API_KEY.to_string()];
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
    // Tiny yield so the acceptor is ready.
    tokio::task::yield_now().await;
    (addr, handle)
}

fn mint_jwt(state: &AppState, tenant_id: Uuid, workspace_id: Uuid) -> String {
    let claims = Claims::new(Uuid::new_v4(), Role::User, 3600)
        .with_tenant_id(tenant_id.to_string())
        .with_workspace_id(workspace_id.to_string());
    state
        .auth
        .jwt
        .generate_token_with_claims(claims)
        .expect("sign jwt")
}

async fn connect_ws(
    addr: SocketAddr,
    token: Option<&str>,
    origin: Option<&str>,
) -> Result<
    (
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        axum::http::Response<Option<Vec<u8>>>,
    ),
    tokio_tungstenite::tungstenite::Error,
> {
    let url = format!("ws://{addr}/ws/pipeline/progress");
    let mut request = url.into_client_request().expect("request");
    if let Some(token) = token {
        // SPEC-154 Wave 5: Authorization header only (no ?token=).
        request.headers_mut().insert(
            "Authorization",
            format!("Bearer {token}").parse().unwrap(),
        );
    }
    if let Some(origin) = origin {
        request
            .headers_mut()
            .insert("Origin", origin.parse().unwrap());
    }
    tokio_tungstenite::connect_async(request).await
}

async fn expect_json_type(
    ws: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    expected: &str,
    timeout: Duration,
) -> serde_json::Value {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            panic!("timed out waiting for type={expected}");
        }
        let msg = tokio::time::timeout(remaining, ws.next())
            .await
            .expect("timeout")
            .expect("stream ended")
            .expect("ws error");
        let Message::Text(text) = msg else {
            continue;
        };
        let value: serde_json::Value = serde_json::from_str(&text).expect("json");
        if value["type"] == expected {
            return value;
        }
        // Skip heartbeats / connected while waiting for a specific type.
        if expected != "Connected" && value["type"] == "Connected" {
            continue;
        }
        if value["type"] == "Heartbeat" {
            continue;
        }
        if value["type"] != expected {
            // Keep scanning for the expected event among noise.
            continue;
        }
    }
}

#[tokio::test]
async fn c149_01_owner_subscribe_receives_stage_transition() {
    let state = production_auth_state();
    let tenant = Uuid::new_v4();
    let workspace = Uuid::new_v4();
    let task = Task::new(tenant, workspace, TaskType::Insert, serde_json::json!({}));
    let track_id = task.track_id.clone();
    let document_id = Uuid::new_v4().to_string();
    state
        .tasks
        .storage
        .create_task(&task)
        .await
        .expect("create task");

    let token = mint_jwt(&state, tenant, workspace);
    let broadcaster = state.tasks.progress_broadcaster.clone();
    let (addr, _handle) = bind_server(state).await;

    let (mut ws, response) = connect_ws(addr, Some(&token), Some(ALLOWED_ORIGIN))
        .await
        .expect("ws connect");
    assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);

    let _ = expect_json_type(&mut ws, "Connected", Duration::from_secs(2)).await;

    ws.send(Message::Text(
        serde_json::json!({
            "type": "subscribe",
            "track_ids": [track_id]
        })
        .to_string()
        .into(),
    ))
    .await
    .expect("send subscribe");

    let ack = expect_json_type(&mut ws, "SubscribedAck", Duration::from_secs(2)).await;
    assert_eq!(ack["data"]["accepted"][0], track_id);

    broadcaster.broadcast(ProgressEvent::StageTransition {
        document_id: document_id.clone(),
        task_id: track_id.clone(),
        stage: "extracting".into(),
        stage_message: "Extracting".into(),
        stage_progress: Some(0.4),
    });

    let event = expect_json_type(&mut ws, "StageTransition", Duration::from_secs(2)).await;
    assert_eq!(event["data"]["task_id"], track_id);
    assert_eq!(event["data"]["stage"], "extracting");
}

#[tokio::test]
async fn c149_02_foreign_workspace_subscribe_silent() {
    let state = production_auth_state();
    let owner_tenant = Uuid::new_v4();
    let owner_ws = Uuid::new_v4();
    let foreign_ws = Uuid::new_v4();
    let task = Task::new(
        owner_tenant,
        owner_ws,
        TaskType::Insert,
        serde_json::json!({}),
    );
    let track_id = task.track_id.clone();
    state
        .tasks
        .storage
        .create_task(&task)
        .await
        .expect("create task");

    let foreign_token = mint_jwt(&state, owner_tenant, foreign_ws);
    let broadcaster = state.tasks.progress_broadcaster.clone();
    let (addr, _handle) = bind_server(state).await;

    let (mut ws, response) = connect_ws(addr, Some(&foreign_token), Some(ALLOWED_ORIGIN))
        .await
        .expect("ws connect");
    assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);
    let _ = expect_json_type(&mut ws, "Connected", Duration::from_secs(2)).await;

    ws.send(Message::Text(
        serde_json::json!({
            "type": "subscribe",
            "track_ids": [track_id]
        })
        .to_string()
        .into(),
    ))
    .await
    .unwrap();

    let ack = expect_json_type(&mut ws, "SubscribedAck", Duration::from_secs(2)).await;
    assert!(
        ack["data"]["accepted"]
            .as_array()
            .map(|a| a.is_empty())
            .unwrap_or(true),
        "foreign workspace must not accept the track"
    );

    broadcaster.broadcast(ProgressEvent::StageTransition {
        document_id: "doc".into(),
        task_id: track_id,
        stage: "extracting".into(),
        stage_message: "Extracting".into(),
        stage_progress: None,
    });

    let result = tokio::time::timeout(Duration::from_millis(400), async {
        while let Some(Ok(Message::Text(text))) = ws.next().await {
            let value: serde_json::Value = serde_json::from_str(&text).unwrap();
            if value["type"] == "StageTransition" {
                return Some(value);
            }
        }
        None
    })
    .await;

    assert!(
        matches!(result, Ok(None) | Err(_)),
        "foreign session must not receive StageTransition"
    );
}

#[tokio::test]
async fn c149_03_unsubscribe_stops_delivery() {
    let state = production_auth_state();
    let tenant = Uuid::new_v4();
    let workspace = Uuid::new_v4();
    let task = Task::new(tenant, workspace, TaskType::Insert, serde_json::json!({}));
    let track_id = task.track_id.clone();
    state
        .tasks
        .storage
        .create_task(&task)
        .await
        .expect("create task");

    let token = mint_jwt(&state, tenant, workspace);
    let broadcaster = state.tasks.progress_broadcaster.clone();
    let (addr, _handle) = bind_server(state).await;

    let (mut ws, _) = connect_ws(addr, Some(&token), Some(ALLOWED_ORIGIN))
        .await
        .unwrap();
    let _ = expect_json_type(&mut ws, "Connected", Duration::from_secs(2)).await;

    ws.send(Message::Text(
        serde_json::json!({"type":"subscribe","track_ids":[&track_id]})
            .to_string()
            .into(),
    ))
    .await
    .unwrap();
    let _ = expect_json_type(&mut ws, "SubscribedAck", Duration::from_secs(2)).await;

    ws.send(Message::Text(
        serde_json::json!({"type":"unsubscribe","track_ids":[&track_id]})
            .to_string()
            .into(),
    ))
    .await
    .unwrap();

    // Allow unsubscribe to apply before broadcast.
    tokio::time::sleep(Duration::from_millis(50)).await;

    broadcaster.broadcast(ProgressEvent::StageTransition {
        document_id: "doc".into(),
        task_id: track_id,
        stage: "merging".into(),
        stage_message: "Merging".into(),
        stage_progress: Some(0.9),
    });

    let result = tokio::time::timeout(Duration::from_millis(400), async {
        while let Some(Ok(Message::Text(text))) = ws.next().await {
            let value: serde_json::Value = serde_json::from_str(&text).unwrap();
            if value["type"] == "StageTransition" {
                return Some(value);
            }
        }
        None
    })
    .await;

    assert!(
        matches!(result, Ok(None) | Err(_)),
        "unsubscribed track must not deliver"
    );
}

#[tokio::test]
async fn c149_04_missing_token_rejected() {
    let state = production_auth_state();
    let (addr, _handle) = bind_server(state).await;

    let result = connect_ws(addr, None, Some(ALLOWED_ORIGIN)).await;
    assert!(
        result.is_err(),
        "missing token must fail handshake when auth enabled"
    );
}

#[tokio::test]
async fn c149_05_bad_origin_rejected() {
    let state = production_auth_state();
    let (addr, _handle) = bind_server(state).await;

    let result = connect_ws(addr, Some(API_KEY), Some("https://evil.example.com")).await;
    assert!(result.is_err(), "disallowed origin must fail handshake");
}

#[tokio::test]
async fn c149_06_duplicate_subscribe_idempotent() {
    let state = production_auth_state();
    let tenant = Uuid::new_v4();
    let workspace = Uuid::new_v4();
    let task = Task::new(tenant, workspace, TaskType::Insert, serde_json::json!({}));
    let track_id = task.track_id.clone();
    state
        .tasks
        .storage
        .create_task(&task)
        .await
        .expect("create task");

    let token = mint_jwt(&state, tenant, workspace);
    let broadcaster = state.tasks.progress_broadcaster.clone();
    let (addr, _handle) = bind_server(state).await;

    let (mut ws, _) = connect_ws(addr, Some(&token), Some(ALLOWED_ORIGIN))
        .await
        .unwrap();
    let _ = expect_json_type(&mut ws, "Connected", Duration::from_secs(2)).await;

    for _ in 0..2 {
        ws.send(Message::Text(
            serde_json::json!({"type":"subscribe","track_ids":[&track_id]})
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
        let ack = expect_json_type(&mut ws, "SubscribedAck", Duration::from_secs(2)).await;
        assert_eq!(ack["data"]["accepted"][0], track_id);
    }

    broadcaster.broadcast(ProgressEvent::StageTransition {
        document_id: "doc".into(),
        task_id: track_id.clone(),
        stage: "embedding".into(),
        stage_message: "Embedding".into(),
        stage_progress: Some(0.7),
    });

    let event = expect_json_type(&mut ws, "StageTransition", Duration::from_secs(2)).await;
    assert_eq!(event["data"]["task_id"], track_id);
}
