//! SPEC-155 W3 — shared graph DTO mapping + workspace totals (DRY).

use std::collections::HashMap;
use std::sync::Arc;

use edgequake_storage::traits::{GraphStorage, NodeListFilter};
use edgequake_storage::{GraphEdge, GraphNode};

use crate::error::{ApiError, ApiResult};
use crate::handlers::graph::graph_node_label;
use crate::handlers::graph_types::{
    CommunitySummary, DegreeBreakdown, GraphEdgeResponse, GraphFacetsResponse, GraphNodeResponse,
    TypeFacetCount,
};
use crate::handlers::isolation::properties_match_tenant_context;
use crate::middleware::TenantContext;

/// Cap for community/facet aggregation scans (CHEAP wave; no new tables).
const AGGREGATE_SCAN_LIMIT: usize = 5_000;

/// Workspace-exact node/edge totals (never shared AGE `reltuples`).
pub async fn workspace_graph_totals(
    graph: &Arc<dyn GraphStorage>,
    tenant_ctx: &TenantContext,
) -> ApiResult<(usize, usize)> {
    let Some(ws) = tenant_ctx.workspace_id_uuid() else {
        return Ok((0, 0));
    };
    let (nodes, edges) = tokio::join!(
        graph.node_count_by_workspace(&ws),
        graph.edge_count_by_workspace(&ws),
    );
    Ok((
        nodes.map_err(ApiError::from)?,
        edges.map_err(ApiError::from)?,
    ))
}

/// Truncation contract: returned set is a proper subset of workspace total,
/// or BFS itself reported truncation.
pub fn graph_is_truncated(
    returned: usize,
    total: usize,
    _max_nodes: usize,
    bfs_truncated: bool,
) -> bool {
    bfs_truncated || returned < total
}

pub fn community_id_from_node(node: &GraphNode) -> Option<String> {
    node.properties.get("community_id").and_then(|v| {
        v.as_str()
            .map(str::to_string)
            .or_else(|| v.as_u64().map(|n| n.to_string()))
            .or_else(|| v.as_i64().map(|n| n.to_string()))
    })
}

/// Map storage node + degree breakdown → API DTO.
pub fn graph_node_response(node: &GraphNode, degree: DegreeBreakdown) -> GraphNodeResponse {
    let props_value = serde_json::to_value(&node.properties).unwrap_or_default();
    GraphNodeResponse {
        id: node.id.clone(),
        label: graph_node_label(node),
        node_type: node
            .properties
            .get("entity_type")
            .and_then(|v| v.as_str())
            .unwrap_or("UNKNOWN")
            .to_string(),
        description: node
            .properties
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        degree,
        community_id: community_id_from_node(node),
        properties: props_value,
    }
}

/// Resolve `{in,out,total}` for a set of node IDs (falls back to total-only).
pub async fn degrees_breakdown_batch(
    graph: &Arc<dyn GraphStorage>,
    node_ids: &[String],
) -> HashMap<String, DegreeBreakdown> {
    if node_ids.is_empty() {
        return HashMap::new();
    }

    if let Ok(rows) = graph.get_nodes_with_degrees_batch(node_ids).await {
        return rows
            .into_iter()
            .map(|(node, in_deg, out_deg)| (node.id, DegreeBreakdown::from_in_out(in_deg, out_deg)))
            .collect();
    }

    graph
        .node_degrees_batch(node_ids)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|(id, total)| (id, DegreeBreakdown::from_total(total)))
        .collect()
}

/// Batch degrees filtered to the caller's workspace (tenant isolation).
pub async fn degrees_breakdown_for_workspace(
    graph: &Arc<dyn GraphStorage>,
    tenant_ctx: &TenantContext,
    node_ids: &[String],
) -> ApiResult<Vec<(String, DegreeBreakdown)>> {
    if node_ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut scoped_ids = Vec::with_capacity(node_ids.len());
    for id in node_ids {
        match graph.get_node(id).await {
            Ok(Some(node)) if properties_match_tenant_context(&node.properties, tenant_ctx) => {
                scoped_ids.push(id.clone());
            }
            _ => {}
        }
    }

    let map = degrees_breakdown_batch(graph, &scoped_ids).await;
    Ok(scoped_ids
        .into_iter()
        .map(|id| {
            let deg = map
                .get(&id)
                .copied()
                .unwrap_or_else(|| DegreeBreakdown::from_total(0));
            (id, deg)
        })
        .collect())
}

