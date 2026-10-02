//! SPEC-155 B1 — chat stream emits explicit `stage` timeline events and always sends `context`.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use edgequake_api::{AppState, Server, ServerConfig};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

fn create_test_app() -> axum::Router {
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

#[tokio::test]
async fn e2e_spec155_chat_stage_events() {
    let app = create_test_app();
    let tenant = Uuid::new_v4().to_string();
    let user = Uuid::new_v4().to_string();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/chat/completions/stream")
                .header("Content-Type", "application/json")
                .header("X-Tenant-Id", &tenant)
                .header("X-User-Id", &user)
                .body(Body::from(
                    json!({
                        "message": "SPEC-155 stage timeline test",
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
    assert!(
        events
            .iter()
            .any(|e| e.get("type").and_then(|t| t.as_str()) == Some("stage")
                && e.get("stage").and_then(|s| s.as_str()) == Some("retrieving")),
        "expected stage=retrieving event, got: {events:?}"
    );

    let context = events
        .iter()
        .find(|e| e.get("type").and_then(|t| t.as_str()) == Some("context"))
        .expect("context SSE event must always be emitted");
    assert!(
        context.get("sources").and_then(|s| s.as_array()).is_some(),
        "context must include sources array (possibly empty)"
    );
}
