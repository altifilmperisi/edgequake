//! SPEC-155 W3 — consolidated graph API contract smoke tests.
//!
//! Covers communities JSON, tenant-gated degrees batch, degree `{in,out,total}`
//! shape, and `GET /graph` `is_truncated` bool. Uses `AppState::test_state()` +
//! axum Router (same pattern as `e2e_graph.rs`).

use std::collections::HashMap;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use edgequake_api::{AppState, Server, ServerConfig};
use edgequake_core::{CreateWorkspaceRequest, Tenant, TenantPlan};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

fn test_config() -> ServerConfig {
    ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 0,
        enable_cors: false,
        enable_compression: false,
        enable_swagger: false,
    }
}

async fn extract_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("body");
    serde_json::from_slice(&bytes).expect("json")
}

async fn setup_workspace(state: &AppState, suffix: &str) -> (Uuid, Uuid) {
    let tenant =
        Tenant::new(format!("C-{suffix}"), format!("c-{suffix}")).with_plan(TenantPlan::Pro);
    let tenant = state.workspace_service.create_tenant(tenant).await.unwrap();
    let ws = state
        .workspace_service
        .create_workspace(
            tenant.tenant_id,
            CreateWorkspaceRequest {
                name: format!("CWS-{suffix}"),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    (tenant.tenant_id, ws.workspace_id)
}

async fn seed_node(state: &AppState, id: &str, tenant: Uuid, workspace: Uuid, entity_type: &str) {
    let mut props = HashMap::new();
    props.insert("entity_type".into(), json!(entity_type));
    props.insert("tenant_id".into(), json!(tenant.to_string()));
    props.insert("workspace_id".into(), json!(workspace.to_string()));
    props.insert("community_id".into(), json!(1));
    state
        .storage
        .graph_storage
        .upsert_node(id, props)
        .await
        .unwrap();
}

async fn seed_edge(
    state: &AppState,
    src: &str,
    tgt: &str,
    rel: &str,
    tenant: Uuid,
    workspace: Uuid,
) {
    let mut props = HashMap::new();
    props.insert("relation_type".into(), json!(rel));
    props.insert("tenant_id".into(), json!(tenant.to_string()));
    props.insert("workspace_id".into(), json!(workspace.to_string()));
    state
        .storage
        .graph_storage
        .upsert_edge(src, tgt, props)
        .await
        .unwrap();
}

#[tokio::test]
async fn e2e_spec155_communities_returns_200_json() {
    let state = AppState::test_state();
    let (tenant, ws) = setup_workspace(&state, "com").await;
    seed_node(&state, "C_N1", tenant, ws, "PERSON").await;
    seed_node(&state, "C_N2", tenant, ws, "PERSON").await;

    let app = Server::new(test_config(), state).build_router();
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph/communities")
                .header("X-Tenant-ID", tenant.to_string())
                .header("X-Workspace-ID", ws.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = extract_json(response).await;
    assert!(
        body.get("communities").and_then(|v| v.as_array()).is_some(),
        "communities endpoint must return JSON array: {body}"
    );
}

#[tokio::test]
async fn e2e_spec155_degrees_batch_requires_tenant() {
    let state = AppState::test_state();
    let (tenant, ws) = setup_workspace(&state, "deg-t").await;
    seed_node(&state, "DEG_NODE", tenant, ws, "PERSON").await;

    let app = Server::new(test_config(), state).build_router();

    let unscoped = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/graph/degrees/batch")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"node_ids":["DEG_NODE"]}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unscoped.status(), StatusCode::OK);
    let body = extract_json(unscoped).await;
    assert_eq!(
        body["count"].as_u64(),
        Some(0),
        "degrees batch without tenant must be empty: {body}"
    );
}

#[tokio::test]
async fn e2e_spec155_degree_shape_has_in_out_total() {
    let state = AppState::test_state();
    let (tenant, ws) = setup_workspace(&state, "deg-s").await;
    seed_node(&state, "CENTER", tenant, ws, "PERSON").await;
    seed_node(&state, "N1", tenant, ws, "PERSON").await;
    seed_edge(&state, "CENTER", "N1", "KNOWS", tenant, ws).await;

    let app = Server::new(test_config(), state).build_router();
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph?start_node=CENTER&depth=1&max_nodes=20")
                .header("X-Tenant-ID", tenant.to_string())
                .header("X-Workspace-ID", ws.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = extract_json(response).await;
    let center = body["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["id"] == "CENTER")
        .expect("CENTER present");
    let deg = &center["degree"];
    assert!(deg.is_object(), "degree must be object: {deg}");
    assert!(deg.get("in").is_some(), "missing in: {deg}");
    assert!(deg.get("out").is_some(), "missing out: {deg}");
    assert!(deg.get("total").is_some(), "missing total: {deg}");
}

#[tokio::test]
async fn e2e_spec155_get_graph_returns_is_truncated_bool() {
    let state = AppState::test_state();
    let (tenant, ws) = setup_workspace(&state, "trunc").await;
    seed_node(&state, "T1", tenant, ws, "PERSON").await;
    seed_node(&state, "T2", tenant, ws, "ORGANIZATION").await;
    seed_edge(&state, "T1", "T2", "WORKS_AT", tenant, ws).await;

    let app = Server::new(test_config(), state).build_router();
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph?max_nodes=10")
                .header("X-Tenant-ID", tenant.to_string())
                .header("X-Workspace-ID", ws.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = extract_json(response).await;
    assert!(
        body.get("is_truncated").and_then(|v| v.as_bool()).is_some(),
        "get_graph must return is_truncated bool: {body}"
    );
}
