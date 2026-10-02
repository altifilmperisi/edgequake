//! SPEC-155 B2 — PATCH message feedback persists and appears in list/get.

mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use common::{
    create_test_app, get_with_tenant, post_json_with_tenant, TEST_TENANT_ID, TEST_USER_ID,
    TEST_WORKSPACE_ID,
};
use serde_json::{json, Value};
use tower::ServiceExt;

async fn patch_json_with_tenant(
    app: &Router,
    uri: &str,
    body: &Value,
    tenant: &str,
    user: &str,
    workspace: &str,
) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(uri)
                .header("Content-Type", "application/json")
                .header("X-Tenant-Id", tenant)
                .header("X-User-Id", user)
                .header("X-Workspace-Id", workspace)
                .body(Body::from(body.to_string()))
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
async fn e2e_spec155_message_feedback_roundtrip() {
    let app = create_test_app();

    let (status, body) = post_json_with_tenant(
        &app,
        "/api/v1/conversations",
        &json!({ "title": "SPEC-155 feedback" }),
        TEST_TENANT_ID,
        TEST_USER_ID,
        TEST_WORKSPACE_ID,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "create conversation: {body}");
    let conv_id = body["id"].as_str().expect("conversation id");

    let (msg_status, msg_body) = post_json_with_tenant(
        &app,
        &format!("/api/v1/conversations/{conv_id}/messages"),
        &json!({
            "content": "assistant answer",
            "role": "assistant",
            "stream": false
        }),
        TEST_TENANT_ID,
        TEST_USER_ID,
        TEST_WORKSPACE_ID,
    )
    .await;
    assert_eq!(
        msg_status,
        StatusCode::CREATED,
        "create message: {msg_body}"
    );
    let message_id = msg_body["id"].as_str().expect("message id");

    let (fb_status, fb_body) = patch_json_with_tenant(
        &app,
        &format!("/api/v1/conversations/{conv_id}/messages/{message_id}/feedback"),
        &json!({ "rating": "up", "reason": "helpful citations" }),
        TEST_TENANT_ID,
        TEST_USER_ID,
        TEST_WORKSPACE_ID,
    )
    .await;
    assert_eq!(fb_status, StatusCode::OK, "set feedback: {fb_body}");
    assert_eq!(fb_body["feedback_rating"], "up");
    assert_eq!(fb_body["feedback_reason"], "helpful citations");

    let (list_status, list_body) = get_with_tenant(
        &app,
        &format!("/api/v1/conversations/{conv_id}/messages"),
        TEST_TENANT_ID,
        TEST_USER_ID,
        TEST_WORKSPACE_ID,
    )
    .await;
    assert_eq!(list_status, StatusCode::OK, "list messages: {list_body}");
    let items = list_body["items"].as_array().expect("items");
    let msg = items
        .iter()
        .find(|m| m["id"].as_str() == Some(message_id))
        .expect("message in list");
    assert_eq!(msg["feedback_rating"], "up");

    let (get_status, get_body) = get_with_tenant(
        &app,
        &format!("/api/v1/conversations/{conv_id}"),
        TEST_TENANT_ID,
        TEST_USER_ID,
        TEST_WORKSPACE_ID,
    )
    .await;
    assert_eq!(get_status, StatusCode::OK, "get conversation: {get_body}");
    let embedded = get_body["messages"]
        .as_array()
        .and_then(|a| a.iter().find(|m| m["id"].as_str() == Some(message_id)))
        .expect("message in conversation payload");
    assert_eq!(embedded["feedback_rating"], "up");

    let (clear_status, clear_body) = patch_json_with_tenant(
        &app,
        &format!("/api/v1/conversations/{conv_id}/messages/{message_id}/feedback"),
        &json!({ "rating": null }),
        TEST_TENANT_ID,
        TEST_USER_ID,
        TEST_WORKSPACE_ID,
    )
    .await;
    assert_eq!(clear_status, StatusCode::OK, "clear feedback: {clear_body}");
    assert!(clear_body["feedback_rating"].is_null());
}
