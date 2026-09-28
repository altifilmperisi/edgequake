//! SPEC-061 — unfiltered ANN `query` p95 + HNSW EXPLAIN.
#![cfg(feature = "postgres")]

#[path = "support/perf_harness.rs"]
mod perf_harness;
#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;
#[path = "support/spec091_w3.rs"]
mod w3;

use edgequake_storage::traits::VectorStorage;
use edgequake_storage::{PgVectorStorage, PostgresConfig, VECTOR_BACKEND_ENV};
use perf_harness::{
    assert_plan_uses_index, finish_report, join_plan_rows, plan_has_buffers, PlanKind,
};
use std::time::Instant;

const DIM: usize = 64;
const ROW_COUNT: usize = 10_000;
const TOP_K: usize = 20;
const SAMPLES: usize = 20;

fn emb(seed: f32) -> Vec<f32> {
    (0..DIM)
        .map(|i| ((i as f32 + seed) * 0.017).sin())
        .collect()
}

#[tokio::test(flavor = "current_thread")]
#[allow(clippy::await_holding_lock)]
async fn e2e_spec061_unfiltered_ann_p95_and_hnsw_explain() {
    let Some(config) = postgres_test_config::require_or_skip_postgres("perf061_ann") else {
        return;
    };
    // Legacy eq_*_vectors unfiltered ANN path (#405: typed query() returns empty).
    let _env = w3::w3_env_guard().await;
    let prev_backend = std::env::var(VECTOR_BACKEND_ENV).ok();
    std::env::set_var(VECTOR_BACKEND_ENV, "legacy_tables");

    let storage = PgVectorStorage::with_dimension(config.clone(), DIM);
    storage.initialize().await.expect("init");

    eprintln!("SPEC-061 unfiltered ANN: seeding {ROW_COUNT}…");
    for batch_start in (0..ROW_COUNT).step_by(2000) {
        let end = (batch_start + 2000).min(ROW_COUNT);
        let batch: Vec<_> = (batch_start..end)
            .map(|i| {
                (
                    format!("ann061-{i}"),
                    emb(i as f32),
                    serde_json::json!({"type": "chunk", "document_id": format!("d{}", i / 10)}),
                )
            })
            .collect();
        storage.upsert(&batch).await.expect("upsert");
    }

    let _ = storage.query(&emb(0.0), TOP_K, None).await.expect("warm");
    let mut samples = Vec::with_capacity(SAMPLES);
    for s in 0..SAMPLES {
        let start = Instant::now();
        let hits = storage
            .query(&emb(s as f32 * 11.0), TOP_K, None)
            .await
            .expect("query");
        samples.push(start.elapsed());
        assert_eq!(hits.len(), TOP_K);
    }
    finish_report(
        "vector_query_unfiltered",
        &samples,
        100.0,
        "hnsw",
        true,
        format!("N={ROW_COUNT}"),
    );

    assert_hnsw_explain(&config).await;
    let _ = storage.clear().await;

    match prev_backend {
        Some(v) => std::env::set_var(VECTOR_BACKEND_ENV, v),
        None => std::env::remove_var(VECTOR_BACKEND_ENV),
    }
}

async fn assert_hnsw_explain(config: &PostgresConfig) {
    let pool = postgres_test_config::contract_pg_pool(config).await;
    let table = format!("public.eq_{}_vectors", config.table_prefix());
    let emb = format!(
        "[{}]",
        (0..DIM)
            .map(|i| format!("{:.4}", (i as f32) * 0.01))
            .collect::<Vec<_>>()
            .join(",")
    );
    let sql = format!(
        r#"EXPLAIN (ANALYZE, BUFFERS, FORMAT TEXT)
           SELECT id FROM {table}
           ORDER BY embedding <=> $1::vector
           LIMIT 20"#
    );
    let plan_rows: Vec<(String,)> = sqlx::query_as(&sql)
        .bind(&emb)
        .fetch_all(&pool)
        .await
        .expect("EXPLAIN");
    let plan = join_plan_rows(plan_rows);
    assert_plan_uses_index(&plan, &[PlanKind::Hnsw, PlanKind::Index]);
    assert!(plan_has_buffers(&plan), "buffers missing:\n{plan}");
    eprintln!("OK SPEC-061 unfiltered ANN EXPLAIN:\n{plan}");
}
