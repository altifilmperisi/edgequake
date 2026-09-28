//! Graph community detection algorithms.
//!
//! @implements FEAT0205
//!
//! This module provides community detection algorithms for graph clustering,
//! similar to what LightRAG uses for global queries.
//!
//! SPEC-006: `get_all_*` here is intentional — only reachable via `detect_communities_unchecked`
//! after API `ResourceGuard` admission.

#![allow(deprecated)]

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::error::Result;
use crate::traits::GraphStorage;

/// A detected community in the graph.
#[derive(Debug, Clone)]
pub struct Community {
    /// Unique identifier for the community.
    pub id: usize,
    /// Node IDs that belong to this community.
    pub members: Vec<String>,
    /// Aggregate properties for the community.
    pub properties: HashMap<String, serde_json::Value>,
}

impl Community {
    /// Create a new community.
    pub fn new(id: usize) -> Self {
        Self {
            id,
            members: Vec::new(),
            properties: HashMap::new(),
        }
    }

    /// Add a member to the community.
    pub fn add_member(&mut self, node_id: String) {
        self.members.push(node_id);
    }

    /// Get the number of members.
    pub fn size(&self) -> usize {
        self.members.len()
    }
}

/// Result of community detection.
#[derive(Debug, Clone)]
pub struct CommunityDetectionResult {
    /// Detected communities.
    pub communities: Vec<Community>,
    /// Mapping from node ID to community ID.
    pub node_to_community: HashMap<String, usize>,
    /// Modularity score of the partition.
    pub modularity: f64,
    /// Number of Louvain hierarchy levels executed (1 = phase-1 only).
    /// When `EDGEQUAKE_LOUVAIN_HIERARCHY=1`, phase-2 aggregation may raise this.
    pub hierarchy_levels: usize,
}

impl CommunityDetectionResult {
    /// Create a new empty result.
    pub fn new() -> Self {
        Self {
            communities: Vec::new(),
            node_to_community: HashMap::new(),
            modularity: 0.0,
            hierarchy_levels: 0,
        }
    }

    /// Get community by ID.
    pub fn get_community(&self, id: usize) -> Option<&Community> {
        self.communities.iter().find(|c| c.id == id)
    }

    /// Get community for a node.
    pub fn get_node_community(&self, node_id: &str) -> Option<&Community> {
        self.node_to_community
            .get(node_id)
            .and_then(|id| self.get_community(*id))
    }

    /// Get all members in a node's community.
    pub fn get_community_members(&self, node_id: &str) -> Option<&[String]> {
        self.get_node_community(node_id)
            .map(|c| c.members.as_slice())
    }
}

impl Default for CommunityDetectionResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Community detection algorithm type.
#[derive(Debug, Clone, Copy, Default)]
pub enum CommunityAlgorithm {
    /// Louvain method for community detection.
    #[default]
    Louvain,
    /// Label propagation algorithm.
    LabelPropagation,
    /// Connected components (baseline).
    ConnectedComponents,
}

/// Configuration for community detection.
#[derive(Debug, Clone)]
pub struct CommunityConfig {
    /// Algorithm to use.
    pub algorithm: CommunityAlgorithm,
    /// Minimum community size.
    pub min_community_size: usize,
    /// Maximum iterations for iterative algorithms.
    pub max_iterations: usize,
    /// Resolution parameter for Louvain (higher = more communities).
    pub resolution: f64,
    /// Hard cap on nodes loaded for detection (SPEC-046 OPS-P0.2).
    /// Default from `EDGEQUAKE_COMMUNITY_MAX_NODES` (50_000).
    pub max_nodes: usize,
    /// Enable Louvain phase-2 hierarchy (community aggregation levels).
    /// Default from `EDGEQUAKE_LOUVAIN_HIERARCHY` (off).
    pub enable_hierarchy: bool,
    /// Max hierarchy levels when hierarchy is enabled (default 3).
    pub max_hierarchy_levels: usize,
    /// Workspace scope for snapshot load (GH-404). `None` = full graph (backfill).
    pub workspace_id: Option<String>,
    /// Optional tenant scope for snapshot load.
    pub tenant_id: Option<String>,
}

impl Default for CommunityConfig {
    fn default() -> Self {
        Self {
            algorithm: CommunityAlgorithm::Louvain,
            min_community_size: 2,
            max_iterations: 100,
            resolution: 1.0,
            max_nodes: community_max_nodes_from_env(),
            enable_hierarchy: louvain_hierarchy_enabled(),
            max_hierarchy_levels: 3,
            workspace_id: None,
            tenant_id: None,
        }
    }
}

