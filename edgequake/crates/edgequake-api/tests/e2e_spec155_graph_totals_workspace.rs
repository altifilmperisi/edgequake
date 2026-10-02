//! SPEC-155 W3 EC-80 — workspace-exact graph totals (not shared AGE reltuples).
//!
//! Seeds two workspaces in memory graph storage and asserts GET /graph
//! `total_nodes` / `total_edges` reflect only the requested workspace.
//!
//! Postgres: `node_count_by_workspace` / `edge_count_by_workspace` are the
//! same trait methods used here; see `common/spec013_postgres.rs` for live PG.

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
        Tenant::new(format!("T-{suffix}"), format!("t-{suffix}")).with_plan(TenantPlan::Pro);
    let tenant = state.workspace_service.create_tenant(tenant).await.unwrap();
    let ws = state
        .workspace_service
        .create_workspace(
            tenant.tenant_id,
            CreateWorkspaceRequest {
                name: format!("WS-{suffix}"),
                slug: None,
                description: None,
                max_documents: None,
                llm_model: None,
                llm_provider: None,
                embedding_model: None,
                embedding_provider: None,
                embedding_dimension: None,
                vision_llm_model: None,
                pdf_parser_backend: None,
                entity_types: None,
                vision_llm_provider: None,
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
    props.insert("description".into(), json!("seed edge"));
    props.insert("keywords".into(), json!("a,b"));
    props.insert("weight".into(), json!(0.9));
    state
        .storage
        .graph_storage
        .upsert_edge(src, tgt, props)
        .await
        .unwrap();
}

#[tokio::test]
async fn e2e_spec155_graph_totals_workspace() {
    let state = AppState::test_state();
    let (tenant_a, ws_a) = setup_workspace(&state, "a").await;
    let (tenant_b, ws_b) = setup_workspace(&state, "b").await;

    // Workspace A: 2 nodes, 1 edge
    seed_node(&state, "A_PERSON", tenant_a, ws_a, "PERSON").await;
    seed_node(&state, "A_ORG", tenant_a, ws_a, "ORGANIZATION").await;
    seed_edge(&state, "A_PERSON", "A_ORG", "WORKS_AT", tenant_a, ws_a).await;

    // Workspace B: 1 node (must not inflate A's totals)
    seed_node(&state, "B_ONLY", tenant_b, ws_b, "PERSON").await;

    let app = Server::new(test_config(), state.clone()).build_router();
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph?max_nodes=10")
                .header("X-Tenant-ID", tenant_a.to_string())
                .header("X-Workspace-ID", ws_a.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = extract_json(response).await;

    assert_eq!(
        body["total_nodes"].as_u64(),
        Some(2),
        "workspace A totals must not include workspace B nodes: {body}"
    );
    assert_eq!(body["total_edges"].as_u64(), Some(1));
    assert_eq!(body["max_nodes"].as_u64(), Some(10));

    let returned = body["nodes"].as_array().map(|a| a.len()).unwrap_or(0);
    let total = body["total_nodes"].as_u64().unwrap_or(0) as usize;
    let is_truncated = body["is_truncated"].as_bool().unwrap_or(true);
    assert_eq!(
        is_truncated,
        returned < total,
        "is_truncated must follow SPEC-155 contract"
    );

    assert_eq!(
        state
            .storage
            .graph_storage
            .node_count_by_workspace(&ws_a)
            .await
            .unwrap(),
        2
    );
    assert_eq!(
        state
            .storage
            .graph_storage
            .node_count_by_workspace(&ws_b)
            .await
            .unwrap(),
        1
    );
}