pub fn edge_response(edge: GraphEdge) -> GraphEdgeResponse {
    GraphEdgeResponse::from_storage_edge(edge)
}

/// Aggregate communities + type facets for a workspace (bounded scan).
pub async fn workspace_communities_and_facets(
    graph: &Arc<dyn GraphStorage>,
    tenant_ctx: &TenantContext,
) -> ApiResult<(Vec<CommunitySummary>, GraphFacetsResponse)> {
    let filter = NodeListFilter {
        tenant_id: tenant_ctx.tenant_id.clone(),
        workspace_id: tenant_ctx.workspace_id.clone(),
        entity_type: None,
        search: None,
        community_ids: None,
    };

    let page = graph
        .list_nodes_filtered(&filter, 0, AGGREGATE_SCAN_LIMIT)
        .await
        .map_err(ApiError::from)?;

    let mut community_sizes: HashMap<String, usize> = HashMap::new();
    let mut entity_types: HashMap<String, usize> = HashMap::new();

    for node in &page.items {
        if let Some(cid) = community_id_from_node(node) {
            *community_sizes.entry(cid).or_insert(0) += 1;
        }
        let et = node
            .properties
            .get("entity_type")
            .and_then(|v| v.as_str())
            .unwrap_or("UNKNOWN");
        *entity_types.entry(et.to_string()).or_insert(0) += 1;
    }

    let edge_filter = edgequake_storage::traits::EdgeListFilter {
        tenant_id: tenant_ctx.tenant_id.clone(),
        workspace_id: tenant_ctx.workspace_id.clone(),
        relationship_type: None,
    };
    let edges = graph
        .list_edges_filtered(&edge_filter, 0, AGGREGATE_SCAN_LIMIT)
        .await
        .map_err(ApiError::from)?;

    let mut relationship_types: HashMap<String, usize> = HashMap::new();
    for edge in &edges.items {
        let rt = edge
            .properties
            .get("relationship_type")
            .or_else(|| edge.properties.get("relation_type"))
            .and_then(|v| v.as_str())
            .unwrap_or("RELATED_TO");
        *relationship_types.entry(rt.to_string()).or_insert(0) += 1;
    }

    let mut communities: Vec<CommunitySummary> = community_sizes
        .into_iter()
        .map(|(id, size)| CommunitySummary {
            id,
            size,
            label: None,
            modularity: None,
        })
        .collect();
    communities.sort_by(|a, b| b.size.cmp(&a.size).then_with(|| a.id.cmp(&b.id)));

    let mut entity_type_counts: Vec<TypeFacetCount> = entity_types
        .into_iter()
        .map(|(name, count)| TypeFacetCount { name, count })
        .collect();
    entity_type_counts.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.name.cmp(&b.name)));

    let mut relationship_type_counts: Vec<TypeFacetCount> = relationship_types
        .into_iter()
        .map(|(name, count)| TypeFacetCount { name, count })
        .collect();
    relationship_type_counts
        .sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.name.cmp(&b.name)));

    Ok((
        communities,
        GraphFacetsResponse {
            entity_types: entity_type_counts,
            relationship_types: relationship_type_counts,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncation_when_returned_less_than_total() {
        assert!(graph_is_truncated(10, 100, 50, false));
        assert!(!graph_is_truncated(100, 100, 200, false));
        assert!(graph_is_truncated(50, 100, 50, false));
        assert!(graph_is_truncated(10, 10, 50, true));
    }

    #[test]
    fn community_id_parses_string_and_number() {
        let mut props = std::collections::HashMap::new();
        props.insert("community_id".into(), serde_json::json!(7));
        let n = GraphNode {
            id: "x".into(),
            properties: props,
        };
        assert_eq!(community_id_from_node(&n), Some("7".into()));

        let mut props2 = std::collections::HashMap::new();
        props2.insert("community_id".into(), serde_json::json!("c1"));
        let n2 = GraphNode {
            id: "y".into(),
            properties: props2,
        };
        assert_eq!(community_id_from_node(&n2), Some("c1".into()));
    }
}
