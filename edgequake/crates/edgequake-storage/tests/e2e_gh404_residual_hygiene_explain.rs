//! GH-404 residual — list/popular/degree hygiene (child tables, no parent text-cast).
//!
//! Run:
//! ```bash
//! export DATABASE_URL="$(cat /tmp/edgequake-db-url)"
//! export EDGEQUAKE_REQUIRE_POSTGRES_TESTS=1
//! cargo test -p edgequake-storage --features postgres --test e2e_gh404_residual_hygiene_explain -- --nocapture
//! ```

#![cfg(feature = "postgres")]

#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;

use edgequake_storage::traits::{GraphStorage, GraphStorageMutateOps, GraphStorageReadOps};
use edgequake_storage::{backfill_communities_if_needed, PostgresAGEGraphStorage};
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
    props.insert("tenant_id".into(), json!("t-gh404-residual"));
    props.insert("label".into(), json!(name));
    (id, props)
}

fn assert_child_table_hygiene(sql_or_plan: &str, label: &str, allow_offset: bool) {
    assert!(
        !sql_or_plan.contains("_ag_label_vertex"),
        "{label} must not touch _ag_label_vertex; got:\n{sql_or_plan}"
    );
    assert!(
        !sql_or_plan.contains("_ag_label_edge"),
        "{label} must not touch _ag_label_edge; got:\n{sql_or_plan}"
    );
    assert!(
        !sql_or_plan.contains("start_id::text"),
        "{label} must not text-cast JOIN on start_id; got:\n{sql_or_plan}"
    );
    if !allow_offset {
        let lower = sql_or_plan.to_lowercase();
        assert!(
            !lower.contains("offset"),
            "{label} must not use OFFSET; got:\n{sql_or_plan}"
        );
    }
}

