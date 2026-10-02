//! Popular labels and batch degree handlers.
//!
//! - `get_popular_labels` — top entities sorted by connection count
//! - `get_degrees_batch` — bulk degree lookup (50× faster than N individual queries)

use axum::{
    extract::{Query, State},
    Json,
};

use crate::error::ApiResult;
use crate::handlers::graph::graph_dto::{degrees_breakdown_for_workspace, workspace_graph_totals};
use crate::handlers::graph_types::*;
use crate::middleware::TenantContext;
use crate::services::{
    admit_graph_materialization, run_timed_graph_query,
    tenant_guard::{has_full_tenant_context, warn_missing_tenant_context},
};
use crate::state::{GraphQueryRuntime, StorageRuntime};

/// Get popular entities/labels sorted by connection count.
#[utoipa::path(
    get,
    path = "/api/v1/graph/labels/popular",
    tag = "Graph",
    params(
        ("limit" = usize, Query, description = "Max results (default 50)"),
        ("min_degree" = Option<usize>, Query, description = "Minimum connections"),
        ("entity_type" = Option<String>, Query, description = "Filter by type")
    ),
    responses(
        (status = 200, description = "Popular labels retrieved", body = PopularLabelsResponse)
    )
)]
pub async fn get_popular_labels(
    State(storage): State<StorageRuntime>,
    State(graph): State<GraphQueryRuntime>,
    tenant_ctx: TenantContext,
    Query(params): Query<PopularLabelsQuery>,
) -> ApiResult<Json<PopularLabelsResponse>> {
    let _materialize_guard = admit_graph_materialization(&graph)?;

    // SPEC-155 B01: workspace-exact entity total.
    let (total_entities, _) = workspace_graph_totals(&storage.graph_storage, &tenant_ctx).await?;

    let limit = params.limit;
    let min_degree = params.min_degree;
    let entity_type = params.entity_type.clone();
    let tenant_id = tenant_ctx.tenant_id.clone();
    let workspace_id = tenant_ctx.workspace_id.clone();
    let graph_storage = storage.graph_storage.clone();
    let popular_nodes = run_timed_graph_query(&graph.budget, "popular_labels", async move {
        graph_storage
            .get_popular_nodes_with_degree(
                limit,
                min_degree,
                entity_type.as_deref(),
                tenant_id.as_deref(),
                workspace_id.as_deref(),
            )
            .await
    })
    .await?;

    let node_ids: Vec<String> = popular_nodes.iter().map(|(n, _)| n.id.clone()).collect();
    let degree_map =
        crate::handlers::graph::degrees_breakdown_batch(&storage.graph_storage, &node_ids).await;

    let labels: Vec<PopularLabel> = popular_nodes
        .into_iter()
        .map(|(node, total)| {
            let entity_type = node
                .properties
                .get("entity_type")
                .and_then(|v| v.as_str())
                .unwrap_or("UNKNOWN")
                .to_string();

            let description = node
                .properties
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let degree = degree_map
                .get(&node.id)
                .copied()
                .unwrap_or_else(|| DegreeBreakdown::from_total(total));

            PopularLabel {
                label: crate::handlers::graph::graph_node_label(&node),
                entity_type,
                degree,
                description,
            }
        })
        .collect();

    Ok(Json(PopularLabelsResponse {
        labels,
        total_entities,
    }))
}

/// Get degrees for multiple nodes in a single optimized query.
///
/// This endpoint uses the optimized `node_degrees_batch()` method which is
/// 50x faster than calling GET /graph/nodes/{id} multiple times.
///
/// Performance: <100ms for 100 nodes (vs 5000ms+ with individual queries).
///
/// SPEC-155 B03: requires TenantContext and filters by workspace.
#[utoipa::path(
    post,
    path = "/api/v1/graph/degrees/batch",
    tag = "Graph",
    request_body = BatchDegreeRequest,
    responses(
        (status = 200, description = "Degrees retrieved", body = BatchDegreeResponse)
    )
)]
pub async fn get_degrees_batch(
    State(storage): State<StorageRuntime>,
    tenant_ctx: TenantContext,
    Json(request): Json<BatchDegreeRequest>,
) -> ApiResult<Json<BatchDegreeResponse>> {
    if !has_full_tenant_context(&tenant_ctx) {
        warn_missing_tenant_context(&tenant_ctx, "get_degrees_batch");
        return Ok(Json(BatchDegreeResponse {
            degrees: Vec::new(),
            count: 0,
        }));
    }

    if request.node_ids.is_empty() {
        return Ok(Json(BatchDegreeResponse {
            degrees: Vec::new(),
            count: 0,
        }));
    }

    let scoped =
        degrees_breakdown_for_workspace(&storage.graph_storage, &tenant_ctx, &request.node_ids)
            .await?;

    let degrees: Vec<NodeDegree> = scoped
        .into_iter()
        .map(|(node_id, degree)| NodeDegree { node_id, degree })
        .collect();

    let count = degrees.len();

    Ok(Json(BatchDegreeResponse { degrees, count }))
}
