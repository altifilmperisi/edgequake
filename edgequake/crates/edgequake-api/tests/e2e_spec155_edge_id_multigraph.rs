//! SPEC-155 W3 EC-85 — edge DTO stable id includes relation_type (multigraph).

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

#[tokio::test]
async fn e2e_spec155_edge_id_multigraph() {
    let state = AppState::test_state();
    let tenant = Tenant::new("EdgeT", "edge-t").with_plan(TenantPlan::Pro);
    let tenant = state.workspace_service.create_tenant(tenant).await.unwrap();
    let ws = state
        .workspace_service
        .create_workspace(
            tenant.tenant_id,
            CreateWorkspaceRequest {
                name: "EdgeWS".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    for id in ["SRC", "TGT"] {
        let mut props = HashMap::new();
        props.insert("entity_type".into(), json!("PERSON"));
        props.insert("tenant_id".into(), json!(tenant.tenant_id.to_string()));
        props.insert("workspace_id".into(), json!(ws.workspace_id.to_string()));
        state
            .storage
            .graph_storage
            .upsert_node(id, props)
            .await
            .unwrap();
    }

    for rel in ["WORKS_AT", "KNOWS"] {
        let mut ep = HashMap::new();
        ep.insert("relation_type".into(), json!(rel));
        ep.insert("description".into(), json!(format!("desc-{rel}")));
        ep.insert("keywords".into(), json!(["k1", "k2"]));
        ep.insert("weight".into(), json!(0.75));
        ep.insert("tenant_id".into(), json!(tenant.tenant_id.to_string()));
        ep.insert("workspace_id".into(), json!(ws.workspace_id.to_string()));
        state
            .storage
            .graph_storage
            .upsert_edge("SRC", "TGT", ep)
            .await
            .unwrap();
    }

    let app = Server::new(test_config(), state).build_router();
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph?max_nodes=20")
                .header("X-Tenant-ID", tenant.tenant_id.to_string())
                .header("X-Workspace-ID", ws.workspace_id.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = extract_json(response).await;
    let edges = body["edges"].as_array().expect("edges");
    assert!(
        edges.len() >= 2,
        "expected multigraph parallel edges, got {edges:?}"
    );

    let ids: Vec<&str> = edges.iter().filter_map(|e| e["id"].as_str()).collect();
    assert!(
        ids.iter().any(|id| id.contains("WORKS_AT")),
        "stable id must include relation_type: {ids:?}"
    );
    assert!(
        ids.iter().any(|id| id.contains("KNOWS")),
        "stable id must include relation_type: {ids:?}"
    );
    assert_ne!(ids[0], ids[1], "parallel edges must have distinct ids");

    let sample = &edges[0];
    assert!(sample.get("description").is_some());
    assert!(sample.get("keywords").is_some());
    assert!(sample.get("weight").is_some());
}