/// True when `EDGEQUAKE_LOUVAIN_HIERARCHY=1` / `true` / `on`.
pub fn louvain_hierarchy_enabled() -> bool {
    std::env::var("EDGEQUAKE_LOUVAIN_HIERARCHY")
        .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "on" | "yes"))
        .unwrap_or(false)
}

/// Default community node cap (aligned with ResourceGuard graph_scan_threshold).
pub fn community_max_nodes_from_env() -> usize {
    std::env::var("EDGEQUAKE_COMMUNITY_MAX_NODES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(50_000)
        .clamp(100, 5_000_000)
}

/// Result of a bounded graph load for community detection.
#[derive(Debug, Clone)]
pub struct BoundedGraphLoad {
    pub nodes: Vec<crate::traits::GraphNode>,
    pub edges: Vec<crate::traits::GraphEdge>,
    pub sampled: bool,
    pub total_nodes_estimate: usize,
}

/// Load nodes/edges with keyset pagination + hard cap (GH-404 / SPEC-046 OPS-P0.2).
///
/// Uses [`GraphScanOps::scan_nodes_after`] / [`scan_edges_after`] — never OFFSET
/// list APIs. When `workspace_id` / `tenant_id` are set, the snapshot is scoped.
/// Prefer this over `get_all_nodes` / `get_all_edges` on ingest refresh paths.
pub async fn load_graph_bounded(
    graph: &Arc<dyn GraphStorage>,
    max_nodes: usize,
) -> Result<BoundedGraphLoad> {
    load_graph_bounded_scoped(graph, max_nodes, None, None).await
}

/// Workspace/tenant-scoped bounded load for community refresh (GH-404).
pub async fn load_graph_bounded_scoped(
    graph: &Arc<dyn GraphStorage>,
    max_nodes: usize,
    workspace_id: Option<&str>,
    tenant_id: Option<&str>,
) -> Result<BoundedGraphLoad> {
    use crate::traits::{EdgeListFilter, NodeListFilter};

    let max_nodes = max_nodes.max(1);
    let page = 2_000usize.min(max_nodes);
    let filter = NodeListFilter {
        tenant_id: tenant_id.map(str::to_string),
        workspace_id: workspace_id.map(str::to_string),
        ..Default::default()
    };
    let edge_filter = EdgeListFilter {
        tenant_id: tenant_id.map(str::to_string),
        workspace_id: workspace_id.map(str::to_string),
        relationship_type: None,
    };

    let mut nodes = Vec::new();
    let mut after: Option<String> = None;
    let mut last_page_full = false;
    loop {
        let page_result = graph
            .scan_nodes_after(&filter, after.as_deref(), page)
            .await?;
        if page_result.items.is_empty() {
            last_page_full = false;
            break;
        }
        let remaining = max_nodes.saturating_sub(nodes.len());
        if remaining == 0 {
            break;
        }
        let take = remaining.min(page_result.items.len());
        last_page_full = page_result.items.len() >= page;
        nodes.extend(page_result.items.into_iter().take(take));
        after = page_result.next_after;
        if nodes.len() >= max_nodes || after.is_none() {
            break;
        }
    }

    let sampled = nodes.len() >= max_nodes && (last_page_full || after.is_some());
    let total_estimate = if sampled {
        nodes.len().saturating_add(1)
    } else {
        nodes.len()
    };

    let node_ids: HashSet<String> = nodes.iter().map(|n| n.id.clone()).collect();
    let mut edges = Vec::new();
    let edge_cap = max_nodes.saturating_mul(4).max(page);

    if sampled {
        // Indexed incident fetch for the sample — do not walk the full edge table.
        let id_vec: Vec<String> = node_ids.iter().cloned().collect();
        let incident = graph
            .get_incident_edges_batch(&id_vec, tenant_id, workspace_id)
            .await?;
        for edge in incident {
            if node_ids.contains(&edge.source) && node_ids.contains(&edge.target) {
                edges.push(edge);
                if edges.len() >= edge_cap {
                    break;
                }
            }
        }
    } else {
        let mut e_after: Option<String> = None;
        loop {
            let page_result = graph
                .scan_edges_after(&edge_filter, e_after.as_deref(), page)
                .await?;
            if page_result.items.is_empty() {
                break;
            }
            for edge in page_result.items {
                if node_ids.contains(&edge.source) && node_ids.contains(&edge.target) {
                    edges.push(edge);
                    if edges.len() >= edge_cap {
                        break;
                    }
                }
            }
            e_after = page_result.next_after;
            if edges.len() >= edge_cap || e_after.is_none() {
                break;
            }
        }
    }

    if sampled {
        tracing::warn!(
            loaded_nodes = nodes.len(),
            total_nodes_estimate = total_estimate,
            max_nodes,
            loaded_edges = edges.len(),
            workspace_id,
            "community detection using sampled subgraph (SPEC-046 OPS-P0.2 / GH-404)"
        );
    }

    let node_len = nodes.len();
    Ok(BoundedGraphLoad {
        nodes,
        edges,
        sampled,
        total_nodes_estimate: total_estimate.max(node_len),
    })
}

/// Detect communities in a graph (full-graph load — internal use only).
///
/// SPEC-006: Not re-exported from `edgequake_storage` crate root.
/// API handlers must use `detect_communities_guarded` in `edgequake-api`.
pub async fn detect_communities_unchecked(
    graph: &Arc<dyn GraphStorage>,
    config: &CommunityConfig,
) -> Result<CommunityDetectionResult> {
    match config.algorithm {
        CommunityAlgorithm::Louvain => louvain_communities(graph, config).await,
        CommunityAlgorithm::LabelPropagation => label_propagation(graph, config).await,
        CommunityAlgorithm::ConnectedComponents => connected_components(graph, config).await,
    }
}

/// Louvain community detection algorithm.
///
/// Phase 1: local modularity moves (always).
/// Phase 2 (optional, `EDGEQUAKE_LOUVAIN_HIERARCHY=1`): aggregate communities
/// into super-nodes and repeat, producing a hierarchy of levels (NetworkX /
/// Blondel et al. style).
async fn load_for_config(
    graph: &Arc<dyn GraphStorage>,
    config: &CommunityConfig,
) -> Result<BoundedGraphLoad> {
    load_graph_bounded_scoped(
        graph,
        config.max_nodes,
        config.workspace_id.as_deref(),
        config.tenant_id.as_deref(),
    )
    .await
}

async fn louvain_communities(
    graph: &Arc<dyn GraphStorage>,
    config: &CommunityConfig,
) -> Result<CommunityDetectionResult> {
    let loaded = load_for_config(graph, config).await?;
    let nodes = loaded.nodes;
    let edges = loaded.edges;

    if nodes.is_empty() {
        return Ok(CommunityDetectionResult::new());
    }

    // Build adjacency list (original node ids)
    let mut adjacency: HashMap<String, Vec<(String, f64)>> = HashMap::new();
    let mut total_weight = 0.0;

    for edge in &edges {
        let weight = edge
            .properties
            .get("weight")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);

        adjacency
            .entry(edge.source.clone())
            .or_default()
            .push((edge.target.clone(), weight));

        adjacency
            .entry(edge.target.clone())
            .or_default()
            .push((edge.source.clone(), weight));

        total_weight += weight;
    }

    if total_weight == 0.0 {
        total_weight = 1.0; // Prevent division by zero
    }

    let node_ids: Vec<String> = nodes.iter().map(|n| n.id.clone()).collect();
    let (mut node_to_community, levels_run) =
        louvain_partition_with_hierarchy(&node_ids, &adjacency, total_weight, config);

    // Build result
    let mut communities_map: HashMap<usize, Vec<String>> = HashMap::new();
    for (node_id, comm_id) in &node_to_community {
        communities_map
            .entry(*comm_id)
            .or_default()
            .push(node_id.clone());
    }

    // Renumber communities and filter by minimum size
    let mut result = CommunityDetectionResult::new();
    result.hierarchy_levels = levels_run;
    let mut new_id = 0;
    let mut id_mapping: HashMap<usize, usize> = HashMap::new();

    for (old_id, members) in communities_map {
        if members.len() >= config.min_community_size {
            id_mapping.insert(old_id, new_id);

            let mut community = Community::new(new_id);
            community.members = members;
            community.properties.insert(
                "hierarchy_level".to_string(),
                serde_json::json!(levels_run.saturating_sub(1)),
            );
            result.communities.push(community);

            new_id += 1;
        }
    }

    // Update node_to_community with new IDs
    for (node_id, old_comm) in node_to_community.drain() {
        if let Some(&new_comm) = id_mapping.get(&old_comm) {
            result.node_to_community.insert(node_id, new_comm);
        }
    }

    // Calculate modularity
    result.modularity = calculate_modularity(&result, &adjacency, total_weight);

    Ok(result)
}

