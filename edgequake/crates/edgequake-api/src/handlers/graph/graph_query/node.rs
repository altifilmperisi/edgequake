//! Single-node lookup handler (`GET /api/v1/graph/nodes/{node_id}`).

use axum::{
    extract::{Path, State},
    Json,
};

use crate::error::ApiResult;
use crate::handlers::graph::graph_dto::{degrees_breakdown_batch, graph_node_response};
use crate::handlers::graph_types::{DegreeBreakdown, GraphNodeResponse};
use crate::handlers::isolation::load_node_for_tenant_context;
use crate::middleware::TenantContext;
use crate::state::StorageRuntime;

/// Get a specific node.
#[utoipa::path(
    get,
    path = "/api/v1/graph/nodes/{node_id}",
    tag = "Graph",
    params(
        ("node_id" = String, Path, description = "Node ID")
    ),
    responses(
        (status = 200, description = "Node retrieved", body = GraphNodeResponse),
        (status = 404, description = "Node not found")
    )
)]
pub async fn get_node(
    State(storage): State<StorageRuntime>,
    tenant_ctx: TenantContext,
    Path(node_id): Path<String>,
) -> ApiResult<Json<GraphNodeResponse>> {
    let node =
        load_node_for_tenant_context(storage.graph_storage.as_ref(), &node_id, &tenant_ctx).await?;

    let degree = degrees_breakdown_batch(&storage.graph_storage, std::slice::from_ref(&node_id))
        .await
        .remove(&node_id)
        .unwrap_or_else(|| DegreeBreakdown::from_total(0));

    Ok(Json(graph_node_response(&node, degree)))
}
