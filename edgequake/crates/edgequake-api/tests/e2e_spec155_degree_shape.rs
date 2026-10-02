//! SPEC-155 W3 EC-83/84 — degree wire shape `{in, out, total}`.

use std::collections::HashMap;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use edgequake_api::{AppState, Server, ServerConfig};
use edgequake_core::{CreateWorkspaceRequest, Tenant, TenantPlan};
use serde_json::{json, Value};
use tower::ServiceExt;

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
async fn e2e_spec155_degree_shape() {
    let state = AppState::test_state();
    let tenant = Tenant::new("DegT", "deg-t").with_plan(TenantPlan::Pro);
    let tenant = state.workspace_service.create_tenant(tenant).await.unwrap();
    let ws = state
        .workspace_service
        .create_workspace(
            tenant.tenant_id,
            CreateWorkspaceRequest {
                name: "DegWS".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    for id in ["CENTER", "N1", "N2"] {
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
    for tgt in ["N1", "N2"] {
        let mut ep = HashMap::new();
        ep.insert("relation_type".into(), json!("KNOWS"));
        ep.insert("tenant_id".into(), json!(tenant.tenant_id.to_string()));
        ep.insert("workspace_id".into(), json!(ws.workspace_id.to_string()));
        state
            .storage
            .graph_storage
            .upsert_edge("CENTER", tgt, ep)
            .await
            .unwrap();
    }

    let app = Server::new(test_config(), state).build_router();

    // GET /graph?start_node= — degree must be object, and non-zero for CENTER.
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph?start_node=CENTER&depth=1&max_nodes=20")
                .header("X-Tenant-ID", tenant.tenant_id.to_string())
                .header("X-Workspace-ID", ws.workspace_id.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = extract_json(response).await;
    let nodes = body["nodes"].as_array().expect("nodes");
    let center = nodes
        .iter()
        .find(|n| n["id"] == "CENTER")
        .expect("CENTER in response");
    let deg = &center["degree"];
    assert!(deg.is_object(), "degree must be object, got {deg}");
    assert!(deg.get("in").is_some());
    assert!(deg.get("out").is_some());
    assert!(deg.get("total").is_some());
    assert!(
        deg["total"].as_u64().unwrap_or(0) > 0,
        "start_node path must return real degree, not 0: {deg}"
    );

    // Batch degrees endpoint shape.
    let batch = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/graph/degrees/batch")
                .header("Content-Type", "application/json")
                .header("X-Tenant-ID", tenant.tenant_id.to_string())
                .header("X-Workspace-ID", ws.workspace_id.to_string())
                .body(Body::from(r#"{"node_ids":["CENTER"]}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    let batch_body = extract_json(batch).await;
    let d = &batch_body["degrees"][0]["degree"];
    assert!(d.is_object());
    assert_eq!(
        d["total"].as_u64().unwrap_or(0),
        d["in"].as_u64().unwrap_or(0) + d["out"].as_u64().unwrap_or(0)
    );
}
