//! Graph traversal handler (`GET /api/v1/graph`).
//!
//! Returns knowledge graph data with optional BFS traversal from a
//! starting node, including timeout-guarded fallback paths.

use axum::{
    extract::{Query, State},
    Json,
};
use tracing::debug;

use crate::error::ApiResult;
use crate::handlers::graph::graph_dto::{
    degrees_breakdown_batch, edge_response, graph_is_truncated, graph_node_response,
    workspace_graph_totals,
};
use crate::handlers::graph_types::*;
use crate::handlers::isolation::properties_match_tenant_context;
use crate::middleware::TenantContext;
use crate::services::{
    admit_graph_materialization, run_timed_graph_query,
    tenant_guard::{empty_graph_response, has_full_tenant_context, warn_missing_tenant_context},
};
use crate::state::{GraphQueryRuntime, StorageRuntime};

/// Get knowledge graph with traversal from optional starting node.
///
/// # Implements
///
/// - **UC0101**: Explore Entity Neighborhood
/// - **FEAT0601**: Knowledge Graph Visualization
///
/// # Enforces
///
/// - **BR0201**: Tenant isolation (filters by workspace)
/// - **BR0009**: Node limit enforcement via `max_nodes`
#[utoipa::path(
    get,
    path = "/api/v1/graph",
    tag = "Graph",
    params(
        ("start_node" = Option<String>, Query, description = "Starting node ID"),
        ("depth" = usize, Query, description = "Max traversal depth"),
        ("max_nodes" = usize, Query, description = "Max nodes to return")
    ),
    responses(
        (status = 200, description = "Graph retrieved", body = KnowledgeGraphResponse)
    )
)]
pub async fn get_graph(
    State(storage): State<StorageRuntime>,
    State(graph): State<GraphQueryRuntime>,
    tenant_ctx: TenantContext,
    Query(params): Query<GraphQueryParams>,
) -> ApiResult<Json<KnowledgeGraphResponse>> {
    let request_start = std::time::Instant::now();

    // WHY: Defense in depth - clamp params to safe ranges even if client sends invalid values
    let params = params.validated();

    debug!(
        tenant_id = ?tenant_ctx.tenant_id,
        workspace_id = ?tenant_ctx.workspace_id,
        "Getting graph with tenant context"
    );

    // SECURITY: Enforce strict tenant context requirement - NO EXCEPTIONS
    // This matches the strict filtering in entities.rs and relationships.rs (commit d11edba8)
    if !has_full_tenant_context(&tenant_ctx) {
        warn_missing_tenant_context(&tenant_ctx, "get_graph");
        return Ok(Json(empty_graph_response()));
    }

    let _materialize_guard = admit_graph_materialization(&graph)?;

    let (nodes, edges, bfs_truncated) = if let Some(start) = &params.start_node {
        let start = start.clone();
        let depth = params.depth;
        let max_nodes = params.max_nodes;
        let scoped = tenant_ctx.tenant_id.is_some() && tenant_ctx.workspace_id.is_some();
        let tenant_for_kg = tenant_ctx.tenant_id.clone();
        let workspace_for_kg = tenant_ctx.workspace_id.clone();
        let graph_storage = storage.graph_storage.clone();
        let kg = run_timed_graph_query(&graph.budget, "knowledge_graph", async move {
            graph_storage
                .get_knowledge_graph(
                    &start,
                    depth,
                    max_nodes,
                    tenant_for_kg.as_deref(),
                    workspace_for_kg.as_deref(),
                )
                .await
        })
        .await?;

        let filtered_nodes: Vec<_> = kg
            .nodes
            .into_iter()
            .filter(|n| !scoped || properties_match_tenant_context(&n.properties, &tenant_ctx))
            .collect();

        let node_ids: Vec<String> = filtered_nodes.iter().map(|n| n.id.clone()).collect();
        let degree_map = degrees_breakdown_batch(&storage.graph_storage, &node_ids).await;

        let nodes: Vec<GraphNodeResponse> = filtered_nodes
            .into_iter()
            .map(|n| {
                let degree = degree_map
                    .get(&n.id)
                    .copied()
                    .unwrap_or_else(|| DegreeBreakdown::from_total(0));
                graph_node_response(&n, degree)
            })
            .collect();

        // Also filter edges by tenant context
        let node_id_set: std::collections::HashSet<_> = nodes.iter().map(|n| &n.id).collect();
        let edges: Vec<GraphEdgeResponse> = kg
            .edges
            .into_iter()
            .filter(|e| {
                (!scoped || properties_match_tenant_context(&e.properties, &tenant_ctx))
                    && node_id_set.contains(&e.source)
                    && node_id_set.contains(&e.target)
            })
            .map(edge_response)
            .collect();

        (nodes, edges, kg.is_truncated)
    } else {
        let max_nodes = params.max_nodes;
        let tenant_id = tenant_ctx.tenant_id.clone();
        let workspace_id = tenant_ctx.workspace_id.clone();
        let graph_storage = storage.graph_storage.clone();
        let nodes_with_degrees =
            run_timed_graph_query(&graph.budget, "popular_nodes", async move {
                graph_storage
                    .get_popular_nodes_with_degree(
                        max_nodes,
                        None,
                        None,
                        tenant_id.as_deref(),
                        workspace_id.as_deref(),
                    )
                    .await
            })
            .await?;

        let node_ids: Vec<String> = nodes_with_degrees
            .iter()
            .map(|(n, _)| n.id.clone())
            .collect();
        let degree_map = degrees_breakdown_batch(&storage.graph_storage, &node_ids).await;

        let nodes: Vec<GraphNodeResponse> = nodes_with_degrees
            .into_iter()
            .map(|(node, total)| {
                let degree = degree_map
                    .get(&node.id)
                    .copied()
                    .unwrap_or_else(|| DegreeBreakdown::from_total(total));
                graph_node_response(&node, degree)
            })
            .collect();

        // OPTIMIZED: Use filtered edge query instead of get_all_edges
        let node_ids: Vec<String> = nodes.iter().map(|n| n.id.clone()).collect();
        let tenant_for_edges = tenant_ctx.tenant_id.clone();
        let workspace_for_edges = tenant_ctx.workspace_id.clone();
        let graph_storage_edges = storage.graph_storage.clone();
        let filtered_edges =
            run_timed_graph_query(&graph.budget, "edges_for_node_set", async move {
                graph_storage_edges
                    .get_edges_for_node_set(
                        &node_ids,
                        tenant_for_edges.as_deref(),
                        workspace_for_edges.as_deref(),
                    )
                    .await
            })
            .await?;

        let edges: Vec<GraphEdgeResponse> =
            filtered_edges.into_iter().map(edge_response).collect();

        (nodes, edges, false)
    };

    // SPEC-155 B01: workspace-exact totals (not shared AGE reltuples).
    let (total_nodes, total_edges) =
        workspace_graph_totals(&storage.graph_storage, &tenant_ctx).await?;
    let is_truncated =
        graph_is_truncated(nodes.len(), total_nodes, params.max_nodes, bfs_truncated);

    let elapsed_ms = request_start.elapsed().as_millis();
    debug!(
        elapsed_ms,
        total_nodes,
        total_edges,
        node_count = nodes.len(),
        edge_count = edges.len(),
        "Graph query completed"
    );

    Ok(Json(KnowledgeGraphResponse {
        nodes,
        edges,
        is_truncated,
        total_nodes,
        total_edges,
        max_nodes: params.max_nodes,
    }))
}
