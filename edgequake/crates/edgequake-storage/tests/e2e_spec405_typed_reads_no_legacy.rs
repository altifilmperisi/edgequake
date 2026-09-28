//! #405 sibling gaps: typed authority must not SELECT retired `eq_*_vectors`
//! on unfiltered query / query_filtered fallthrough / get / count / is_empty.
//!
//! Run:
//!   DATABASE_URL=postgresql://edgequake:edgequake_secret@localhost:5432/edgequake \
//!     cargo test -p edgequake-storage --features postgres --test e2e_spec405_typed_reads_no_legacy -- --nocapture
#![cfg(feature = "postgres")]

#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;
#[path = "support/spec091_w3.rs"]
mod w3;

use edgequake_storage::adapters::postgres::PostgresPool;
use edgequake_storage::traits::{MetadataFilter, VectorStorage};
use edgequake_storage::{PgVectorStorage, VECTOR_BACKEND_ENV};
use postgres_test_config::{contract_pg_pool, require_or_skip_postgres};

const DIM: usize = 4;

async fn table_exists(pool: &sqlx::PgPool, table: &str) -> bool {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM information_schema.tables \
         WHERE table_schema = 'public' AND table_name = $1)",
    )
    .bind(table)
    .fetch_one(pool)
    .await
    .unwrap_or(false)
}

#[tokio::test(flavor = "current_thread")]
#[allow(clippy::await_holding_lock)]
async fn e2e_spec405_typed_reads_never_hit_missing_legacy() {
    let Some(cfg) = require_or_skip_postgres("spec405_typed_reads") else {
        return;
    };
    let _env = w3::w3_env_guard().await;
    let pool = contract_pg_pool(&cfg).await;

    let prev_backend = std::env::var(VECTOR_BACKEND_ENV).ok();
    std::env::set_var(VECTOR_BACKEND_ENV, "typed_embeddings");

    let mut pg_cfg = cfg.clone();
    pg_cfg.namespace = format!("spec405_reads_{}", uuid::Uuid::new_v4().as_simple());
    let storage = PgVectorStorage::with_pool(
        PostgresPool::from_existing(pool.clone(), pg_cfg.clone()),
        pg_cfg,
        DIM,
    );
    let legacy_table = storage
        .vectors_table_name()
        .trim_start_matches("public.")
        .to_string();
    assert!(
        !table_exists(&pool, &legacy_table).await,
        "precondition: {legacy_table} must be absent"
    );

    let emb = vec![0.1_f32; DIM];

    // Gap B/C: empty / None filter → empty, no 42P01.
    let empty_filter = storage
        .query_filtered(&emb, 5, None, None)
        .await
        .expect("empty filter under typed");
    assert!(empty_filter.is_empty());

    let empty_mf = storage
        .query_filtered(&emb, 5, None, Some(&MetadataFilter::default()))
        .await
        .expect("default empty MetadataFilter under typed");
    assert!(empty_mf.is_empty());

    // Gap A: vector_type=chunk without workspace → empty (no legacy fallthrough).
    let no_ws = storage
        .query_filtered(
            &emb,
            5,
            None,
            Some(&MetadataFilter {
                vector_type: Some("chunk".into()),
                ..Default::default()
            }),
        )
        .await
        .expect("chunk filter without workspace");
    assert!(
        no_ws.is_empty(),
        "typed + no workspace must not SELECT legacy, got {no_ws:?}"
    );

    // Gap C: unfiltered query → empty.
    let unfiltered = storage
        .query(&emb, 5, None)
        .await
        .expect("unfiltered query under typed");
    assert!(unfiltered.is_empty());

    // Gap D: get / get_by_ids / is_empty / count soft-empty.
    let got = storage
        .get_by_id("missing-id")
        .await
        .expect("get_by_id absent legacy");
    assert!(got.is_none());

    let batch = storage
        .get_by_ids(&["a".into(), "b".into()])
        .await
        .expect("get_by_ids absent legacy");
    assert!(batch.is_empty());

    assert!(
        storage.is_empty().await.expect("is_empty"),
        "absent legacy under typed is empty"
    );
    assert_eq!(
        storage.count().await.expect("count"),
        0,
        "absent legacy under typed counts as 0"
    );
    assert_eq!(
        storage
            .count_workspace_rows("any-ws")
            .await
            .expect("count_workspace_rows"),
        0
    );

    assert!(
        !table_exists(&pool, &legacy_table).await,
        "reads must not CREATE {legacy_table}"
    );

    storage
        .ping()
        .await
        .expect("typed ping uses chunk_embeddings");
    assert!(
        storage
            .get_stored_dimension()
            .await
            .expect("dim reconcile without table")
            .is_none(),
        "absent table must not 42P01 on dimension fallback"
    );

    match prev_backend {
        Some(v) => std::env::set_var(VECTOR_BACKEND_ENV, v),
        None => std::env::remove_var(VECTOR_BACKEND_ENV),
    }
}

#[tokio::test(flavor = "current_thread")]
#[allow(clippy::await_holding_lock)]
async fn e2e_spec405_legacy_backend_absent_table_soft_empty() {
    let Some(cfg) = require_or_skip_postgres("spec405_legacy_absent") else {
        return;
    };
    let _env = w3::w3_env_guard().await;
    let pool = contract_pg_pool(&cfg).await;

    let prev_backend = std::env::var(VECTOR_BACKEND_ENV).ok();
    std::env::set_var(VECTOR_BACKEND_ENV, "legacy_tables");

    let mut pg_cfg = cfg.clone();
    pg_cfg.namespace = format!("spec405_leg_{}", uuid::Uuid::new_v4().as_simple());
    let storage = PgVectorStorage::with_pool(
        PostgresPool::from_existing(pool.clone(), pg_cfg.clone()),
        pg_cfg,
        DIM,
    );
    let legacy_table = storage
        .vectors_table_name()
        .trim_start_matches("public.")
        .to_string();
    assert!(
        !table_exists(&pool, &legacy_table).await,
        "precondition: {legacy_table} must be absent"
    );

    let emb = vec![0.1_f32; DIM];
    assert!(storage
        .query(&emb, 5, None)
        .await
        .expect("legacy query absent")
        .is_empty());
    assert!(storage
        .query_filtered(
            &emb,
            5,
            None,
            Some(&MetadataFilter {
                workspace_id: Some("ws".into()),
                vector_type: Some("chunk".into()),
                ..Default::default()
            }),
        )
        .await
        .expect("legacy query_filtered absent")
        .is_empty());
    assert!(storage
        .get_by_id("x")
        .await
        .expect("legacy get absent")
        .is_none());
    assert!(storage
        .get_by_ids(&["x".into()])
        .await
        .expect("legacy get_by_ids absent")
        .is_empty());
    assert!(storage.is_empty().await.expect("legacy is_empty"));
    assert_eq!(storage.count().await.expect("legacy count"), 0);
    assert_eq!(
        storage
            .count_workspace_rows("ws")
            .await
            .expect("legacy count_workspace_rows"),
        0
    );
    storage.ping().await.expect("legacy ping absent is ok");
    assert!(storage
        .get_stored_dimension()
        .await
        .expect("legacy dim")
        .is_none());

    assert!(
        !table_exists(&pool, &legacy_table).await,
        "reads must not CREATE {legacy_table}"
    );

    match prev_backend {
        Some(v) => std::env::set_var(VECTOR_BACKEND_ENV, v),
        None => std::env::remove_var(VECTOR_BACKEND_ENV),
    }
}
