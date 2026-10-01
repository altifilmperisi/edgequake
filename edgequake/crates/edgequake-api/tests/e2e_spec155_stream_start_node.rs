//! SPEC-155 W3 EC-81 — GET /graph/stream honours `start_node` via BFS.

use std::collections::HashMap;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use edgequake_api::{AppState, Server, ServerConfig};
use edgequake_core::{CreateWorkspaceRequest, Tenant, TenantPlan};
use futures::StreamExt;
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

async fn setup_workspace(state: &AppState) -> (Uuid, Uuid) {
    let tenant = Tenant::new("StreamT", "stream-t").with_plan(TenantPlan::Pro);
    let tenant = state.workspace_service.create_tenant(tenant).await.unwrap();
    let ws = state
        .workspace_service
        .create_workspace(
            tenant.tenant_id,
            CreateWorkspaceRequest {
                name: "StreamWS".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    (tenant.tenant_id, ws.workspace_id)
}

async fn seed(
    state: &AppState,
    id: &str,
    tenant: Uuid,
    workspace: Uuid,
) {
    let mut props = HashMap::new();
    props.insert("entity_type".into(), json!("PERSON"));
    props.insert("tenant_id".into(), json!(tenant.to_string()));
    props.insert("workspace_id".into(), json!(workspace.to_string()));
    state
        .storage
        .graph_storage
        .upsert_node(id, props)
        .await
        .unwrap();
}

#[tokio::test]
async fn e2e_spec155_stream_start_node() {
    let state = AppState::test_state();
    let (tenant, ws) = setup_workspace(&state).await;

    // Hub + spokes + an isolated popular node that must NOT appear when streaming from hub.
    seed(&state, "HUB", tenant, ws).await;
    seed(&state, "SPOKE_A", tenant, ws).await;
    seed(&state, "SPOKE_B", tenant, ws).await;
    seed(&state, "ISOLATED_POPULAR", tenant, ws).await;

    for tgt in ["SPOKE_A", "SPOKE_B"] {
        let mut ep = HashMap::new();
        ep.insert("relation_type".into(), json!("LINKS"));
        ep.insert("tenant_id".into(), json!(tenant.to_string()));
        ep.insert("workspace_id".into(), json!(ws.to_string()));
        state
            .storage
            .graph_storage
            .upsert_edge("HUB", tgt, ep)
            .await
            .unwrap();
    }

    let app = Server::new(test_config(), state).build_router();
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph/stream?start_node=HUB&max_nodes=50&batch_size=10")
                .header("X-Tenant-ID", tenant.to_string())
                .header("X-Workspace-ID", ws.to_string())
                .header("Accept", "text/event-stream")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let mut body = response.into_body().into_data_stream();
    let mut buf = String::new();
    while let Some(chunk) = body.next().await {
        let bytes = chunk.expect("sse chunk");
        buf.push_str(&String::from_utf8_lossy(&bytes));
        if buf.contains("\"type\":\"done\"") {
            break;
        }
    }

    let mut streamed_ids = Vec::new();
    for line in buf.lines() {
        let Some(data) = line.strip_prefix("data: ") else {
            continue;
        };
        let Ok(event) = serde_json::from_str::<Value>(data) else {
            continue;
        };
        if event["type"] == "nodes" {
            if let Some(nodes) = event["nodes"].as_array() {
                for n in nodes {
                    if let Some(id) = n["id"].as_str() {
                        streamed_ids.push(id.to_string());
                    }
                }
            }
        }
    }

    assert!(
        streamed_ids.iter().any(|id| id == "HUB"),
        "start_node HUB must be in stream: {streamed_ids:?}"
    );
    assert!(
        streamed_ids.iter().any(|id| id == "SPOKE_A"),
        "BFS neighbors must be streamed: {streamed_ids:?}"
    );
    assert!(
        !streamed_ids.iter().any(|id| id == "ISOLATED_POPULAR"),
        "isolated popular node must not appear when start_node is set: {streamed_ids:?}"
    );
}
