//! Persist community detection labels on graph nodes (SPEC-023 I6).
//!
//! Index-time community refresh keeps query-time global search O(batch) instead of
//! running Louvain on every request. Refreshes are **debounced** per workspace
//! via [`crate::community_index_service`] (SPEC-024 Phase 1.3 / 4.2).

use std::sync::Arc;

use serde_json::json;

use crate::community::{detect_communities_unchecked, CommunityConfig, CommunityDetectionResult};
use crate::error::Result;
use crate::traits::GraphStorage;

/// Whether community index refresh + global query expansion is enabled (default: true).
///
/// Set `EDGEQUAKE_COMMUNITY_GLOBAL=false` to disable (backward compatible).
pub fn community_features_enabled() -> bool {
    std::env::var("EDGEQUAKE_COMMUNITY_GLOBAL")
        .map(|v| !matches!(v.to_ascii_lowercase().as_str(), "false" | "0" | "off"))
        .unwrap_or(true)
}

/// Max nodes for automatic community Louvain (backfill + ingest refresh). Default 50_000.
pub fn community_auto_max_nodes() -> usize {
    std::env::var("EDGEQUAKE_COMMUNITY_BACKFILL_MAX_NODES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(50_000)
}

/// Detect communities and write `community_id` onto node properties.
pub async fn detect_and_persist_communities(
    graph: Arc<dyn GraphStorage>,
    config: &CommunityConfig,
) -> Result<CommunityDetectionResult> {
    let result = detect_communities_unchecked(&graph, config).await?;
    persist_community_labels(&graph, &result).await?;
    Ok(result)
}

/// Write `community_id` for all nodes in the detection result (batch upsert).
pub async fn persist_community_labels(
    graph: &Arc<dyn GraphStorage>,
    result: &CommunityDetectionResult,
) -> Result<usize> {
    if result.node_to_community.is_empty() {
        return Ok(0);
    }

    let ids: Vec<String> = result.node_to_community.keys().cloned().collect();
    let existing = graph.get_nodes_batch(&ids).await?;
    let mut batch = Vec::with_capacity(existing.len());

    // SPEC-046 EQ-046-11: optional extractive community reports on node props
    let reports_on = crate::community_reports::community_reports_enabled();
    let report_by_cid: std::collections::HashMap<usize, String> = if reports_on {
        crate::community_reports::build_community_report_records(result, 24, None, None)
            .into_iter()
            .filter_map(|(_id, text, meta)| {
                let cid = meta.get("community_id")?.as_u64()? as usize;
                Some((cid, text))
            })
            .collect()
    } else {
        std::collections::HashMap::new()
    };

    for (node_id, community_id) in &result.node_to_community {
        let Some(node) = existing.get(node_id) else {
            continue;
        };
        let mut props = node.properties.clone();
        props.insert("community_id".to_string(), json!(community_id));
        if let Some(report) = report_by_cid.get(community_id) {
            props.insert("community_report".to_string(), json!(report));
        }
        batch.push((node_id.clone(), props));
    }

    if batch.is_empty() {
        return Ok(0);
    }

    let count = batch.len();
    graph.upsert_nodes_batch(&batch).await?;
    Ok(count)
}

/// True when a sample of graph nodes lacks `community_id` (legacy graphs pre-044).
pub async fn needs_community_backfill(graph: &Arc<dyn GraphStorage>) -> Result<bool> {
    let sample = graph
        .get_popular_nodes_with_degree(8, None, None, None, None)
        .await?;
    if sample.is_empty() {
        return Ok(false);
    }
    Ok(sample
        .iter()
        .any(|(node, _)| !node.properties.contains_key("community_id")))
}

/// Collect distinct UUID workspace ids from nodes that still need community labels.
///
/// GH-404 residual: backfill must never run unscoped Louvain on the full AGE graph.
/// Only UUID-parseable `workspace_id` properties are returned; missing → skip.
pub async fn workspaces_needing_community_backfill(
    graph: &Arc<dyn GraphStorage>,
) -> Result<Vec<String>> {
    // Broader sample than needs_* so multi-workspace fleets are covered.
    let sample = graph
        .get_popular_nodes_with_degree(64, None, None, None, None)
        .await?;
    let mut seen = std::collections::BTreeSet::new();
    for (node, _) in &sample {
        if node.properties.contains_key("community_id") {
            continue;
        }
        let Some(ws) = node.properties.get("workspace_id").and_then(|v| v.as_str()) else {
            continue;
        };
        if uuid::Uuid::parse_str(ws).is_ok() {
            seen.insert(ws.to_string());
        }
    }
    Ok(seen.into_iter().collect())
}

/// One-shot backfill for existing graphs (startup / migration 044).
///
/// GH-404 residual: runs **per workspace** with scoped `CommunityConfig`.
/// If no UUID workspace ids are discoverable, skips (fail-closed) rather than
/// loading the entire AGE graph.
pub async fn backfill_communities_if_needed(
    graph: Arc<dyn GraphStorage>,
) -> Result<Option<CommunityDetectionResult>> {
    if !community_features_enabled() {
        return Ok(None);
    }
    if !needs_community_backfill(&graph).await? {
        return Ok(None);
    }

    let workspaces = workspaces_needing_community_backfill(&graph).await?;
    if workspaces.is_empty() {
        tracing::warn!(
            "Skipping automatic community backfill — no UUID workspace_id on unlabeled nodes (fail-closed GH-404)"
        );
        return Ok(None);
    }

    let threshold = community_auto_max_nodes();
    let mut last: Option<CommunityDetectionResult> = None;

    for ws in &workspaces {
        let ws_uuid = match uuid::Uuid::parse_str(ws) {
            Ok(u) => u,
            Err(_) => continue,
        };
        let node_count = match graph.node_count_by_workspace(&ws_uuid).await {
            Ok(n) => n,
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    workspace_id = %ws,
                    "Skipping workspace community backfill — node count failed (fail-closed)"
                );
                continue;
            }
        };
        if node_count > threshold {
            tracing::warn!(
                node_count,
                threshold,
                workspace_id = %ws,
                "Skipping workspace community backfill — graph too large (set EDGEQUAKE_COMMUNITY_BACKFILL_MAX_NODES)"
            );
            continue;
        }
        if node_count == 0 {
            continue;
        }

        tracing::info!(
            node_count,
            workspace_id = %ws,
            "Running automatic community backfill (migration 044, scoped)"
        );
        let config = CommunityConfig {
            workspace_id: Some(ws.clone()),
            ..CommunityConfig::default()
        };
        let result = detect_and_persist_communities(graph.clone(), &config).await?;
        tracing::info!(
            communities = result.communities.len(),
            labeled_nodes = result.node_to_community.len(),
            workspace_id = %ws,
            "Community backfill complete for workspace"
        );
        last = Some(result);
    }

    Ok(last)
}

