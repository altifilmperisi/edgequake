//! SSE streaming handler for progressive graph data loading.
//!
//! Contains: `stream_graph`.
//!
//! SPEC-155 W3: workspace-exact totals, optional `start_node` BFS, degree SSOT.

use axum::{
    extract::{Query, State},
    response::sse::Event,
    response::Response,
};
use futures::stream::StreamExt;
use std::convert::Infallible;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::debug;

use crate::error::{ApiError, TransientCongestion};
use crate::handlers::graph::graph_dto::{
    degrees_breakdown_batch, edge_response, graph_is_truncated, graph_node_response,
    workspace_graph_totals,
};
use crate::handlers::graph_types::*;
use crate::middleware::TenantContext;
use crate::services::{admit_graph_materialization, run_timed_graph_query};
use crate::state::{GraphQueryRuntime, StorageRuntime};
use crate::streaming::live_sse;

/// Stream graph data progressively via SSE.
///
/// This endpoint streams graph nodes and edges in batches, making it suitable
/// for very large graphs where loading everything at once would be too slow.
///
/// Events are sent in order:
/// 1. `metadata` - Initial graph statistics
/// 2. `nodes` - Multiple batches of nodes (batch_size per event)
/// 3. `edges` - Edges between streamed nodes
/// 4. `done` - Completion summary
#[utoipa::path(
    get,
    path = "/api/v1/graph/stream",
    tag = "Graph",
    params(
        ("start_node" = Option<String>, Query, description = "Starting node ID"),
        ("max_nodes" = usize, Query, description = "Max nodes to stream (default 200)"),
        ("batch_size" = usize, Query, description = "Nodes per batch (default 50)")
    ),
    responses(
        (status = 200, description = "SSE stream of GraphStreamEvent payloads",
            content(
                (GraphStreamEvent = "text/event-stream")
            )
        )
    )
)]
pub async fn stream_graph(
    State(storage): State<StorageRuntime>,
    State(graph): State<GraphQueryRuntime>,
    tenant_ctx: TenantContext,
    Query(params): Query<GraphStreamQueryParams>,
) -> Result<Response, ApiError> {
    // WHY: Defense in depth - clamp params to safe ranges even if client sends invalid values
    let params = params.validated();

    debug!(
        tenant_id = ?tenant_ctx.tenant_id,
        workspace_id = ?tenant_ctx.workspace_id,
        max_nodes = params.max_nodes,
        batch_size = params.batch_size,
        start_node = ?params.start_node,
        "Starting graph stream"
    );

    // Create channel for SSE events
    let (tx, rx) = mpsc::channel::<GraphStreamEvent>(100);

    // Clone for async task
    let graph_storage = storage.graph_storage.clone();
    let graph_clone = graph.clone();
    let params_clone = params.clone();
    let tenant_ctx_clone = tenant_ctx.clone();

    // Spawn background task for streaming
    tokio::spawn(async move {
        let start_time = std::time::Instant::now();

        let _materialize_guard = match admit_graph_materialization(&graph_clone) {
            Ok(guard) => guard,
            Err(ApiError::ServiceUnavailable {
                message,
                retry_after_secs,
            }) => {
                // WHY: reuse the single TransientCongestion SSOT so the SSE
                // error event carries the same reason + retry_after_secs as
                // the HTTP 503 would. Previously the SSE path sent only the
                // bare string and the client could not retry intelligently.
                let payload = TransientCongestion {
                    reason: "transient_congestion",
                    retry_after_secs,
                };
                let (msg, reason, retry) = payload.sse_error_fields(message);
                let _ = tx
                    .send(GraphStreamEvent::Error {
                        message: msg,
                        reason,
                        retry_after_secs: retry,
                    })
                    .await;
                return;
            }
            Err(other) => {
                let _ = tx
                    .send(GraphStreamEvent::Error {
                        message: other.to_string(),
                        reason: None,
                        retry_after_secs: None,
                    })
                    .await;
                return;
            }
        };

        debug!("About to query workspace totals + nodes");

        let max_nodes = params_clone.max_nodes;
        let tenant_id = tenant_ctx_clone.tenant_id.clone();
        let workspace_id = tenant_ctx_clone.workspace_id.clone();
        let graph_for_query = graph_clone.clone();
        let graph_for_totals = graph_storage.clone();
        let tenant_for_totals = tenant_ctx_clone.clone();
        let graph_for_materialize = graph_storage.clone();
        let graph_for_edges = graph_storage.clone();
        let start_node = params_clone.start_node.clone();

        let (totals_result, materialize_result) = tokio::join!(
            async move { workspace_graph_totals(&graph_for_totals, &tenant_for_totals).await },
            async move {
                run_timed_graph_query(&graph_for_query.budget, "graph_stream", async move {
                    if let Some(start) = start_node.as_deref() {
                        let kg = graph_for_materialize
                            .get_knowledge_graph(
                                start,
                                default_depth(),
                                max_nodes,
                                tenant_id.as_deref(),
                                workspace_id.as_deref(),
                            )
                            .await?;
                        let node_ids: Vec<String> =
                            kg.nodes.iter().map(|n| n.id.clone()).collect();
                        let degree_map =
                            degrees_breakdown_batch(&graph_for_materialize, &node_ids).await;
                        let nodes: Vec<GraphNodeResponse> = kg
                            .nodes
                            .iter()
                            .map(|n| {
                                let degree = degree_map
                                    .get(&n.id)
                                    .copied()
                                    .unwrap_or_else(|| DegreeBreakdown::from_total(0));
                                graph_node_response(n, degree)
                            })
                            .collect();
                        let edges: Vec<GraphEdgeResponse> =
                            kg.edges.into_iter().map(edge_response).collect();
                        Ok::<_, edgequake_storage::error::StorageError>((nodes, Some(edges)))
                    } else {
                        let popular = graph_for_materialize
                            .get_popular_nodes_with_degree(
                                max_nodes,
                                None,
                                None,
                                tenant_id.as_deref(),
                                workspace_id.as_deref(),
                            )
                            .await?;
                        let node_ids: Vec<String> =
                            popular.iter().map(|(n, _)| n.id.clone()).collect();
                        let degree_map =
                            degrees_breakdown_batch(&graph_for_materialize, &node_ids).await;
                        let nodes: Vec<GraphNodeResponse> = popular
                            .into_iter()
                            .map(|(node, total)| {
                                let degree = degree_map
                                    .get(&node.id)
                                    .copied()
                                    .unwrap_or_else(|| DegreeBreakdown::from_total(total));
                                graph_node_response(&node, degree)
                            })
                            .collect();
                        Ok::<_, edgequake_storage::error::StorageError>((nodes, None))
                    }
                })
                .await
            }
        );

        let (total_nodes, total_edges) = match totals_result {
            Ok(t) => t,
            Err(e) => {
                let _ = tx
                    .send(GraphStreamEvent::Error {
                        message: e.to_string(),
                        reason: None,
                        retry_after_secs: None,
                    })
                    .await;
                return;
            }
        };

        let (nodes, prefetched_edges) = match materialize_result {
            Ok(payload) => {
                debug!("Query succeeded with {} nodes", payload.0.len());
                payload
            }
            Err(e) => {
                let _ = tx
                    .send(GraphStreamEvent::Error {
                        message: e.to_string(),
                        reason: None,
                        retry_after_secs: None,
                    })
                    .await;
                return;
            }
        };

        // WHY release the guard here (SPEC-053 B2):
        // The materialization semaphore guards the DB-intensive initial fetch.
        // Once the data is in memory, streaming SSE events to the client uses
        // no additional DB connections. Holding the permit for the entire
        // streaming loop (seconds) starves concurrent search_nodes, traversal,
        // and popular_labels handlers.
        drop(_materialize_guard);

        let nodes_to_stream = nodes.len();
        let is_truncated =
            graph_is_truncated(nodes_to_stream, total_nodes, params_clone.max_nodes, false);
        let total_batches = nodes_to_stream.div_ceil(params_clone.batch_size);

        // Send metadata event
        if tx
            .send(GraphStreamEvent::Metadata {
                total_nodes,
                total_edges,
                nodes_to_stream,
                edges_to_stream: 0, // Will be determined after node streaming
                is_truncated,
            })
            .await
            .is_err()
        {
            return; // Client disconnected
        }

        // Collect all node IDs for edge fetching (popular path)
        let all_node_ids: Vec<String> = nodes.iter().map(|n| n.id.clone()).collect();

        // Stream nodes in batches
        for (batch_idx, chunk) in nodes.chunks(params_clone.batch_size).enumerate() {
            let batch_nodes: Vec<GraphNodeResponse> = chunk.to_vec();

            if tx
                .send(GraphStreamEvent::Nodes {
                    batch: batch_idx + 1,
                    total_batches,
                    nodes: batch_nodes,
                })
                .await
                .is_err()
            {
                return; // Client disconnected
            }

            // Small yield to prevent blocking
            tokio::task::yield_now().await;
        }

        // Edges: prefer BFS prefetch; otherwise batch-fetch for popular nodes.
        let edge_responses: Vec<GraphEdgeResponse> = if let Some(edges) = prefetched_edges {
            edges
        } else {
            match graph_for_edges
                .get_edges_for_node_set(
                    &all_node_ids,
                    tenant_ctx_clone.tenant_id.as_deref(),
                    tenant_ctx_clone.workspace_id.as_deref(),
                )
                .await
            {
                Ok(e) => e.into_iter().map(edge_response).collect(),
                Err(e) => {
                    let _ = tx
                        .send(GraphStreamEvent::Error {
                            message: format!("Failed to fetch edges: {}", e),
                            reason: None,
                            retry_after_secs: None,
                        })
                        .await;
                    return;
                }
            }
        };

        let edges_count = edge_responses.len();

        if tx
            .send(GraphStreamEvent::Edges {
                edges: edge_responses,
            })
            .await
            .is_err()
        {
            return;
        }

        // Send completion event
        let duration_ms = start_time.elapsed().as_millis() as u64;
        let _ = tx
            .send(GraphStreamEvent::Done {
                nodes_count: nodes_to_stream,
                edges_count,
                duration_ms,
            })
            .await;
    });

    // Convert channel to SSE stream
    let sse_stream = ReceiverStream::new(rx).map(|event| {
        let json = serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_string());
        Ok::<_, Infallible>(Event::default().data(json))
    });

    Ok(live_sse(sse_stream))
}
