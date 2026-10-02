//! Graph handler DTOs and request/response types.
//!
//! This module contains all Data Transfer Objects (DTOs) for graph operations,
//! extracted from the main graph.rs handler for better modularity.

use edgequake_storage::GraphEdge;
use serde::de::{self, Deserializer, Visitor};
use serde::{Deserialize, Serialize};
use std::fmt;
use utoipa::ToSchema;

// ============================================================================
// Degree SSOT (SPEC-155 W3)
// ============================================================================

/// Directional degree breakdown — always serialized as `{in, out, total}`.
///
/// Bare JSON numbers still deserialize as `total` for one-release compat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
pub struct DegreeBreakdown {
    /// Incoming edge count.
    #[serde(rename = "in")]
    pub in_degree: u64,
    /// Outgoing edge count.
    pub out: u64,
    /// `in + out` (self-loops counted once per product decision).
    pub total: u64,
}

impl DegreeBreakdown {
    pub fn from_total(total: usize) -> Self {
        let total = total as u64;
        Self {
            in_degree: 0,
            out: 0,
            total,
        }
    }

    pub fn from_in_out(in_degree: usize, out_degree: usize) -> Self {
        let in_degree = in_degree as u64;
        let out = out_degree as u64;
        Self {
            in_degree,
            out,
            total: in_degree.saturating_add(out),
        }
    }

    pub fn total_usize(self) -> usize {
        self.total as usize
    }
}

impl<'de> Deserialize<'de> for DegreeBreakdown {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct DegreeVisitor;

        impl<'de> Visitor<'de> for DegreeVisitor {
            type Value = DegreeBreakdown;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a degree object {in,out,total} or a bare number (total)")
            }

            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
                Ok(DegreeBreakdown::from_total(v as usize))
            }

            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
                if v < 0 {
                    return Err(E::custom("degree must be non-negative"));
                }
                Ok(DegreeBreakdown::from_total(v as usize))
            }

            fn visit_map<A: de::MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut in_degree: Option<u64> = None;
                let mut out: Option<u64> = None;
                let mut total: Option<u64> = None;
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "in" => in_degree = Some(map.next_value()?),
                        "out" => out = Some(map.next_value()?),
                        "total" => total = Some(map.next_value()?),
                        _ => {
                            let _: de::IgnoredAny = map.next_value()?;
                        }
                    }
                }
                let in_degree = in_degree.unwrap_or(0);
                let out = out.unwrap_or(0);
                let total = total.unwrap_or_else(|| in_degree.saturating_add(out));
                Ok(DegreeBreakdown {
                    in_degree,
                    out,
                    total,
                })
            }
        }

        deserializer.deserialize_any(DegreeVisitor)
    }
}

// ============================================================================
// Graph Core DTOs
// ============================================================================

/// Graph node response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GraphNodeResponse {
    /// Node ID.
    pub id: String,

    /// Node label/name.
    pub label: String,

    /// Node type.
    pub node_type: String,

    /// Node description.
    pub description: String,

    /// Degree SSOT `{in, out, total}` (SPEC-155).
    pub degree: DegreeBreakdown,

    /// Index-time Louvain community label when present (SPEC-155 B07).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub community_id: Option<String>,

    /// Additional properties.
    pub properties: serde_json::Value,
}

/// Graph edge response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GraphEdgeResponse {
    /// Stable multigraph edge id including relation type (SPEC-155 B06).
    pub id: String,

    /// Source node ID.
    pub source: String,

    /// Target node ID.
    pub target: String,

    /// Relationship / edge type.
    /// WHY: field renamed from `edge_type` to match the frontend `GraphEdge.relationship_type`
    /// field expected by graph-renderer.tsx — the mismatch caused edge labels to always be
    /// `undefined` in the browser even with `forceLabel: true` enabled.
    pub relationship_type: String,

    /// Relationship description when present.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,

    /// Keywords (comma-joined or JSON array flattened).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,

    /// Edge weight.
    pub weight: f32,

    /// Additional properties.
    pub properties: serde_json::Value,
}

