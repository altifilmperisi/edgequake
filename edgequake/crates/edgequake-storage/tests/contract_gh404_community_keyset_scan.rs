//! Contract: community refresh must use keyset + eq_* endpoints (GH-404).
//!
//! The v0.26.0 failure mode was OFFSET pages with
//! `JOIN _ag_label_vertex ... start_id::text` that monopolized PostgreSQL.

#[test]
fn contract_community_edge_scan_is_keyset_eq_endpoints() {
    let scan = include_str!("../src/adapters/postgres/graph/scan_ops.rs");

    assert!(
        scan.contains("fn community_edge_scan_sql"),
        "community edge scan SQL builder must exist"
    );
    assert!(
        scan.contains("fn community_node_scan_sql"),
        "community node scan SQL builder must exist"
    );
    assert!(
        scan.contains("pg_scan_edges_after"),
        "community keyset edge entrypoint must exist"
    );
    assert!(
        scan.contains("pg_scan_nodes_after"),
        "community keyset node entrypoint must exist"
    );

    let edge_fn = scan
        .find("pub(super) fn community_edge_scan_sql")
        .expect("community_edge_scan_sql");
    let edge_body = &scan[edge_fn..];
    let end = edge_body
        .find("pub(super) fn community_node_scan_sql")
        .unwrap_or(edge_body.len().min(1200));
    let sql_region = &edge_body[..end];

    assert!(
        sql_region.contains("eq_source_id")
            || sql_region.contains("{src}")
            || sql_region.contains("source_id"),
        "edge keyset scan must project stored source endpoints"
    );
    assert!(
        sql_region.contains("e.id::text"),
        "edge keyset must seek on e.id::text"
    );
    assert!(
        sql_region.contains("ORDER BY e.id::text"),
        "edge keyset must ORDER BY e.id::text"
    );
    assert!(
        sql_region.contains("LIMIT {limit}"),
        "edge keyset must LIMIT"
    );
    assert!(
        !sql_region.contains("OFFSET"),
        "community edge scan must not use OFFSET"
    );
    assert!(
        !sql_region.contains("_ag_label_vertex"),
        "community edge scan must not join _ag_label_vertex"
    );
    assert!(
        !sql_region.contains("start_id::text"),
        "community edge scan must not text-cast JOIN on start_id"
    );
}

#[test]
fn contract_list_edges_drops_vertex_text_cast_joins() {
    let scan = include_str!("../src/adapters/postgres/graph/scan_ops.rs");

    let list_fn = scan
        .find("pub(super) async fn pg_list_edges_filtered")
        .expect("pg_list_edges_filtered");
    let list_body = &scan[list_fn..];
    let end = list_body
        .find("pub(super) fn list_edges_page_sql")
        .unwrap_or(list_body.len().min(2500));
    let region = &list_body[..end];

    assert!(
        region.contains("eq_source_id") || region.contains("coalesce_endpoint"),
        "list edges must resolve endpoints via eq_* / coalesce_endpoint"
    );
    assert!(
        !region.contains("_ag_label_vertex"),
        "list edges must not join _ag_label_vertex"
    );
    assert!(
        !region.contains("start_id::text"),
        "list edges must not text-cast JOIN on start_id"
    );

    let page_sql = scan
        .find("pub(super) fn list_edges_page_sql")
        .expect("list_edges_page_sql");
    let page_body = &scan[page_sql..];
    let page_end = page_body
        .find("pub(super) fn community_edge_scan_sql")
        .unwrap_or(page_body.len().min(800));
    let page_region = &page_body[..page_end];
    assert!(
        page_region.contains("OFFSET {offset}"),
        "relationships API list must keep OFFSET for page/total contract"
    );
    assert!(
        page_region.contains(".\"EDGE\" e") || page_region.contains(".\\\"EDGE\\\" e"),
        "list must scan child EDGE table"
    );
}

#[test]
fn contract_community_load_uses_keyset_not_list_offset() {
    let community = include_str!("../src/community.rs");
    assert!(
        community.contains("scan_nodes_after"),
        "load_graph_bounded must use scan_nodes_after"
    );
    assert!(
        community.contains("scan_edges_after"),
        "load_graph_bounded must use scan_edges_after"
    );
    assert!(
        community.contains("load_graph_bounded_scoped"),
        "workspace-scoped loader must exist"
    );
    // The OFFSET list APIs must not appear in the bounded loader body.
    let loader = community
        .find("pub async fn load_graph_bounded_scoped")
        .expect("load_graph_bounded_scoped");
    let body = &community[loader..];
    let end = body
        .find("async fn load_for_config")
        .or_else(|| body.find("/// Detect communities"))
        .unwrap_or(body.len().min(3500));
    let region = &body[..end];
    assert!(
        !region.contains("list_edges_filtered"),
        "scoped loader must not call list_edges_filtered (OFFSET path)"
    );
    assert!(
        !region.contains("list_nodes_filtered"),
        "scoped loader must not call list_nodes_filtered (OFFSET path)"
    );
}

#[test]
fn contract_find_edge_by_relationship_id_drops_vertex_joins() {
    let scan = include_str!("../src/adapters/postgres/graph/scan_ops.rs");
    let fn_start = scan
        .find("pub(super) async fn pg_find_edge_by_relationship_id")
        .expect("pg_find_edge_by_relationship_id");
    let body = &scan[fn_start..];
    let end = body
        .find("\n#[cfg(test)]")
        .or_else(|| body.find("\nmod source_prefix"))
        .unwrap_or(body.len().min(2500));
    let region = &body[..end];

    assert!(
        region.contains("coalesce_endpoint") || region.contains("eq_source_id"),
        "relationship lookup must use eq_* / coalesce_endpoint"
    );
    assert!(
        region.contains(".\"EDGE\"") || region.contains(".\\\"EDGE\\\""),
        "relationship lookup must scan child EDGE"
    );
    assert!(
        !region.contains("_ag_label_vertex"),
        "relationship lookup must not join _ag_label_vertex"
    );
    assert!(
        !region.contains("start_id::text"),
        "relationship lookup must not text-cast JOIN on start_id"
    );
}

#[test]
fn contract_refresh_community_index_uses_guarded_path() {
    let persist = include_str!("../src/community_persist.rs");
    let fn_start = persist
        .find("pub async fn refresh_community_index")
        .expect("refresh_community_index");
    let body = &persist[fn_start..];
    let end = body
        .find("pub fn spawn_community_backfill")
        .unwrap_or(body.len().min(600));
    let region = &body[..end];
    assert!(
        region.contains("refresh_community_index_now"),
        "legacy refresh_community_index must route through guarded path"
    );
    assert!(
        !region.contains("detect_and_persist_communities(graph, &CommunityConfig::default())"),
        "must not call unscoped detect directly"
    );
}
