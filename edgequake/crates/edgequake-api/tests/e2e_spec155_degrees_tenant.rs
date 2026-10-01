//! SPEC-155 W3 EC-82 — POST /graph/degrees/batch requires tenant + workspace filter.

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
    let tenant = Tenant::new(format!("DT-{suffix}"), format!("dt-{suffix}")).with_plan(TenantPlan::Pro);
    let tenant = state.workspace_service.create_tenant(tenant).await.unwrap();
    let ws = state
        .workspace_service
        .create_workspace(
            tenant.tenant_id,
            CreateWorkspaceRequest {
                name: format!("DWS-{suffix}"),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    (tenant.tenant_id, ws.workspace_id)
}

#[tokio::test]
async fn e2e_spec155_degrees_tenant() {
    let state = AppState::test_state();
    let (tenant_a, ws_a) = setup_workspace(&state, "a").await;
    let (tenant_b, ws_b) = setup_workspace(&state, "b").await;

    for (id, tenant, ws) in [
        ("NODE_A", tenant_a, ws_a),
        ("NODE_B", tenant_b, ws_b),
    ] {
        let mut props = HashMap::new();
        props.insert("entity_type".into(), json!("PERSON"));
        props.insert("tenant_id".into(), json!(tenant.to_string()));
        props.insert("workspace_id".into(), json!(ws.to_string()));
        state
            .storage
            .graph_storage
            .upsert_node(id, props)
            .await
            .unwrap();
    }

    let app = Server::new(test_config(), state).build_router();

    // Missing tenant context → empty (fail closed).
    let unscoped = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/graph/degrees/batch")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"node_ids":["NODE_A","NODE_B"]}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unscoped.status(), StatusCode::OK);
    let body = extract_json(unscoped).await;
    assert_eq!(body["count"].as_u64(), Some(0));

    // Scoped to A → only NODE_A.
    let scoped = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/graph/degrees/batch")
                .header("Content-Type", "application/json")
                .header("X-Tenant-ID", tenant_a.to_string())
                .header("X-Workspace-ID", ws_a.to_string())
                .body(Body::from(r#"{"node_ids":["NODE_A","NODE_B"]}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(scoped.status(), StatusCode::OK);
    let body = extract_json(scoped).await;
    let degrees = body["degrees"].as_array().expect("degrees array");
    assert_eq!(degrees.len(), 1);
    assert_eq!(degrees[0]["node_id"], "NODE_A");
    assert!(degrees[0]["degree"].is_object(), "degree must be object SSOT");
}