impl GraphEdgeResponse {
    /// Stable id: `{source}|{relation_type}|{target}` (multigraph-safe).
    pub fn stable_id(source: &str, relation_type: &str, target: &str) -> String {
        format!("{source}|{relation_type}|{target}")
    }

    fn extract_keywords(props: &serde_json::Map<String, serde_json::Value>) -> Vec<String> {
        match props.get("keywords") {
            Some(serde_json::Value::Array(arr)) => arr
                .iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .filter(|s| !s.is_empty())
                .collect(),
            Some(serde_json::Value::String(s)) => s
                .split(',')
                .map(str::trim)
                .filter(|p| !p.is_empty())
                .map(str::to_string)
                .collect(),
            _ => Vec::new(),
        }
    }

    /// SSOT mapping from storage [`GraphEdge`] to API DTO (ARCH-006 / SPEC-027 / SPEC-155).
    pub fn from_storage_edge(edge: GraphEdge) -> Self {
        let relationship_type = edge
            .properties
            .get("relationship_type")
            .or_else(|| edge.properties.get("relation_type"))
            .and_then(|v| v.as_str())
            .unwrap_or("RELATED_TO")
            .to_string();
        let description = edge
            .properties
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let props_value = serde_json::to_value(&edge.properties).unwrap_or_default();
        let keywords = props_value
            .as_object()
            .map(Self::extract_keywords)
            .unwrap_or_default();
        let id = edge
            .properties
            .get("relationship_id")
            .or_else(|| edge.properties.get("id"))
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .unwrap_or_else(|| Self::stable_id(&edge.source, &relationship_type, &edge.target));
        Self {
            id,
            source: edge.source,
            target: edge.target,
            relationship_type,
            description,
            keywords,
            weight: edge
                .properties
                .get("weight")
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0) as f32,
            properties: props_value,
        }
    }
}

/// Knowledge graph response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct KnowledgeGraphResponse {
    /// Nodes in the graph.
    pub nodes: Vec<GraphNodeResponse>,

    /// Edges in the graph.
    pub edges: Vec<GraphEdgeResponse>,

    /// Whether the graph was truncated.
    pub is_truncated: bool,

    /// Workspace-exact total node count (SPEC-155 B01).
    pub total_nodes: usize,

    /// Workspace-exact total edge count (SPEC-155 B01).
    pub total_edges: usize,

    /// Echo of request `max_nodes` clamp (SPEC-155 truncation contract).
    pub max_nodes: usize,
}

/// Graph query parameters.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct GraphQueryParams {
    /// Starting node ID.
    pub start_node: Option<String>,

    /// Maximum traversal depth.
    #[serde(default = "default_depth")]
    pub depth: usize,

    /// Maximum nodes to return.
    #[serde(default = "default_max_nodes")]
    pub max_nodes: usize,
}

/// SPEC-006: DRY re-export from `edgequake_core::resource` SSOT.
pub use edgequake_core::{MAX_GRAPH_DEPTH, MAX_GRAPH_NODES};

/// Default traversal depth.
pub fn default_depth() -> usize {
    2
}

/// Default max nodes.
pub fn default_max_nodes() -> usize {
    100
}

impl GraphQueryParams {
    /// WHY: Defense in depth - clamp parameters to safe ranges even if client sends invalid values
    /// This ensures server stability regardless of frontend validation.
    pub fn validated(mut self) -> Self {
        self.max_nodes = self.max_nodes.clamp(1, MAX_GRAPH_NODES);
        self.depth = self.depth.clamp(1, MAX_GRAPH_DEPTH);
        self
    }
}

// ============================================================================
// Label Search DTOs
// ============================================================================

/// Search labels query.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SearchLabelsQuery {
    /// Search query.
    pub q: String,

    /// Maximum results.
    #[serde(default = "graph_default_limit")]
    pub limit: usize,
}

/// Default search limit for graph operations.
pub fn graph_default_limit() -> usize {
    20
}

