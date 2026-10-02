//! SPEC-155 B3 — stream persistence exposes `finish_reason` on assistant messages.
//!
//! Note: `interrupted` is set when the SSE consumer drops mid-token-loop (see
//! `handlers/chat/streaming.rs` + `stream_persist` unit tests). Tower `oneshot`
//! does not reliably abort the producer task when the response body is dropped,
//! so this e2e proves the happy-path `stop` marker and partial content persistence.

use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use edgequake_api::{AppState, Server, ServerConfig};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

fn create_test_app() -> Router {
    Server::new(
        ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            enable_cors: false,
            enable_compression: false,
            enable_swagger: false,
        },
        AppState::test_state(),
    )
    .build_router()
}

async fn collect_sse_json_events(response: axum::response::Response) -> Vec<Value> {
    let bytes = axum::body::to_bytes(response.into_body(), 512 * 1024)
        .await
        .expect("read sse body");
    let text = String::from_utf8_lossy(&bytes);
    text.lines()
        .filter_map(|line| {
            let payload = line.strip_prefix("data: ")?;
            serde_json::from_str(payload).ok()
        })
        .collect()
}

async fn get_json_with_tenant(
    app: &Router,
    uri: &str,
    tenant: &str,
    user: &str,
) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(uri)
                .header("X-Tenant-Id", tenant)
                .header("X-User-Id", user)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let parsed = serde_json::from_slice(&bytes).unwrap_or(json!({}));
    (status, parsed)
}

#[tokio::test]
async fn e2e_spec155_chat_stream_persists_finish_reason_stop() {
    let app = create_test_app();
    let tenant = Uuid::new_v4().to_string();
    let user = Uuid::new_v4().to_string();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/chat/completions/stream")
                .header("Content-Type", "application/json")
                .header("X-Tenant-Id", &tenant)
                .header("X-User-Id", &user)
                .body(Body::from(
                    json!({
                        "message": "SPEC-155 finish_reason test",
                        "mode": "bypass",
                        "stream": true
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let events = collect_sse_json_events(response).await;
    let conversation_id = events
        .iter()
        .find_map(|e| {
            if e.get("type").and_then(|t| t.as_str()) != Some("conversation") {
                return None;
            }
            e.get("conversation_id")
                .and_then(|c| c.as_str())
                .map(str::to_string)
        })
        .expect("conversation event");

    assert!(
        events
            .iter()
            .any(|e| e.get("type").and_then(|t| t.as_str()) == Some("done")),
        "stream should complete: {events:?}"
    );

    let (status, list) = get_json_with_tenant(
        &app,
        &format!("/api/v1/conversations/{conversation_id}/messages"),
        &tenant,
        &user,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "list messages: {list}");

    let assistant = list["items"]
        .as_array()
        .and_then(|items| {
            items
                .iter()
                .find(|m| m.get("role").and_then(|r| r.as_str()) == Some("assistant"))
        })
        .expect("assistant message persisted");

    assert!(assistant
        .get("content")
        .and_then(|c| c.as_str())
        .is_some_and(|c| !c.is_empty()));
    assert_eq!(
        assistant.get("finish_reason").and_then(|f| f.as_str()),
        Some("stop")
    );
}
