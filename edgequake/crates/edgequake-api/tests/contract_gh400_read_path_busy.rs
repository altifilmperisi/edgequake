//! GH-400 — Documents list must not detoast `content` or omit PG cancel under
//! the interactive read-path envelope (LAW-H2).

#[test]
fn contract_gh400_relational_list_never_detoasts_content() {
    let model = include_str!("../src/document_read_model.rs");
    assert!(
        !model.contains("LEFT(content"),
        "GH-400: relational list must not LEFT(content, …) — detoast holds pool slots"
    );
    assert!(
        !model.contains("LENGTH(content)"),
        "GH-400: relational list must not LENGTH(content) — detoast holds pool slots"
    );
    assert!(
        model.contains("metadata->>'content_summary'")
            || model.contains("metadata->>'content_preview'"),
        "GH-400: preview must come from metadata, not documents.content"
    );
    assert!(
        model.contains("LocalTimeoutTx"),
        "GH-400: relational list must SET LOCAL statement_timeout via LocalTimeoutTx"
    );
    assert!(
        model.contains("LIMIT $3") || model.contains("LIMIT $"),
        "GH-400: relational list must be bounded with LIMIT"
    );
    assert!(
        model.contains("count_relational_document_statuses"),
        "GH-400: status chips must have a GROUP BY aggregate path"
    );
    assert!(
        model.contains("lookup_document_tenant_workspace") && model.contains("LocalTimeoutTx"),
        "GH-400: detail scope SELECT must use LocalTimeoutTx via lookup helper"
    );
    assert!(
        model.contains("::bigint")
            && model.contains("AS content_length")
            && !model.contains("NULLIF(metadata->>'content_length', '')::int"),
        "GH-400: content_length COALESCE must be BIGINT (file_size_bytes is INT8)"
    );
}

#[test]
fn contract_gh400_list_handler_uses_bounded_relational_scan() {
    let list = include_str!("../src/handlers/documents/query/list.rs");
    assert!(
        list.contains("list_relational_document_summaries_limited"),
        "GH-400: list must call the bounded relational scan"
    );
    assert!(
        list.contains("count_relational_document_statuses"),
        "GH-400: list must prefer SQL status aggregate when unfiltered"
    );
    assert!(
        list.contains("MAX_LIST_METADATA_ENTRIES"),
        "GH-400: relational cap must share the interactive envelope constant"
    );
    assert!(
        list.contains("!truncated") && list.contains("count_relational_document_statuses"),
        "GH-400: SQL status chips only when !truncated (honest vs pager total)"
    );
}

#[test]
fn contract_gh400_search_under_guard_and_limited_scan() {
    let search = include_str!("../src/handlers/documents/query/search.rs");
    assert!(
        search.contains("run_with_read_path_guard"),
        "GH-400: document search must run under the read-path guard"
    );
    assert!(
        search.contains("load_scoped_document_metadata_entries_limited"),
        "GH-400: document search must use the limited metadata scan"
    );
    assert!(
        search.contains(r#"(status = 503, description = "Read path busy under ingest load")"#),
        "GH-400: document search OpenAPI must document 503 read_path_busy"
    );
}

#[test]
fn contract_gh400_tenant_workspace_list_openapi_documents_busy() {
    let tenants = include_str!("../src/handlers/workspaces/tenants.rs");
    assert!(
        tenants.contains(r#"(status = 503, description = "Read path busy under ingest load")"#),
        "GH-400: list tenants OpenAPI must document 503 read_path_busy"
    );
    let crud = include_str!("../src/handlers/workspaces/workspace_crud.rs");
    assert!(
        crud.contains(r#"(status = 503, description = "Read path busy under ingest load")"#),
        "GH-400: list workspaces OpenAPI must document 503 read_path_busy"
    );
}

#[test]
fn contract_gh400_workspace_list_stats_cache_only() {
    let crud = include_str!("../src/handlers/workspaces/workspace_crud.rs");
    assert!(
        crud.contains("workspace_stats_cached_only"),
        "GH-400: include_stats must read cache only"
    );
    assert!(
        !crud.contains("workspace_stats_best_effort"),
        "GH-400: include_stats must not call workspace_stats_best_effort (4s nested fetch)"
    );
    let stats = include_str!("../src/handlers/workspaces/stats.rs");
    assert!(
        stats.contains("workspace_stats_cached_only"),
        "GH-400: cache-only helper must exist for list include_stats"
    );
}

#[test]
fn contract_gh400_detail_promote_uses_shared_deadline() {
    let detail = include_str!("../src/handlers/documents/query/detail.rs");
    assert!(
        detail.contains("run_best_effort_interactive"),
        "GH-400: detail projecting promote must use the shared timeout helper"
    );
    assert!(
        detail.contains("lookup_document_tenant_workspace"),
        "GH-400: detail scope fence must use the timed lookup helper"
    );
}

#[test]
fn contract_gh400_read_path_exposes_busy_reason() {
    let read_path = include_str!("../src/read_path.rs");
    assert!(
        read_path.contains("permit_wait"),
        "GH-400: permit wait must tag reason=permit_wait"
    );
    assert!(
        read_path.contains("work_deadline"),
        "GH-400: work overrun must tag reason=work_deadline"
    );
    assert!(
        read_path.contains("read_path_busy_with_reason"),
        "GH-400: busy errors must carry an explicit reason"
    );
}

#[test]
fn contract_gh400_page_enrich_uses_statement_timeout() {
    let enrich = include_str!("../src/services/list_run_enrich.rs");
    assert!(
        enrich.contains("LocalTimeoutTx"),
        "GH-400: query_ready enrich must cancel via LocalTimeoutTx"
    );
    assert!(
        enrich.contains("interactive_statement_timeout_ms"),
        "GH-400: enrich timeout must track the interactive budget"
    );
    assert!(
        enrich.contains("run_best_effort_interactive")
            && enrich.contains("enrich_page_projecting_promote_inner"),
        "GH-400: projecting promote must use run_best_effort_interactive"
    );
}

#[tokio::test]
async fn contract_gh400_permit_released_after_work_deadline() {
    // Mirror of read_path::permit_released_after_work_timeout — kept here so
    // the GH-400 contract suite fails loudly if the guard regresses.
    use edgequake_api::error::ApiError;
    use edgequake_api::read_path::{run_with_read_path_guard, ReadPathDbPermit};
    use std::time::Duration;

    std::env::set_var("EDGEQUAKE_DOCUMENTS_READ_TIMEOUT_MS", "500");
    let permits = ReadPathDbPermit::new(1);
    let _ = run_with_read_path_guard(&permits, || async {
        tokio::time::sleep(Duration::from_millis(800)).await;
        Ok::<_, ApiError>(())
    })
    .await;
    let acquired = permits
        .acquire(Duration::from_millis(50))
        .await
        .expect("GH-400: permit must be free after work_deadline");
    drop(acquired);
    std::env::remove_var("EDGEQUAKE_DOCUMENTS_READ_TIMEOUT_MS");
}