/// Search labels response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SearchLabelsResponse {
    /// Matching labels.
    pub labels: Vec<String>,
}

// ============================================================================
// Search Nodes DTOs (Full Node Search)
// ============================================================================

/// Search nodes query - returns full node data with degree.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SearchNodesQuery {
    /// Search query string (searches label and description).
    pub q: String,

    /// Maximum results to return.
    #[serde(default = "default_search_nodes_limit")]
    pub limit: usize,

    /// Whether to include neighbors of matching nodes.
    #[serde(default)]
    pub include_neighbors: bool,

    /// Neighbor depth when include_neighbors is true.
    #[serde(default = "default_neighbor_depth")]
    pub neighbor_depth: usize,

    /// Filter by entity type (optional).
    pub entity_type: Option<String>,
}

/// Default search nodes limit.
pub fn default_search_nodes_limit() -> usize {
    50
}

/// Default neighbor depth for search.
pub fn default_neighbor_depth() -> usize {
    1
}

/// Search nodes response - full node data for graph display.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SearchNodesResponse {
    /// Matching nodes with full data.
    pub nodes: Vec<GraphNodeResponse>,

    /// Edges connecting the returned nodes.
    pub edges: Vec<GraphEdgeResponse>,

    /// Total matches in database (before limit).
    pub total_matches: usize,

    /// Whether results were truncated.
    pub is_truncated: bool,
}

// ============================================================================
// Popular Labels DTOs
// ============================================================================

/// Query parameters for popular labels.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct PopularLabelsQuery {
    /// Maximum number of labels to return.
    #[serde(default = "default_popular_limit")]
    pub limit: usize,

    /// Minimum degree (connections) to include.
    #[serde(default)]
    pub min_degree: Option<usize>,

    /// Filter by entity type.
    #[serde(default)]
    pub entity_type: Option<String>,
}

/// Default popular labels limit.
pub fn default_popular_limit() -> usize {
    50
}

/// Popular label with metadata.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PopularLabel {
    /// Label/entity name.
    pub label: String,

    /// Entity type.
    pub entity_type: String,

    /// Degree SSOT `{in, out, total}`.
    pub degree: DegreeBreakdown,

    /// Brief description.
    pub description: String,
}

/// Response with popular labels.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PopularLabelsResponse {
    /// List of popular labels sorted by degree.
    pub labels: Vec<PopularLabel>,

    /// Workspace-exact entity count in graph.
    pub total_entities: usize,
}

// ============================================================================
// Batch Operations DTOs
// ============================================================================

/// Request body for batch degree query.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct BatchDegreeRequest {
    /// List of node IDs to query.
    pub node_ids: Vec<String>,
}

/// Response for a single node degree.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct NodeDegree {
    /// Node ID.
    pub node_id: String,

    /// Degree SSOT `{in, out, total}` (bare number still accepted on deserialize via DegreeBreakdown).
    pub degree: DegreeBreakdown,
}

/// Response for batch degree query.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BatchDegreeResponse {
    /// Degrees for each requested node.
    pub degrees: Vec<NodeDegree>,

    /// Number of nodes queried.
    pub count: usize,
}

// ============================================================================
// Communities & Facets (SPEC-155 W3)
// ============================================================================

/// Community summary for Graph Studio legend / hulls.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CommunitySummary {
    pub id: String,
    pub size: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modularity: Option<f64>,
}

/// Response for `GET /graph/communities`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GraphCommunitiesResponse {
    pub communities: Vec<CommunitySummary>,
}

/// Name/count facet entry.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TypeFacetCount {
    pub name: String,
    pub count: usize,
}

/// Response for `GET /graph/facets`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GraphFacetsResponse {
    pub entity_types: Vec<TypeFacetCount>,
    pub relationship_types: Vec<TypeFacetCount>,
}

// ============================================================================
// Streaming Graph DTOs
// ============================================================================