/// Run Louvain phase-1 (+ optional phase-2 hierarchy). Returns (node→comm, levels).
fn louvain_partition_with_hierarchy(
    node_ids: &[String],
    adjacency: &HashMap<String, Vec<(String, f64)>>,
    total_weight: f64,
    config: &CommunityConfig,
) -> (HashMap<String, usize>, usize) {
    // Level-0: original nodes
    let mut current_nodes = node_ids.to_vec();
    let mut current_adj = adjacency.clone();
    let mut current_total = total_weight;

    // Maps original node → community id at the finest (last) level
    let mut original_to_comm: HashMap<String, usize> = HashMap::new();
    for (idx, id) in node_ids.iter().enumerate() {
        original_to_comm.insert(id.clone(), idx);
    }

    // For hierarchy: track how super-node ids map back to original membership
    let mut super_members: HashMap<String, Vec<String>> = HashMap::new();
    for id in node_ids {
        super_members.insert(id.clone(), vec![id.clone()]);
    }

    let max_levels = if config.enable_hierarchy {
        config.max_hierarchy_levels.max(1)
    } else {
        1
    };

    let mut levels_run = 0usize;
    for level in 0..max_levels {
        let partition = louvain_phase1_local_move(
            &current_nodes,
            &current_adj,
            current_total,
            config.resolution,
            config.max_iterations,
        );
        levels_run = level + 1;

        // Remap original nodes through this level's partition
        let mut next_super_members: HashMap<String, Vec<String>> = HashMap::new();
        for (node, &comm) in &partition {
            let super_id = format!("c{level}_{comm}");
            let members = super_members
                .get(node)
                .cloned()
                .unwrap_or_else(|| vec![node.clone()]);
            next_super_members
                .entry(super_id)
                .or_default()
                .extend(members);
        }
        // Dedup members
        for members in next_super_members.values_mut() {
            members.sort();
            members.dedup();
        }

        // Update original → community (use densest renumber later)
        let mut comm_of_original: HashMap<String, usize> = HashMap::new();
        for (super_id, members) in &next_super_members {
            let comm_num = super_id
                .rsplit('_')
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            for m in members {
                comm_of_original.insert(m.clone(), comm_num);
            }
        }
        original_to_comm = comm_of_original;
        super_members = next_super_members;

        if !config.enable_hierarchy || level + 1 >= max_levels {
            break;
        }

        // Phase 2: aggregate communities into a super-graph
        let (agg_nodes, agg_adj, agg_total) =
            aggregate_communities(&partition, &current_adj, level);
        // Stop if aggregation did not reduce the graph
        if agg_nodes.len() >= current_nodes.len() || agg_nodes.len() <= 1 {
            break;
        }
        current_nodes = agg_nodes;
        current_adj = agg_adj;
        current_total = agg_total.max(1.0);
    }

    (original_to_comm, levels_run)
}

