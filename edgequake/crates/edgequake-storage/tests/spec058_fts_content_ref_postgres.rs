//! SPEC-058 Wave 3 — FTS hits when chunk body lives only in KV (content_ref).
//!
//! Post migration 125, per-workspace KV tables are retired on typed fleets.
//! This test exercises the legacy rollback path only when the KV relation
//! still exists. Typed-authority FTS is covered by `e2e_spec405_typed_fts_no_legacy`.

#![cfg(feature = "postgres")]

#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;
#[path = "support/spec091_w3.rs"]
mod w3;

use edgequake_storage::adapters::postgres::PostgresPool;
use edgequake_storage::serving_fence::SERVING_FENCE_ENV;
use edgequake_storage::traits::{KVStorage, MetadataFilter, VectorStorage};
use edgequake_storage::{PgVectorStorage, PostgresKVStorage, VECTOR_BACKEND_ENV};

const DIM: usize = 4;

async fn relation_exists(pool: &sqlx::PgPool, qualified: &str) -> bool {
    let name = qualified.trim_start_matches("public.");
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM information_schema.tables \
         WHERE table_schema = 'public' AND table_name = $1)",
    )
    .bind(name)
    .fetch_one(pool)
    .await
    .unwrap_or(false)
}

#[tokio::test(flavor = "current_thread")]
#[allow(clippy::await_holding_lock)]
async fn spec058_fts_hits_content_ref_only_chunks() {
    let Some(config) = postgres_test_config::contract_postgres_config("spec058_fts_content_ref")
    else {
        eprintln!("SKIP spec058_fts: DATABASE_URL or POSTGRES_PASSWORD not set");
        return;
    };

    let _env = w3::w3_env_guard().await;

    let prev_backend = std::env::var(VECTOR_BACKEND_ENV).ok();
    let prev_fence = std::env::var(SERVING_FENCE_ENV).ok();
    std::env::set_var(VECTOR_BACKEND_ENV, "legacy_tables");
    std::env::set_var(SERVING_FENCE_ENV, "off");

    let kv = PostgresKVStorage::new(config.clone());
    kv.initialize().await.expect("kv init");

    let pool = PostgresPool::new(config.clone());
    pool.initialize().await.expect("pool init");
    let pg = pool.get().await.expect("pg");
    let kv_table = config.qualified_kv_table();
    if !relation_exists(&pg, &kv_table).await {
        eprintln!(
            "SKIP spec058_fts: KV relation {kv_table} absent (migration 125 / typed fleet) — \
             see e2e_spec405_typed_fts_no_legacy for typed FTS"
        );
        match prev_backend {
            Some(v) => std::env::set_var(VECTOR_BACKEND_ENV, v),
            None => std::env::remove_var(VECTOR_BACKEND_ENV),
        }
        match prev_fence {
            Some(v) => std::env::set_var(SERVING_FENCE_ENV, v),
            None => std::env::remove_var(SERVING_FENCE_ENV),
        }
        return;
    }

    let vectors = PgVectorStorage::with_pool_and_dimension(pool, config.clone(), DIM)
        .with_chunk_kv_table(kv_table);
    vectors.initialize().await.expect("vector init");

    let chunk_id = "spec058-fts-chunk-0";
    let body = "quantum entanglement and photon polarization uniquephrase058";
    kv.upsert(&[(
        chunk_id.to_string(),
        serde_json::json!({"content": body, "type": "chunk"}),
    )])
    .await
    .expect("kv upsert");

    vectors
        .upsert(&[(
            chunk_id.to_string(),
            vec![0.1, 0.2, 0.3, 0.4],
            serde_json::json!({
                "type": "chunk",
                "content_ref": chunk_id,
                "document_id": "spec058-doc"
            }),
        )])
        .await
        .expect("vector upsert");

    let filter = MetadataFilter {
        vector_type: Some("chunk".to_string()),
        ..Default::default()
    };
    let hits = vectors
        .text_search_filtered("uniquephrase058", 10, None, Some(&filter))
        .await
        .expect("fts");

    assert!(
        hits.iter().any(|h| h.id == chunk_id && h.score > 0.0),
        "FTS must rank content_ref chunk via populated content_tsv, got {hits:?}"
    );

    let _ = vectors.delete(&[chunk_id.to_string()]).await;
    let _ = kv.delete(&[chunk_id.to_string()]).await;

    match prev_backend {
        Some(v) => std::env::set_var(VECTOR_BACKEND_ENV, v),
        None => std::env::remove_var(VECTOR_BACKEND_ENV),
    }
    match prev_fence {
        Some(v) => std::env::set_var(SERVING_FENCE_ENV, v),
        None => std::env::remove_var(SERVING_FENCE_ENV),
    }
}
