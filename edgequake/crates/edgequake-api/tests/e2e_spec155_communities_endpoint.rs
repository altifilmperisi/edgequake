//! SPEC-155 W3 EC-86 — GET /graph/communities (+ facets smoke).

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
async fn e2e_spec155_communities_endpoint() {
    let state = AppState::test_state();
    let tenant = Tenant::new("ComT", "com-t").with_plan(TenantPlan::Pro);
    let tenant = state.workspace_service.create_tenant(tenant).await.unwrap();
    let ws = state
        .workspace_service
        .create_workspace(
            tenant.tenant_id,
            CreateWorkspaceRequest {
                name: "ComWS".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    for (id, cid, et) in [
        ("N1", 1u64, "PERSON"),
        ("N2", 1u64, "PERSON"),
        ("N3", 2u64, "ORGANIZATION"),
    ] {
        let mut props = HashMap::new();
        props.insert("entity_type".into(), json!(et));
        props.insert("community_id".into(), json!(cid));
        props.insert("tenant_id".into(), json!(tenant.tenant_id.to_string()));
        props.insert("workspace_id".into(), json!(ws.workspace_id.to_string()));
        state
            .storage
            .graph_storage
            .upsert_node(id, props)
            .await
            .unwrap();
    }

    let mut ep = HashMap::new();
    ep.insert("relation_type".into(), json!("MEMBER_OF"));
    ep.insert("tenant_id".into(), json!(tenant.tenant_id.to_string()));
    ep.insert("workspace_id".into(), json!(ws.workspace_id.to_string()));
    state
        .storage
        .graph_storage
        .upsert_edge("N1", "N3", ep)
        .await
        .unwrap();

    let app = Server::new(test_config(), state).build_router();

    let communities = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph/communities")
                .header("X-Tenant-ID", tenant.tenant_id.to_string())
                .header("X-Workspace-ID", ws.workspace_id.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(communities.status(), StatusCode::OK);
    let body = extract_json(communities).await;
    let list = body["communities"].as_array().expect("communities");
    assert_eq!(list.len(), 2);
    let c1 = list.iter().find(|c| c["id"] == "1").expect("community 1");
    assert_eq!(c1["size"].as_u64(), Some(2));

    // Graph nodes expose community_id first-class.
    let graph = app
        .clone()
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
    let gbody = extract_json(graph).await;
    assert!(gbody["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|n| n.get("community_id").is_some()));

    let facets = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph/facets")
                .header("X-Tenant-ID", tenant.tenant_id.to_string())
                .header("X-Workspace-ID", ws.workspace_id.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(facets.status(), StatusCode::OK);
    let fbody = extract_json(facets).await;
    assert!(!fbody["entity_types"].as_array().unwrap().is_empty());
    assert!(!fbody["relationship_types"].as_array().unwrap().is_empty());
}