/// Query parameters for streaming graph endpoint.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct GraphStreamQueryParams {
    /// Starting node ID for traversal.
    pub start_node: Option<String>,

    /// Maximum nodes to return.
    #[serde(default = "default_stream_max_nodes")]
    pub max_nodes: usize,

    /// Batch size for streaming (how many nodes per chunk).
    #[serde(default = "default_stream_batch_size")]
    pub batch_size: usize,
}

impl GraphStreamQueryParams {
    /// WHY: Defense in depth - clamp streaming params to safe ranges
    pub fn validated(mut self) -> Self {
        self.max_nodes = self.max_nodes.clamp(1, MAX_GRAPH_NODES);
        self.batch_size = self.batch_size.clamp(10, 100);
        self
    }
}

/// Default streaming depth.
pub fn default_stream_depth() -> usize {
    2
}

/// Default batch size for streaming.
pub fn default_batch_size() -> usize {
    50
}

/// Default stream max nodes (for original SSE endpoint).
pub fn default_stream_max_nodes() -> usize {
    200
}

/// Default stream batch size.
pub fn default_stream_batch_size() -> usize {
    50
}

// ============================================================================
// Streaming Event Types
// ============================================================================

/// Events sent during graph streaming.
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(tag = "type")]
pub enum GraphStreamEvent {
    /// Initial metadata about the graph.
    #[serde(rename = "metadata")]
    Metadata {
        /// Total nodes in graph (workspace-exact).
        total_nodes: usize,
        /// Total edges in graph (workspace-exact).
        total_edges: usize,
        /// Nodes to be streamed.
        nodes_to_stream: usize,
        /// Edges to be streamed (estimated).
        edges_to_stream: usize,
        /// True when streamed set is a proper subset of workspace totals (SPEC-155).
        #[serde(default)]
        is_truncated: bool,
    },

    /// Batch of nodes.
    #[serde(rename = "nodes")]
    Nodes {
        /// Current batch number.
        batch: usize,
        /// Total batches expected.
        total_batches: usize,
        /// Nodes in this batch.
        nodes: Vec<GraphNodeResponse>,
    },

    /// Batch of edges.
    #[serde(rename = "edges")]
    Edges {
        /// Edges in this batch.
        edges: Vec<GraphEdgeResponse>,
    },

    /// Stream complete.
    #[serde(rename = "done")]
    Done {
        /// Total nodes streamed.
        nodes_count: usize,
        /// Total edges streamed.
        edges_count: usize,
        /// Duration in milliseconds.
        duration_ms: u64,
    },

    /// Error during streaming.
    #[serde(rename = "error")]
    Error {
        /// Error message.
        message: String,
        /// Machine-readable reason code for transient congestion
        /// (e.g. `"transient_congestion"`) so the client can retry
        /// appropriately. Absent for non-transient errors.
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
        /// Seconds the client should wait before retrying, when the
        /// error is transient congestion. Mirrors the HTTP 503
        /// `retry_after_secs` so the SSE path is not lossy vs the
        /// REST path (SPEC-021 R2).
        #[serde(skip_serializing_if = "Option::is_none")]
        retry_after_secs: Option<u64>,
    },
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_node_response_serialization() {
        let node = GraphNodeResponse {
            id: "test_node".to_string(),
            label: "Test Node".to_string(),
            node_type: "PERSON".to_string(),
            description: "A test node".to_string(),
            degree: DegreeBreakdown::from_in_out(2, 3),
            community_id: Some("1".into()),
            properties: serde_json::json!({"custom": "value"}),
        };

        let json = serde_json::to_string(&node).unwrap();
        assert!(json.contains("test_node"));
        assert!(json.contains("PERSON"));
        assert!(json.contains("\"in\":2"));
        assert!(json.contains("\"out\":3"));
        assert!(json.contains("\"total\":5"));
        assert!(json.contains("\"community_id\":\"1\""));
    }

    #[test]
    fn test_degree_breakdown_accepts_bare_number() {
        let deg: DegreeBreakdown = serde_json::from_str("7").unwrap();
        assert_eq!(deg.total, 7);
        assert_eq!(deg.in_degree, 0);
        assert_eq!(deg.out, 0);
    }