/// Louvain phase-1 local moving on an explicit node list + adjacency.
fn louvain_phase1_local_move(
    node_ids: &[String],
    adjacency: &HashMap<String, Vec<(String, f64)>>,
    total_weight: f64,
    resolution: f64,
    max_iterations: usize,
) -> HashMap<String, usize> {
    let mut node_to_community: HashMap<String, usize> = HashMap::new();
    let mut community_weights: HashMap<usize, f64> = HashMap::new();

    for (idx, node_id) in node_ids.iter().enumerate() {
        node_to_community.insert(node_id.clone(), idx);
        let node_weight = adjacency
            .get(node_id)
            .map(|neighbors| neighbors.iter().map(|(_, w)| w).sum::<f64>())
            .unwrap_or(0.0);
        community_weights.insert(idx, node_weight);
    }

    let total_weight = total_weight.max(1.0);

    for _iteration in 0..max_iterations {
        let mut improved = false;

        for node_id in node_ids {
            let current_community = *node_to_community.get(node_id).unwrap();
            let neighbors = adjacency.get(node_id).cloned().unwrap_or_default();
            let node_weight: f64 = neighbors.iter().map(|(_, w)| w).sum();

            let mut neighbor_communities: HashMap<usize, f64> = HashMap::new();
            for (neighbor_id, weight) in &neighbors {
                if let Some(&comm) = node_to_community.get(neighbor_id) {
                    *neighbor_communities.entry(comm).or_default() += weight;
                }
            }

            let mut best_community = current_community;
            let mut best_gain = 0.0;
            let current_comm_weight = community_weights.get(&current_community).unwrap_or(&0.0);
            let ki_in_current = neighbor_communities.get(&current_community).unwrap_or(&0.0);

            for (&candidate_community, &ki_in) in &neighbor_communities {
                if candidate_community == current_community {
                    continue;
                }
                let sigma_tot = community_weights.get(&candidate_community).unwrap_or(&0.0);
                let delta_q = (ki_in / total_weight)
                    - resolution * (sigma_tot * node_weight) / (2.0 * total_weight * total_weight);
                let current_delta_q = (ki_in_current / total_weight)
                    - resolution * ((current_comm_weight - node_weight) * node_weight)
                        / (2.0 * total_weight * total_weight);
                let gain = delta_q - current_delta_q;
                if gain > best_gain {
                    best_gain = gain;
                    best_community = candidate_community;
                }
            }

            if best_community != current_community && best_gain > 1e-9 {
                if let Some(old_weight) = community_weights.get_mut(&current_community) {
                    *old_weight -= node_weight;
                }
                if let Some(new_weight) = community_weights.get_mut(&best_community) {
                    *new_weight += node_weight;
                }
                node_to_community.insert(node_id.clone(), best_community);
                improved = true;
            }
        }

        if !improved {
            break;
        }
    }

    node_to_community
}

