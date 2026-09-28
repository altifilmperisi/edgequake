//! GH-404 — community keyset scan must not plan OFFSET / parent-vertex text-cast joins.
//!
//! Run:
//! ```bash
//! export DATABASE_URL="$(cat /tmp/edgequake-db-url)"
//! export EDGEQUAKE_REQUIRE_POSTGRES_TESTS=1
//! cargo test -p edgequake-storage --features postgres --test e2e_gh404_community_keyset_explain -- --nocapture
//! ```

#![cfg(feature = "postgres")]

#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;

use edgequake_storage::load_graph_bounded_scoped;
use edgequake_storage::traits::{
    EdgeListFilter, GraphScanOps, GraphStorage, GraphStorageMutateOps,
};
use edgequake_storage::PostgresAGEGraphStorage;
use serde_json::json;
use sqlx::Row;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

fn scoped_node(ws: &str, name: &str) -> (String, HashMap<String, serde_json::Value>) {
    let id = format!("{ws}::{name}");
    let mut props = HashMap::new();
    props.insert("node_id".into(), json!(id));
    props.insert("entity_type".into(), json!("CONCEPT"));
    props.insert("workspace_id".into(), json!(ws));
    props.insert("tenant_id".into(), json!("t-gh404"));
    (id, props)
}

fn assert_no_gh404_anti_patterns(sql_or_plan: &str, label: &str) {
    let lower = sql_or_plan.to_lowercase();
    assert!(
        !lower.contains("offset"),
        "{label} must not use OFFSET; got:\n{sql_or_plan}"
    );
    assert!(
        !sql_or_plan.contains("_ag_label_vertex"),
        "{label} must not touch _ag_label_vertex; got:\n{sql_or_plan}"
    );
    assert!(
        !sql_or_plan.contains("start_id::text"),
        "{label} must not text-cast JOIN on start_id; got:\n{sql_or_plan}"
    );
}