    #[test]
    fn test_graph_edge_response_serialization() {
        let edge = GraphEdgeResponse {
            id: "node_a|RELATED_TO|node_b".to_string(),
            source: "node_a".to_string(),
            target: "node_b".to_string(),
            relationship_type: "RELATED_TO".to_string(),
            description: "linked".to_string(),
            keywords: vec!["k1".into()],
            weight: 0.8,
            properties: serde_json::json!({}),
        };

        let json = serde_json::to_string(&edge).unwrap();
        assert!(json.contains("node_a"));
        assert!(json.contains("RELATED_TO"));
        assert!(json.contains("node_a|RELATED_TO|node_b"));
    }

    #[test]
    fn test_graph_query_params_defaults() {
        let json = r#"{"start_node": "test"}"#;
        let params: GraphQueryParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.depth, 2);
        assert_eq!(params.max_nodes, 100);
    }

    #[test]
    fn test_search_labels_query_defaults() {
        let json = r#"{"q": "test"}"#;
        let query: SearchLabelsQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.limit, 20);
    }

    #[test]
    fn test_popular_labels_query_defaults() {
        let json = r#"{}"#;
        let query: PopularLabelsQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.limit, 50);
        assert!(query.min_degree.is_none());
        assert!(query.entity_type.is_none());
    }

    #[test]
    fn test_batch_degree_request_deserialization() {
        let json = r#"{"node_ids": ["a", "b", "c"]}"#;
        let request: BatchDegreeRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.node_ids.len(), 3);
    }

    #[test]
    fn test_knowledge_graph_response_serialization() {
        let response = KnowledgeGraphResponse {
            nodes: vec![],
            edges: vec![],
            is_truncated: false,
            total_nodes: 10,
            total_edges: 20,
            max_nodes: 100,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("total_nodes"));
        assert!(json.contains("is_truncated"));
        assert!(json.contains("max_nodes"));
    }

    #[test]
    fn test_edge_stable_id_distinguishes_multigraph() {
        let id_a = GraphEdgeResponse::stable_id("A", "WORKS_AT", "B");
        let id_b = GraphEdgeResponse::stable_id("A", "KNOWS", "B");
        assert_ne!(id_a, id_b);
    }

    #[test]
    fn test_graph_stream_query_params_defaults() {
        let json = r#"{}"#;
        let params: GraphStreamQueryParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.max_nodes, 200);
        assert_eq!(params.batch_size, 50);
    }

    #[test]
    fn test_graph_stream_event_serialization() {
        let event = GraphStreamEvent::Metadata {
            total_nodes: 100,
            total_edges: 200,
            nodes_to_stream: 50,
            edges_to_stream: 100,
            is_truncated: true,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("metadata"));
        assert!(json.contains("total_nodes"));
    }

    /// SPEC-021 R2: the SSE error event must carry reason + retry_after_secs
    /// for transient congestion so the client can retry, and must omit those
    /// fields (skip_serializing_if) for non-transient errors to stay compact.
    #[test]
    fn test_graph_stream_error_event_serialization_transient_and_non_transient() {
        let transient = GraphStreamEvent::Error {
            message: "Graph materialization capacity reached".into(),
            reason: Some("transient_congestion".into()),
            retry_after_secs: Some(5),
        };
        let json = serde_json::to_string(&transient).unwrap();
        assert!(json.contains("\"error\""));
        assert!(json.contains("\"reason\":\"transient_congestion\""));
        assert!(json.contains("\"retry_after_secs\":5"));

        let non_transient = GraphStreamEvent::Error {
            message: "Failed to fetch edges: boom".into(),
            reason: None,
            retry_after_secs: None,
        };
        let json = serde_json::to_string(&non_transient).unwrap();
        assert!(json.contains("\"error\""));
        assert!(!json.contains("reason"));
        assert!(!json.contains("retry_after_secs"));
    }
}