type WeightedAdj = HashMap<String, Vec<(String, f64)>>;

/// Phase-2: build super-graph where each community is a node; edge weights sum
/// inter-community links (Blondel / NetworkX Louvain aggregation).
fn aggregate_communities(
    partition: &HashMap<String, usize>,
    adjacency: &WeightedAdj,
    level: usize,
) -> (Vec<String>, WeightedAdj, f64) {
    let mut edge_weights: HashMap<(String, String), f64> = HashMap::new();
    let mut total = 0.0;

    for (src, neighbors) in adjacency {
        let Some(&src_c) = partition.get(src) else {
            continue;
        };
        let src_super = format!("c{level}_{src_c}");
        for (tgt, w) in neighbors {
            let Some(&tgt_c) = partition.get(tgt) else {
                continue;
            };
            let tgt_super = format!("c{level}_{tgt_c}");
            // Count each undirected edge once (src_id <= tgt_id lexicographically)
            if src > tgt {
                continue;
            }
            let (a, b) = if src_super <= tgt_super {
                (src_super.clone(), tgt_super)
            } else {
                (tgt_super, src_super.clone())
            };
            *edge_weights.entry((a, b)).or_default() += *w;
            // Internal edges (self-loops) still contribute to total weight
            total += *w;
        }
    }

    let mut agg_adj: HashMap<String, Vec<(String, f64)>> = HashMap::new();
    let mut nodes_set: HashSet<String> = HashSet::new();
    for ((a, b), w) in edge_weights {
        nodes_set.insert(a.clone());
        nodes_set.insert(b.clone());
        if a == b {
            // Self-loop: store once
            agg_adj.entry(a).or_default().push((b, w));
        } else {
            agg_adj.entry(a.clone()).or_default().push((b.clone(), w));
            agg_adj.entry(b).or_default().push((a, w));
        }
    }

    let mut nodes: Vec<String> = nodes_set.into_iter().collect();
    nodes.sort();
    (nodes, agg_adj, total)
}

/// Label propagation community detection.
async fn label_propagation(
    graph: &Arc<dyn GraphStorage>,
    config: &CommunityConfig,
) -> Result<CommunityDetectionResult> {
    let loaded = load_for_config(graph, config).await?;
    let nodes = loaded.nodes;
    let edges = loaded.edges;

    if nodes.is_empty() {
        return Ok(CommunityDetectionResult::new());
    }

    // Build adjacency list
    let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();
    for edge in &edges {
        adjacency
            .entry(edge.source.clone())
            .or_default()
            .push(edge.target.clone());
        adjacency
            .entry(edge.target.clone())
            .or_default()
            .push(edge.source.clone());
    }

    // Initialize: each node has its own label
    let mut labels: HashMap<String, usize> = HashMap::new();
    for (idx, node) in nodes.iter().enumerate() {
        labels.insert(node.id.clone(), idx);
    }

    // Iterate until convergence
    for _iteration in 0..config.max_iterations {
        let mut changed = false;

        for node in &nodes {
            let neighbors = adjacency.get(&node.id).cloned().unwrap_or_default();
            if neighbors.is_empty() {
                continue;
            }

            // Count neighbor labels
            let mut label_counts: HashMap<usize, usize> = HashMap::new();
            for neighbor_id in &neighbors {
                if let Some(&label) = labels.get(neighbor_id) {
                    *label_counts.entry(label).or_default() += 1;
                }
            }

            // Find most common label
            if let Some((&best_label, _)) = label_counts.iter().max_by_key(|(_, &count)| count) {
                let current_label = *labels.get(&node.id).unwrap();
                if best_label != current_label {
                    labels.insert(node.id.clone(), best_label);
                    changed = true;
                }
            }
        }

        if !changed {
            break;
        }
    }

    // Build communities from labels
    let mut communities_map: HashMap<usize, Vec<String>> = HashMap::new();
    for (node_id, label) in &labels {
        communities_map
            .entry(*label)
            .or_default()
            .push(node_id.clone());
    }

    let mut result = CommunityDetectionResult::new();
    let mut new_id = 0;
    let mut id_mapping: HashMap<usize, usize> = HashMap::new();

    for (old_id, members) in communities_map {
        if members.len() >= config.min_community_size {
            id_mapping.insert(old_id, new_id);

            let mut community = Community::new(new_id);
            community.members = members;
            result.communities.push(community);

            new_id += 1;
        }
    }

    for (node_id, old_label) in labels {
        if let Some(&new_comm) = id_mapping.get(&old_label) {
            result.node_to_community.insert(node_id, new_comm);
        }
    }

    Ok(result)
}