/// Refresh community labels after ingest merge (non-fatal on failure).
///
/// GH-404: routes through the guarded path (size gate, advisory lock, scoped
/// config when callers use [`refresh_community_index_now_with_extras`]).
pub async fn refresh_community_index(graph: Arc<dyn GraphStorage>) {
    crate::community_index_service::refresh_community_index_now(graph).await;
}

/// Background startup backfill for legacy graphs (migration 044 / postgres bootstrap).
pub fn spawn_community_backfill_if_needed(graph: Arc<dyn GraphStorage>) {
    if !community_features_enabled() {
        tracing::debug!("Community backfill skipped (EDGEQUAKE_COMMUNITY_GLOBAL=false)");
        return;
    }
    tokio::spawn(async move {
        match backfill_communities_if_needed(graph).await {
            Ok(Some(_)) => tracing::info!("Automatic community backfill complete"),
            Ok(None) => {}
            Err(e) => tracing::warn!(
                error = %e,
                "Community backfill failed — will retry on next restart"
            ),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::memory::MemoryGraphStorage;
    use serde_json::json;
    use std::collections::HashMap;

    #[tokio::test]
    async fn persist_community_labels_writes_property() {
        let graph: Arc<dyn GraphStorage> = Arc::new(MemoryGraphStorage::new("comm-persist"));
        graph.initialize().await.unwrap();
        graph.upsert_node("A", HashMap::new()).await.unwrap();
        graph.upsert_node("B", HashMap::new()).await.unwrap();
        graph.upsert_edge("A", "B", HashMap::new()).await.unwrap();

        let mut result = CommunityDetectionResult::new();
        result.node_to_community.insert("A".to_string(), 0);
        result.node_to_community.insert("B".to_string(), 0);

        let n = persist_community_labels(&graph, &result).await.unwrap();
        assert_eq!(n, 2);

        let node = graph.get_node("A").await.unwrap().unwrap();
        assert_eq!(
            node.properties.get("community_id").and_then(|v| v.as_u64()),
            Some(0)
        );
    }

    #[tokio::test]
    async fn backfill_skips_when_no_uuid_workspace() {
        // Makefile defaults EDGEQUAKE_COMMUNITY_GLOBAL=false; tests need product default.
        let prev = std::env::var("EDGEQUAKE_COMMUNITY_GLOBAL").ok();
        // SAFETY: test-only env restore; not concurrent with other env writers in this module.
        unsafe { std::env::remove_var("EDGEQUAKE_COMMUNITY_GLOBAL") };

        let graph: Arc<dyn GraphStorage> = Arc::new(MemoryGraphStorage::new("comm-backfill-skip"));
        graph.initialize().await.unwrap();
        let mut props = HashMap::new();
        props.insert("workspace_id".into(), json!("not-a-uuid"));
        graph.upsert_node("A", props.clone()).await.unwrap();
        graph.upsert_node("B", props).await.unwrap();
        graph.upsert_edge("A", "B", HashMap::new()).await.unwrap();

        let result = backfill_communities_if_needed(graph.clone()).await.unwrap();
        assert!(result.is_none(), "must skip unscoped Louvain");
        let a = graph.get_node("A").await.unwrap().unwrap();
        assert!(
            !a.properties.contains_key("community_id"),
            "must not label without UUID workspace"
        );

        match prev {
            Some(v) => unsafe { std::env::set_var("EDGEQUAKE_COMMUNITY_GLOBAL", v) },
            None => unsafe { std::env::remove_var("EDGEQUAKE_COMMUNITY_GLOBAL") },
        }
    }

    #[tokio::test]
    async fn backfill_scopes_to_uuid_workspace() {
        let prev = std::env::var("EDGEQUAKE_COMMUNITY_GLOBAL").ok();
        unsafe { std::env::remove_var("EDGEQUAKE_COMMUNITY_GLOBAL") };

        let graph: Arc<dyn GraphStorage> = Arc::new(MemoryGraphStorage::new("comm-backfill-ws"));
        graph.initialize().await.unwrap();
        let ws = uuid::Uuid::new_v4().to_string();
        for name in ["A", "B", "C"] {
            let mut props = HashMap::new();
            props.insert("workspace_id".into(), json!(ws));
            props.insert("node_id".into(), json!(format!("{ws}::{name}")));
            graph
                .upsert_node(&format!("{ws}::{name}"), props)
                .await
                .unwrap();
        }
        graph
            .upsert_edge(&format!("{ws}::A"), &format!("{ws}::B"), HashMap::new())
            .await
            .unwrap();
        graph
            .upsert_edge(&format!("{ws}::B"), &format!("{ws}::C"), HashMap::new())
            .await
            .unwrap();

        let result = backfill_communities_if_needed(graph.clone()).await.unwrap();
        assert!(result.is_some(), "scoped backfill should run");
        let a = graph.get_node(&format!("{ws}::A")).await.unwrap().unwrap();
        assert!(
            a.properties.contains_key("community_id"),
            "workspace nodes must be labeled"
        );

        match prev {
            Some(v) => unsafe { std::env::set_var("EDGEQUAKE_COMMUNITY_GLOBAL", v) },
            None => unsafe { std::env::remove_var("EDGEQUAKE_COMMUNITY_GLOBAL") },
        }
    }
}
