//! SPEC-061/062 — concurrent FTS stress.
//!
//! pg16: N=8 ≤2×; pg17/18: N=16 ≤1.5× (vs single-client p95).
//! Scale via `EDGEQUAKE_PERF_SCALE=prod|large`.
#![cfg(feature = "postgres")]

#[path = "support/perf_harness.rs"]
mod perf_harness;
#[path = "support/perf_stress.rs"]
mod perf_stress;
#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;
#[path = "support/spec091_w3.rs"]
mod w3;

use edgequake_storage::adapters::postgres::PostgresPool;
use edgequake_storage::traits::{KVStorage, MetadataFilter, VectorStorage};
use edgequake_storage::{PgVectorStorage, PostgresKVStorage, VECTOR_BACKEND_ENV};
use perf_harness::{finish_report, percentile_p95_ms};
use perf_stress::{
    fts_scale, perf_scale, stress_clients, stress_mult, stress_pool_max, with_stress_pool,
    PerfScale,
};
use std::sync::Arc;
use std::time::Instant;

fn emb(dim: usize, i: usize) -> Vec<f32> {
    (0..dim).map(|d| ((d + i) as f32 * 0.013).sin()).collect()
}

#[tokio::test(flavor = "current_thread")]
#[allow(clippy::await_holding_lock)]
async fn e2e_spec061_stress_concurrent_fts() {
    let scale = perf_scale();
    let fts = fts_scale(scale);
    let clients = stress_clients();
    let mult = stress_mult();
    let pool_n = stress_pool_max(clients);

    let Some(base) = postgres_test_config::require_or_skip_postgres("stress061_fts") else {
        return;
    };
    let _env = w3::w3_env_guard().await;
    // Legacy eq_*_vectors FTS stress path (not typed chunks FTS / #405).
    let prev_backend = std::env::var(VECTOR_BACKEND_ENV).ok();
    std::env::set_var(VECTOR_BACKEND_ENV, "legacy_tables");

    let config = with_stress_pool(base, clients);
    let kv = PostgresKVStorage::new(config.clone());
    kv.initialize().await.expect("kv");
    let pool = PostgresPool::new(config.clone());
    pool.initialize().await.expect("pool");
    let vectors = Arc::new(
        PgVectorStorage::with_pool_and_dimension(pool, config.clone(), fts.dim)
            .with_chunk_kv_table(config.qualified_kv_table()),
    );
    vectors.initialize().await.expect("vec");

    for batch_start in (0..fts.rows).step_by(fts.batch_size) {
        let end = (batch_start + fts.batch_size).min(fts.rows);
        let mut kv_batch = Vec::new();
        let mut vec_batch = Vec::new();
        for i in batch_start..end {
            let id = format!("sfts-{i}");
            let phrase = if i % 40 == 0 {
                "uniquephrase061 stress fts"
            } else {
                "ordinary filler body"
            };
            kv_batch.push((
                id.clone(),
                serde_json::json!({"content": phrase, "type": "chunk"}),
            ));
            vec_batch.push((
                id.clone(),
                emb(fts.dim, i),
                serde_json::json!({
                    "type": "chunk",
                    "content_ref": id,
                    "workspace_id": "ws-fts",
                }),
            ));
        }
        kv.upsert(&kv_batch).await.expect("kv");
        vectors.upsert(&vec_batch).await.expect("vec");
    }

    let mf = MetadataFilter {
        workspace_id: Some("ws-fts".into()),
        vector_type: Some("chunk".into()),
        ..Default::default()
    };

    let mut single = Vec::new();
    for _ in 0..20 {
        let start = Instant::now();
        let hits = vectors
            .text_search_filtered("uniquephrase061", 20, None, Some(&mf))
            .await
            .expect("fts single");
        single.push(start.elapsed());
        assert!(!hits.is_empty());
    }
    let single_p95 = percentile_p95_ms(&single);

    let qpc = fts.queries_per_client;
    let mut handles = Vec::new();
    for _ in 0..clients {
        let vectors = Arc::clone(&vectors);
        let mf = mf.clone();
        handles.push(tokio::spawn(async move {
            let mut samples = Vec::new();
            for _ in 0..qpc {
                let start = Instant::now();
                let hits = vectors
                    .text_search_filtered("uniquephrase061", 20, None, Some(&mf))
                    .await
                    .expect("fts");
                samples.push(start.elapsed());
                assert!(!hits.is_empty());
            }
            samples
        }));
    }
    let mut all = Vec::new();
    for h in handles {
        all.extend(h.await.expect("join"));
    }
    // Q-FTS=200ms @10k; prod @50k uses 500ms floor (same class as Q1-d).
    let floor = match scale {
        PerfScale::Prod | PerfScale::Large => 500.0,
        PerfScale::Default => 200.0,
    };
    let budget = (floor * mult).max(single_p95 * mult);
    finish_report(
        "stress_concurrent_fts",
        &all,
        budget,
        "gin",
        false,
        format!(
            "scale={} clients={clients} rows={} q/client={qpc} pool={pool_n} single_p95={single_p95:.2} mult={mult} noise_ok",
            scale.as_str(),
            fts.rows,
        ),
    );
    let _ = vectors.clear().await;

    match prev_backend {
        Some(v) => std::env::set_var(VECTOR_BACKEND_ENV, v),
        None => std::env::remove_var(VECTOR_BACKEND_ENV),
    }
}
