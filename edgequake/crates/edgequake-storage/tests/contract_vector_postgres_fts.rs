//! SPEC-023 I10 — Postgres native FTS on vector chunk content.

#[test]
fn contract_postgres_vector_fts_joins_shared_kv_for_chunk_text() {
    let fts = include_str!("../src/adapters/postgres/vector/fts.rs");
    assert!(fts.contains("ts_rank_cd"));
    assert!(fts.contains("websearch_to_tsquery"));
    assert!(fts.contains("chunk_kv_table_name"));
    assert!(fts.contains("k.value->>'content'"));
    assert!(fts.contains("LEFT JOIN"));
    assert!(fts.contains("content_tsv"));
    // SPEC-058: empty generated tsv must not block KV fallthrough.
    assert!(fts.contains("NULLIF(v.content_tsv"));
    assert!(fts.contains("content_ref"));
    // X-05: honest ts_rank_cd naming + configurable language.
    assert!(fts.contains("EDGEQUAKE_FTS_LANGUAGE") || fts.contains("FTS_LANGUAGE_ENV"));
    assert!(fts.contains("fts_language_from_env"));
    assert!(
        fts.contains("cover-density") && fts.contains("not") && fts.contains("Okapi BM25"),
        "X-05: docs must not market postgres FTS as BM25"
    );
}

#[test]
fn contract_spec058_upsert_populates_content_tsv() {
    let impl_src = include_str!("../src/adapters/postgres/vector/storage_impl.rs");
    let ddl = include_str!("../src/adapters/postgres/vector/ddl.rs");
    let migration = include_str!("../../../migrations/091_vector_content_tsv_writable.sql");
    assert!(
        impl_src.contains("content_tsv = EXCLUDED.content_tsv"),
        "upsert must write content_tsv"
    );
    assert!(
        !ddl.contains("GENERATED ALWAYS AS"),
        "ddl must not recreate generated content_tsv"
    );
    assert!(
        migration.contains("ADD COLUMN content_tsv TSVECTOR"),
        "migration 091 must add writable content_tsv"
    );
}

#[test]
fn contract_workspace_vector_uses_shared_chunk_kv_table() {
    let ws = include_str!("../src/adapters/postgres/workspace_vector.rs");
    assert!(
        ws.contains("qualified_kv_table"),
        "workspace vectors must join the shared default KV for FTS"
    );
}

#[test]
fn contract_postgres_vector_fts_filters_modality_metadata() {
    let fts = include_str!("../src/adapters/postgres/vector/fts.rs");
    let storage_impl = include_str!("../src/adapters/postgres/vector/storage_impl.rs");
    assert!(fts.contains("Failed to bind modalities"));
    assert!(storage_impl.contains("Failed to bind modalities"));
    assert!(fts.contains("build_sql_with_alias"));
}

#[test]
fn contract_vector_ddl_adds_content_tsv() {
    let ddl = include_str!("../src/adapters/postgres/vector/ddl.rs");
    assert!(ddl.contains("content_tsv"));
    assert!(ddl.contains("ensure_content_fts"));
}