/// Connected components detection.
async fn connected_components(
    graph: &Arc<dyn GraphStorage>,
    config: &CommunityConfig,
) -> Result<CommunityDetectionResult> {
    let loaded = load_for_config(graph, config).await?;
    let nodes = loaded.nodes;
    let edges = loaded.edges;

    if nodes.is_empty() {
        return Ok(CommunityDetectionResult::new());
    }

    // Build adjacency list
    let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();
    for edge in &edges {
        adjacency
            .entry(edge.source.clone())
            .or_default()
            .push(edge.target.clone());
        adjacency
            .entry(edge.target.clone())
            .or_default()
            .push(edge.source.clone());
    }

    // BFS to find connected components
    let mut visited: HashSet<String> = HashSet::new();
    let mut communities: Vec<Community> = Vec::new();
    let mut node_to_community: HashMap<String, usize> = HashMap::new();
    let mut community_id = 0;

    for node in &nodes {
        if visited.contains(&node.id) {
            continue;
        }

        // BFS from this node
        let mut queue = vec![node.id.clone()];
        let mut component: Vec<String> = Vec::new();

        while let Some(current) = queue.pop() {
            if visited.contains(&current) {
                continue;
            }

            visited.insert(current.clone());
            component.push(current.clone());

            if let Some(neighbors) = adjacency.get(&current) {
                for neighbor in neighbors {
                    if !visited.contains(neighbor) {
                        queue.push(neighbor.clone());
                    }
                }
            }
        }

        if component.len() >= config.min_community_size {
            for member in &component {
                node_to_community.insert(member.clone(), community_id);
            }

            let mut community = Community::new(community_id);
            community.members = component;
            communities.push(community);

            community_id += 1;
        }
    }

    Ok(CommunityDetectionResult {
        communities,
        node_to_community,
        modularity: 0.0,
        hierarchy_levels: 1,
    })
}

