//! SPEC-047 MV-32 — modality filter wired + e2e tested on all vector backends.

#[path = "support/metadata_filter_modality_contract.rs"]
mod metadata_filter_modality_contract;

#[path = "support/e2e_fixtures.rs"]
mod e2e_fixtures;

use e2e_fixtures::generate_namespace;
use edgequake_storage::{MemoryVectorStorage, VectorStorage};

use metadata_filter_modality_contract::{
    assert_query_filtered_chart_modality, assert_query_filtered_excludes_missing_modality,
    assert_text_search_chart_modality_when_supported, assert_text_search_empty_when_unsupported,
    chart_chunk_filter,
};

const TEST_DIM: usize = 384;

async fn memory_vector() -> MemoryVectorStorage {
    let storage = MemoryVectorStorage::new(generate_namespace(), TEST_DIM);
    storage.initialize().await.unwrap();
    storage
}

#[tokio::test]
async fn memory_vector_query_filtered_chart_modality_e2e() {
    let storage = memory_vector().await;
    assert_query_filtered_chart_modality(&storage).await;
}

#[tokio::test]
async fn memory_vector_query_filtered_strict_missing_modality_e2e() {
    let storage = memory_vector().await;
    assert_query_filtered_excludes_missing_modality(&storage).await;
}

#[tokio::test]
async fn memory_vector_text_search_empty_without_fts_e2e() {
    let storage = memory_vector().await;
    assert_text_search_empty_when_unsupported(&storage).await;
}

#[tokio::test]
async fn memory_vector_emulated_fts_chart_modality_e2e() {
    let storage =
        MemoryVectorStorage::new(generate_namespace(), TEST_DIM).with_emulated_native_fts(true);
    storage.initialize().await.unwrap();
    assert_text_search_chart_modality_when_supported(&storage).await;
}

#[test]
fn metadata_filter_predicate_matches_sql_for_modalities() {
    let mf = chart_chunk_filter();
    assert!(mf.matches(&serde_json::json!({
        "type": "chunk",
        "modality": "chart"
    })));
    assert!(!mf.matches(&serde_json::json!({
        "type": "chunk",
        "modality": "figure"
    })));
    assert!(!mf.matches(&serde_json::json!({
        "type": "chunk"
    })));

    let sql = mf.build_sql(false, 2);
    assert!(
        sql.conditions
            .iter()
            .any(|c| c.contains("metadata->>'modality'")),
        "SQL builder must emit modality condition"
    );
}

#[cfg(feature = "postgres")]
#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;

#[cfg(feature = "postgres")]
#[path = "support/spec091_w3.rs"]
mod w3;

#[cfg(feature = "postgres")]
mod postgres_modality_e2e {
    use super::{
        assert_query_filtered_chart_modality, assert_query_filtered_excludes_missing_modality,
        assert_text_search_chart_modality_when_supported, postgres_test_config, w3,
    };
    use edgequake_storage::{PgVectorStorage, VectorStorage, VECTOR_BACKEND_ENV};

    async fn postgres_vector() -> Option<(PgVectorStorage, Option<String>)> {
        // Legacy eq_*_vectors modality/FTS fixtures (typed path is #405 / chunks).
        let prev = std::env::var(VECTOR_BACKEND_ENV).ok();
        std::env::set_var(VECTOR_BACKEND_ENV, "legacy_tables");
        let config = postgres_test_config::contract_postgres_config("modality_filter")?;
        let storage = PgVectorStorage::with_dimension(config, super::TEST_DIM);
        storage.initialize().await.ok()?;
        Some((storage, prev))
    }

    fn restore_backend(prev: Option<String>) {
        match prev {
            Some(v) => std::env::set_var(VECTOR_BACKEND_ENV, v),
            None => std::env::remove_var(VECTOR_BACKEND_ENV),
        }
    }

    #[tokio::test(flavor = "current_thread")]
    #[allow(clippy::await_holding_lock)]
    async fn postgres_vector_query_filtered_chart_modality_e2e() {
        let _env = w3::w3_env_guard().await;
        let Some((storage, prev)) = postgres_vector().await else {
            eprintln!("Skipping: DATABASE_URL/POSTGRES_PASSWORD not set");
            return;
        };
        assert_query_filtered_chart_modality(&storage).await;
        let _ = storage.clear().await;
        restore_backend(prev);
    }

    #[tokio::test(flavor = "current_thread")]
    #[allow(clippy::await_holding_lock)]
    async fn postgres_vector_query_filtered_strict_missing_modality_e2e() {
        let _env = w3::w3_env_guard().await;
        let Some((storage, prev)) = postgres_vector().await else {
            eprintln!("Skipping: DATABASE_URL/POSTGRES_PASSWORD not set");
            return;
        };
        assert_query_filtered_excludes_missing_modality(&storage).await;
        let _ = storage.clear().await;
        restore_backend(prev);
    }

    #[tokio::test(flavor = "current_thread")]
    #[allow(clippy::await_holding_lock)]
    async fn postgres_vector_native_fts_chart_modality_e2e() {
        let _env = w3::w3_env_guard().await;
        let Some((storage, prev)) = postgres_vector().await else {
            eprintln!("Skipping: DATABASE_URL/POSTGRES_PASSWORD not set");
            return;
        };
        assert_text_search_chart_modality_when_supported(&storage).await;
        let _ = storage.clear().await;
        restore_backend(prev);
    }
}