async fn explain_plan(pool: &sqlx::PgPool, sql: &str, label: &str) -> String {
    let explain = format!("EXPLAIN (FORMAT TEXT) {sql}");
    let plan_rows = sqlx::query(&explain)
        .fetch_all(pool)
        .await
        .unwrap_or_else(|e| panic!("{label} EXPLAIN failed: {e}\nSQL:\n{sql}"));
    plan_rows
        .iter()
        .map(|r| {
            r.try_get::<String, _>(0)
                .or_else(|_| r.try_get::<String, _>("QUERY PLAN"))
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[tokio::test]
async fn gh404_residual_list_popular_search_explain_hygiene() {
    let Some(config) = postgres_test_config::require_or_skip_postgres("gh404_residual_explain")
    else {
        return;
    };
    let prev = std::env::var("EDGEQUAKE_NATIVE_GRAPH_WRITES").ok();
    std::env::set_var("EDGEQUAKE_NATIVE_GRAPH_WRITES", "1");

    let graph = PostgresAGEGraphStorage::new(config.clone());
    graph.initialize().await.expect("init");

    let ws = Uuid::new_v4().to_string();
    let list_sql = graph.list_nodes_filtered_sql_for_explain(Some(&ws), 0, 50);
    let popular_sql = graph.popular_nodes_sql_for_explain(Some(&ws), 20);
    let search_sql = graph.search_nodes_sql_for_explain(Some(&ws), "ALPHA", 20);

    assert!(
        list_sql.contains(".\"Node\""),
        "list SQL must scan Node: {list_sql}"
    );
    assert_child_table_hygiene(&list_sql, "list_nodes SQL", true);
    assert!(
        list_sql.to_lowercase().contains("offset"),
        "list API keeps OFFSET on Node child"
    );

    assert_child_table_hygiene(&popular_sql, "popular SQL", false);
    assert!(
        popular_sql.contains(".\"EDGE\"") && popular_sql.contains(".\"Node\""),
        "popular must use Node+EDGE: {popular_sql}"
    );

    assert_child_table_hygiene(&search_sql, "search_nodes SQL", false);
    assert!(
        search_sql.contains(".\"EDGE\"") && search_sql.contains(".\"Node\""),
        "search must use Node+EDGE: {search_sql}"
    );

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
    for (label, sql, allow_offset) in [
        ("list", list_sql.as_str(), true),
        ("popular", popular_sql.as_str(), false),
        ("search", search_sql.as_str(), false),
    ] {
        let plan = explain_plan(&pool, sql, label).await;
        assert_child_table_hygiene(&plan, &format!("{label} EXPLAIN plan"), allow_offset);
        assert!(!plan.is_empty(), "{label} EXPLAIN empty");
    }

    let _ = graph.clear().await;
    match prev {
        Some(v) => std::env::set_var("EDGEQUAKE_NATIVE_GRAPH_WRITES", v),
        None => std::env::remove_var("EDGEQUAKE_NATIVE_GRAPH_WRITES"),
    }
}

#[tokio::test]
async fn gh404_residual_runtime_popular_matches_batch_and_scoped_backfill() {
    let Some(config) = postgres_test_config::require_or_skip_postgres("gh404_residual_runtime")
    else {
        return;
    };
    let prev = std::env::var("EDGEQUAKE_NATIVE_GRAPH_WRITES").ok();
    std::env::set_var("EDGEQUAKE_NATIVE_GRAPH_WRITES", "1");
    let prev_comm = std::env::var("EDGEQUAKE_COMMUNITY_GLOBAL").ok();
    std::env::set_var("EDGEQUAKE_COMMUNITY_GLOBAL", "true");

    let graph = Arc::new(PostgresAGEGraphStorage::new(config.clone()));
    graph.initialize().await.expect("init");

    let ws = Uuid::new_v4().to_string();
    let (a_id, a_props) = scoped_node(&ws, "A1");
    let (b_id, b_props) = scoped_node(&ws, "A2");
    let (c_id, c_props) = scoped_node(&ws, "A3");
    graph
        .upsert_nodes_batch(&[
            (a_id.clone(), a_props),
            (b_id.clone(), b_props),
            (c_id.clone(), c_props),
        ])
        .await
        .expect("nodes");

    for (src, tgt) in [(&a_id, &b_id), (&b_id, &c_id)] {
        let mut props = HashMap::new();
        props.insert("workspace_id".into(), json!(ws));
        props.insert("relation_type".into(), json!("RELATED"));
        props.insert("weight".into(), json!(1.0));
        graph.upsert_edge(src, tgt, props).await.expect("edge");
    }

    let popular = graph
        .get_popular_nodes_with_degree(10, None, None, None, Some(&ws))
        .await
        .expect("popular");
    assert!(!popular.is_empty());

    let ids: Vec<String> = popular.iter().map(|(n, _)| n.id.clone()).collect();
    let batch_vec = graph.node_degrees_batch(&ids).await.expect("batch degrees");
    let batch: HashMap<String, usize> = batch_vec.into_iter().collect();
    for (node, out_deg) in &popular {
        // Popular reports out-degree only; batch is total degree.
        let total = *batch.get(&node.id).unwrap_or(&0);
        assert!(
            total >= *out_deg,
            "batch total degree {} should be >= popular out-degree {} for {}",
            total,
            out_deg,
            node.id
        );
        let single = graph.node_degree(&node.id).await.expect("node_degree");
        assert_eq!(single, total, "single degree must match batch");
    }

    let graph_dyn: Arc<dyn GraphStorage> = graph.clone();
    let backfill = backfill_communities_if_needed(graph_dyn.clone())
        .await
        .expect("backfill");
    assert!(
        backfill.is_some(),
        "scoped backfill should label UUID workspace nodes"
    );
    let labeled = graph.get_node(&a_id).await.expect("get").expect("exists");
    assert!(
        labeled.properties.contains_key("community_id"),
        "backfill must set community_id on workspace node"
    );

    let _ = graph.clear().await;
    match prev {
        Some(v) => std::env::set_var("EDGEQUAKE_NATIVE_GRAPH_WRITES", v),
        None => std::env::remove_var("EDGEQUAKE_NATIVE_GRAPH_WRITES"),
    }
    match prev_comm {
        Some(v) => std::env::set_var("EDGEQUAKE_COMMUNITY_GLOBAL", v),
        None => std::env::remove_var("EDGEQUAKE_COMMUNITY_GLOBAL"),
    }
}