/// #405 — typed backend must route FTS to chunks.content_tsv, not eq_*_vectors.
#[test]
fn contract_spec405_text_search_branches_typed() {
    let storage_impl = include_str!("../src/adapters/postgres/vector/storage_impl.rs");
    let fts = include_str!("../src/adapters/postgres/vector/fts.rs");
    assert!(
        storage_impl.contains("vector_backend_reads_typed"),
        "text_search_filtered must branch on typed read authority"
    );
    assert!(
        storage_impl.contains("typed_chunks_text_search_filtered"),
        "text_search_filtered must call typed chunk FTS under typed backend"
    );
    assert!(
        storage_impl.contains("legacy_vectors_relation_exists_cached"),
        "legacy rollback must probe before SELECT to avoid 42P01"
    );
    assert!(
        fts.contains("typed_chunks_text_search_filtered"),
        "fts.rs must implement typed_chunks_text_search_filtered"
    );
    assert!(fts.contains("TYPED_CHUNKS_FTS_SQL") || fts.contains("c.content_tsv"));
    assert!(fts.contains("legacy_chunk_key"));
    assert!(fts.contains("websearch_to_tsquery('english'"));
    assert!(
        fts.contains("Non-chunk vector_type short-circuit")
            || fts.contains("eq_ignore_ascii_case(\"chunk\")"),
        "typed FTS must short-circuit non-chunk vector_type"
    );
    // Typed SQL constant must not interpolate the legacy vectors table name.
    let typed_sql_region = fts
        .split("TYPED_CHUNKS_FTS_SQL")
        .nth(1)
        .unwrap_or("")
        .split("impl PgVectorStorage")
        .next()
        .unwrap_or("");
    assert!(
        !typed_sql_region.contains("self.table_name") && !typed_sql_region.contains("eq_%_vectors"),
        "typed FTS SQL must not reference legacy vectors relation"
    );
}

/// #405 sibling — typed query_filtered must not fall through to legacy ANN SQL.
#[test]
fn contract_spec405_typed_query_filtered_never_falls_through() {
    let storage_impl = include_str!("../src/adapters/postgres/vector/storage_impl.rs");
    // After typed short-circuits, must return Ok(Vec::new()) unconditionally.
    assert!(
        storage_impl.contains("Typed authority: after typed short-circuits, never SELECT legacy")
            || storage_impl.contains("never SELECT legacy"),
        "query_filtered typed block must document/enforce no-legacy fallthrough"
    );
    // Unfiltered query under typed returns empty (Gap C).
    assert!(
        storage_impl.contains("unfiltered query returns empty")
            || storage_impl.contains("use query_filtered with workspace"),
        "query() must short-circuit under typed authority"
    );
    // Empty filter under typed must not delegate to query() legacy path.
    assert!(
        storage_impl.contains("typed ANN is workspace-scoped")
            || storage_impl.contains("empty filter → empty"),
        "empty MetadataFilter under typed must return empty without legacy SELECT"
    );
}

/// #405 residual — every legacy SELECT must probe via skip_legacy_read_if_absent.
#[test]
fn contract_spec405_legacy_reads_probe_before_select() {
    let storage_impl = include_str!("../src/adapters/postgres/vector/storage_impl.rs");
    let ddl = include_str!("../src/adapters/postgres/vector/ddl.rs");
    let migration = include_str!("../src/adapters/postgres/vector/migration.rs");
    let vector_mod = include_str!("../src/adapters/postgres/vector/mod.rs");
    assert!(
        vector_mod.contains("fn skip_legacy_read_if_absent"),
        "shared probe-before-read helper must exist"
    );
    for needle in [
        "skip_legacy_read_if_absent(\"query\")",
        "skip_legacy_read_if_absent(\"query_filtered\")",
        "skip_legacy_read_if_absent(\"get_by_id\")",
        "skip_legacy_read_if_absent(\"get_by_ids\")",
        "skip_legacy_read_if_absent(\"is_empty\")",
        "skip_legacy_read_if_absent(\"count\")",
        "skip_legacy_read_if_absent(\"ping\")",
    ] {
        assert!(
            storage_impl.contains(needle),
            "storage_impl must probe before {needle}"
        );
    }
    assert!(
        ddl.contains("skip_legacy_read_if_absent(\"count_workspace_rows\")"),
        "count_workspace_rows must probe regardless of backend"
    );
    assert!(
        ddl.contains("legacy_vectors_relation_exists_cached")
            && ddl.contains("legacy_chunk_ddl_retired"),
        "legacy_chunk_ddl_retired must probe before COUNT on eq_*_vectors"
    );
    assert!(
        migration.contains("if !self.table_exists().await?"),
        "get_stored_dimension fallback must not SELECT a missing table"
    );
}