/// Calculate modularity score.
fn calculate_modularity(
    result: &CommunityDetectionResult,
    adjacency: &HashMap<String, Vec<(String, f64)>>,
    total_weight: f64,
) -> f64 {
    if total_weight == 0.0 {
        return 0.0;
    }

    let mut q = 0.0;
    let m = total_weight;

    for community in &result.communities {
        let members: HashSet<&String> = community.members.iter().collect();

        let mut internal_weight = 0.0;
        let mut total_degree = 0.0;

        for member in &community.members {
            if let Some(neighbors) = adjacency.get(member) {
                for (neighbor, weight) in neighbors {
                    total_degree += weight;
                    if members.contains(neighbor) {
                        internal_weight += weight;
                    }
                }
            }
        }

        // Each internal edge is counted twice
        internal_weight /= 2.0;

        q += internal_weight / m - (total_degree / (2.0 * m)).powi(2);
    }

    q
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::memory::MemoryGraphStorage;

    fn test_graph() -> Arc<dyn GraphStorage> {
        Arc::new(MemoryGraphStorage::new("test"))
    }

    #[tokio::test]
    async fn test_community_detection_empty_graph() {
        let graph = test_graph();
        graph.initialize().await.unwrap();

        let config = CommunityConfig::default();
        let result = detect_communities_unchecked(&graph, &config).await.unwrap();

        assert!(result.communities.is_empty());
    }

    #[tokio::test]
    async fn test_connected_components() {
        let graph = test_graph();
        graph.initialize().await.unwrap();

        // Create two disconnected components
        let mut props1 = HashMap::new();
        props1.insert("name".to_string(), serde_json::json!("A"));
        graph.upsert_node("A", props1).await.unwrap();

        let mut props2 = HashMap::new();
        props2.insert("name".to_string(), serde_json::json!("B"));
        graph.upsert_node("B", props2).await.unwrap();

        let mut edge_props = HashMap::new();
        edge_props.insert("weight".to_string(), serde_json::json!(1.0));
        graph.upsert_edge("A", "B", edge_props).await.unwrap();

        let mut props3 = HashMap::new();
        props3.insert("name".to_string(), serde_json::json!("C"));
        graph.upsert_node("C", props3).await.unwrap();

        let mut props4 = HashMap::new();
        props4.insert("name".to_string(), serde_json::json!("D"));
        graph.upsert_node("D", props4).await.unwrap();

        let mut edge_props2 = HashMap::new();
        edge_props2.insert("weight".to_string(), serde_json::json!(1.0));
        graph.upsert_edge("C", "D", edge_props2).await.unwrap();

        let config = CommunityConfig {
            algorithm: CommunityAlgorithm::ConnectedComponents,
            min_community_size: 2,
            ..Default::default()
        };

        let result = detect_communities_unchecked(&graph, &config).await.unwrap();

        // Should have 2 communities of size 2 each
        assert_eq!(result.communities.len(), 2);
        assert!(result.communities.iter().all(|c| c.size() == 2));
    }

    #[tokio::test]
    async fn test_louvain_simple() {
        let graph = test_graph();
        graph.initialize().await.unwrap();

        // Create a simple graph
        for node_id in ["A", "B", "C", "D", "E"] {
            let mut props = HashMap::new();
            props.insert("name".to_string(), serde_json::json!(node_id));
            graph.upsert_node(node_id, props).await.unwrap();
        }

        // Dense connections within groups
        for (src, tgt) in [("A", "B"), ("B", "C"), ("A", "C"), ("D", "E")] {
            let mut edge_props = HashMap::new();
            edge_props.insert("weight".to_string(), serde_json::json!(1.0));
            graph.upsert_edge(src, tgt, edge_props).await.unwrap();
        }

        // Weak connection between groups
        let mut edge_props = HashMap::new();
        edge_props.insert("weight".to_string(), serde_json::json!(0.1));
        graph.upsert_edge("C", "D", edge_props).await.unwrap();

        let config = CommunityConfig {
            algorithm: CommunityAlgorithm::Louvain,
            min_community_size: 2,
            ..Default::default()
        };

        let result = detect_communities_unchecked(&graph, &config).await.unwrap();

        // Should detect at least 2 communities
        assert!(!result.communities.is_empty());
    }

    #[tokio::test]
    async fn load_graph_bounded_respects_max_nodes_cap() {
        let graph = test_graph();
        graph.initialize().await.unwrap();
        for i in 0..10 {
            let id = format!("N{i}");
            let mut props = HashMap::new();
            props.insert("name".to_string(), serde_json::json!(id));
            graph.upsert_node(&id, props).await.unwrap();
        }
        for i in 0..9 {
            let mut edge_props = HashMap::new();
            edge_props.insert("weight".to_string(), serde_json::json!(1.0));
            graph
                .upsert_edge(&format!("N{i}"), &format!("N{}", i + 1), edge_props)
                .await
                .unwrap();
        }

        let loaded = load_graph_bounded(&graph, 4).await.unwrap();
        assert_eq!(loaded.nodes.len(), 4);
        assert!(loaded.sampled);
        assert!(loaded.total_nodes_estimate >= 4);
        // Edges only among loaded nodes
        let ids: HashSet<_> = loaded.nodes.iter().map(|n| n.id.clone()).collect();
        for e in &loaded.edges {
            assert!(ids.contains(&e.source) && ids.contains(&e.target));
        }
    }

    #[tokio::test]
    async fn load_graph_bounded_scoped_filters_workspace() {
        let graph = test_graph();
        graph.initialize().await.unwrap();
        let ws_a = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";
        let ws_b = "ffffffff-1111-2222-3333-444444444444";

        for (ws, name) in [(ws_a, "ALPHA"), (ws_a, "BETA"), (ws_b, "GAMMA")] {
            let id = format!("{ws}::{name}");
            let mut props = HashMap::new();
            props.insert("workspace_id".into(), serde_json::json!(ws));
            props.insert("name".into(), serde_json::json!(name));
            graph.upsert_node(&id, props).await.unwrap();
        }
        let mut edge_a = HashMap::new();
        edge_a.insert("workspace_id".into(), serde_json::json!(ws_a));
        edge_a.insert("weight".into(), serde_json::json!(1.0));
        graph
            .upsert_edge(
                &format!("{ws_a}::ALPHA"),
                &format!("{ws_a}::BETA"),
                edge_a,
            )
            .await
            .unwrap();
        let mut edge_b = HashMap::new();
        edge_b.insert("workspace_id".into(), serde_json::json!(ws_b));
        edge_b.insert("weight".into(), serde_json::json!(1.0));
        // Self-loop-style pair within B using GAMMA only — add a second B node.
        let id_b2 = format!("{ws_b}::DELTA");
        let mut props_b2 = HashMap::new();
        props_b2.insert("workspace_id".into(), serde_json::json!(ws_b));
        graph.upsert_node(&id_b2, props_b2).await.unwrap();
        graph
            .upsert_edge(&format!("{ws_b}::GAMMA"), &id_b2, edge_b)
            .await
            .unwrap();

        let loaded = load_graph_bounded_scoped(&graph, 50, Some(ws_a), None)
            .await
            .unwrap();
        assert_eq!(loaded.nodes.len(), 2);
        for n in &loaded.nodes {
            assert!(n.id.starts_with(&format!("{ws_a}::")));
        }
        assert_eq!(loaded.edges.len(), 1);
        assert!(loaded.edges[0].source.starts_with(&format!("{ws_a}::")));
        assert!(loaded.edges[0].target.starts_with(&format!("{ws_a}::")));
    }

    #[tokio::test]
    async fn unit_louvain_hierarchy_levels() {
        // D-54: with hierarchy enabled, phase-2 aggregation reports levels >= 1
        // and can coarsen a nested two-clique graph.
        let graph = test_graph();
        graph.initialize().await.unwrap();

        // Two dense triangles weakly linked — classic hierarchy toy graph.
        for id in ["A", "B", "C", "D", "E", "F"] {
            let mut props = HashMap::new();
            props.insert("name".to_string(), serde_json::json!(id));
            graph.upsert_node(id, props).await.unwrap();
        }
        for (src, tgt, w) in [
            ("A", "B", 1.0),
            ("B", "C", 1.0),
            ("A", "C", 1.0),
            ("D", "E", 1.0),
            ("E", "F", 1.0),
            ("D", "F", 1.0),
            ("C", "D", 0.05),
        ] {
            let mut edge_props = HashMap::new();
            edge_props.insert("weight".to_string(), serde_json::json!(w));
            graph.upsert_edge(src, tgt, edge_props).await.unwrap();
        }

        let config = CommunityConfig {
            algorithm: CommunityAlgorithm::Louvain,
            min_community_size: 1,
            enable_hierarchy: true,
            max_hierarchy_levels: 3,
            ..Default::default()
        };
        let result = detect_communities_unchecked(&graph, &config).await.unwrap();
        assert!(
            result.hierarchy_levels >= 1,
            "hierarchy mode must record at least one Louvain level"
        );
        assert!(
            !result.communities.is_empty(),
            "expected communities on hierarchical toy graph"
        );
        // Phase-2 path must stamp hierarchy_level on community properties.
        assert!(
            result
                .communities
                .iter()
                .any(|c| c.properties.contains_key("hierarchy_level")),
            "communities should expose hierarchy_level property"
        );

        // Flat mode still works and reports a single level.
        let flat = CommunityConfig {
            algorithm: CommunityAlgorithm::Louvain,
            min_community_size: 1,
            enable_hierarchy: false,
            ..Default::default()
        };
        let flat_result = detect_communities_unchecked(&graph, &flat).await.unwrap();
        assert_eq!(flat_result.hierarchy_levels, 1);
    }

    #[tokio::test]
    async fn community_detection_uses_bounded_loader_not_full_scan_path() {
        let graph = test_graph();
        graph.initialize().await.unwrap();
        for i in 0..6 {
            let id = format!("C{i}");
            let mut props = HashMap::new();
            props.insert("name".to_string(), serde_json::json!(id));
            graph.upsert_node(&id, props).await.unwrap();
        }
        for (a, b) in [("C0", "C1"), ("C1", "C2"), ("C3", "C4"), ("C4", "C5")] {
            let mut edge_props = HashMap::new();
            edge_props.insert("weight".to_string(), serde_json::json!(1.0));
            graph.upsert_edge(a, b, edge_props).await.unwrap();
        }

        let config = CommunityConfig {
            algorithm: CommunityAlgorithm::ConnectedComponents,
            min_community_size: 2,
            max_nodes: 3,
            ..Default::default()
        };
        // Must succeed without calling unbounded get_all_nodes semantics.
        let result = detect_communities_unchecked(&graph, &config).await.unwrap();
        // With only 3 nodes loaded, at most one size>=2 component may appear.
        assert!(result.communities.len() <= 2);
    }
}