#[tokio::test]
async fn gh404_community_edge_keyset_explain_avoids_offset_and_vertex_joins() {
    let Some(config) = postgres_test_config::require_or_skip_postgres("gh404_explain") else {
        return;
    };
    let prev = std::env::var("EDGEQUAKE_NATIVE_GRAPH_WRITES").ok();
    std::env::set_var("EDGEQUAKE_NATIVE_GRAPH_WRITES", "1");

    let graph = PostgresAGEGraphStorage::new(config.clone());
    graph.initialize().await.expect("init");

    let ws = Uuid::new_v4().to_string();
    let edge_sql = graph.community_edge_keyset_sql_for_explain(Some(&ws), 2000);
    let node_sql = graph.community_node_keyset_sql_for_explain(Some(&ws), 2000);

    assert_no_gh404_anti_patterns(&edge_sql, "community edge keyset SQL");
    assert_no_gh404_anti_patterns(&node_sql, "community node keyset SQL");
    assert!(
        edge_sql.contains("eq_source_id") && edge_sql.contains("ORDER BY e.id::text"),
        "edge keyset must seek on id::text with eq_* endpoints: {edge_sql}"
    );
    assert!(
        edge_sql.contains(".\"EDGE\"") || edge_sql.contains(".\\\"EDGE\\\""),
        "edge keyset must scan child EDGE: {edge_sql}"
    );

    // Seed so EXPLAIN has a real relation.
    let (a_id, a_props) = scoped_node(&ws, "ALPHA");
    let (b_id, b_props) = scoped_node(&ws, "BETA");
    graph
        .upsert_nodes_batch(&[(a_id.clone(), a_props), (b_id.clone(), b_props)])
        .await
        .expect("upsert nodes");
    let mut edge_props = HashMap::new();
    edge_props.insert("workspace_id".into(), json!(ws));
    edge_props.insert("relation_type".into(), json!("RELATED"));
    edge_props.insert("weight".into(), json!(1.0));
    graph
        .upsert_edge(&a_id, &b_id, edge_props)
        .await
        .expect("upsert edge");

    let pool = postgres_test_config::contract_pg_pool(&config).await;
    for (label, sql) in [("edge", edge_sql.as_str()), ("node", node_sql.as_str())] {
        let explain = format!("EXPLAIN (FORMAT TEXT) {sql}");
        let plan_rows = sqlx::query(&explain)
            .fetch_all(&pool)
            .await
            .unwrap_or_else(|e| panic!("{label} EXPLAIN failed: {e}\nSQL:\n{sql}"));
        let plan = plan_rows
            .iter()
            .map(|r| {
                r.try_get::<String, _>(0)
                    .or_else(|_| r.try_get::<String, _>("QUERY PLAN"))
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert_no_gh404_anti_patterns(&plan, &format!("{label} EXPLAIN plan"));
        assert!(!plan.is_empty(), "{label} EXPLAIN returned empty plan");
    }

    let _ = graph.clear().await;
    match prev {
        Some(v) => std::env::set_var("EDGEQUAKE_NATIVE_GRAPH_WRITES", v),
        None => std::env::remove_var("EDGEQUAKE_NATIVE_GRAPH_WRITES"),
    }
}

#[tokio::test]
async fn gh404_runtime_scoped_scan_and_join_free_list() {
    let Some(config) = postgres_test_config::require_or_skip_postgres("gh404_runtime") else {
        return;
    };
    let prev = std::env::var("EDGEQUAKE_NATIVE_GRAPH_WRITES").ok();
    std::env::set_var("EDGEQUAKE_NATIVE_GRAPH_WRITES", "1");

    let graph = Arc::new(PostgresAGEGraphStorage::new(config.clone()));
    graph.initialize().await.expect("init");

    let ws_a = Uuid::new_v4().to_string();
    let ws_b = Uuid::new_v4().to_string();
    let (a1, p1) = scoped_node(&ws_a, "A1");
    let (a2, p2) = scoped_node(&ws_a, "A2");
    let (b1, p3) = scoped_node(&ws_b, "B1");
    let (b2, p4) = scoped_node(&ws_b, "B2");
    graph
        .upsert_nodes_batch(&[
            (a1.clone(), p1),
            (a2.clone(), p2),
            (b1.clone(), p3),
            (b2.clone(), p4),
        ])
        .await
        .expect("nodes");

    for (src, tgt, ws) in [(&a1, &a2, &ws_a), (&b1, &b2, &ws_b)] {
        let mut props = HashMap::new();
        props.insert("workspace_id".into(), json!(ws));
        props.insert("relation_type".into(), json!("RELATED"));
        props.insert("weight".into(), json!(1.0));
        graph.upsert_edge(src, tgt, props).await.expect("edge");
    }

    let edge_filter = EdgeListFilter {
        workspace_id: Some(ws_a.clone()),
        ..Default::default()
    };
    let page = graph
        .scan_edges_after(&edge_filter, None, 100)
        .await
        .expect("scan_edges_after");
    assert_eq!(page.items.len(), 1, "workspace A should see one edge");
    assert!(page.items[0].source.starts_with(&format!("{ws_a}::")));
    assert!(page.items[0].target.starts_with(&format!("{ws_a}::")));

    let graph_dyn: Arc<dyn GraphStorage> = graph.clone();
    let loaded = load_graph_bounded_scoped(&graph_dyn, 50, Some(&ws_a), None)
        .await
        .expect("scoped load");
    assert_eq!(loaded.nodes.len(), 2);
    assert_eq!(loaded.edges.len(), 1);
    for n in &loaded.nodes {
        assert!(n.id.starts_with(&format!("{ws_a}::")));
    }

    // Join-free list path must return endpoints for the relationships API.
    let list = graph
        .list_edges_filtered(
            &EdgeListFilter {
                workspace_id: Some(ws_a.clone()),
                ..Default::default()
            },
            0,
            20,
        )
        .await
        .expect("list_edges_filtered");
    assert!(list.total >= 1);
    assert!(!list.items.is_empty());
    assert!(!list.items[0].source.is_empty());
    assert!(!list.items[0].target.is_empty());

    // Relationship-id lookup must also resolve without vertex joins.
    let rel_id = format!("{}_{}", list.items[0].source, list.items[0].target);
    let found = graph
        .find_edge_by_relationship_id(
            &EdgeListFilter {
                workspace_id: Some(ws_a.clone()),
                ..Default::default()
            },
            &rel_id,
        )
        .await
        .expect("find_edge_by_relationship_id");
    assert!(
        found.is_some(),
        "must find edge by composite relationship id"
    );

    let _ = graph.clear().await;
    match prev {
        Some(v) => std::env::set_var("EDGEQUAKE_NATIVE_GRAPH_WRITES", v),
        None => std::env::remove_var("EDGEQUAKE_NATIVE_GRAPH_WRITES"),
    }
}
