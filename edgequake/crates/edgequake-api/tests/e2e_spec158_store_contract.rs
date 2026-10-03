//! SPEC-158 — `FederationStore` contract (memory always; PostgreSQL when
//! `EDGEQUAKE_SPEC158_PG_URL` points at a scratch database) + migration 164 behaviour.
#![cfg(feature = "postgres")]

mod common;

use std::sync::Arc;

use edgequake_api::services::federation::pg_store::PgFederationStore;
use edgequake_api::services::federation::MemoryFederationStore;
use serial_test::serial;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Executor, PgPool};
use uuid::Uuid;

const MIGRATION_164: &str = include_str!("../../../migrations/164_spec158_federation.sql");
const MIGRATION_165: &str = include_str!("../../../migrations/165_spec158_access_jti.sql");
const SCHEMA: &str = "spec158_contract";

#[tokio::test]
async fn memory_store_satisfies_contract() {
    common::federation_contract::run(Arc::new(MemoryFederationStore::new()), Uuid::new_v4()).await;
}

/// Isolated schema with the pre-164 tables 164 depends on (copied from migration 001).
async fn scratch_pool() -> Option<PgPool> {
    let url = std::env::var("EDGEQUAKE_SPEC158_PG_URL").ok()?;
    let pool = PgPoolOptions::new()
        .max_connections(8)
        .after_connect(|conn, _| {
            Box::pin(async move {
                conn.execute(format!("SET search_path = {SCHEMA}").as_str())
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await
        .expect("connect EDGEQUAKE_SPEC158_PG_URL");
    for stmt in [
        format!("DROP SCHEMA IF EXISTS {SCHEMA} CASCADE"),
        format!("CREATE SCHEMA {SCHEMA}"),
    ] {
        sqlx::query(&stmt).execute(&pool).await.unwrap();
    }
    pool.execute(
        "CREATE TABLE tenants (tenant_id UUID PRIMARY KEY DEFAULT gen_random_uuid());
         CREATE TABLE workspaces (workspace_id UUID PRIMARY KEY DEFAULT gen_random_uuid());
         CREATE TABLE users (user_id UUID PRIMARY KEY DEFAULT gen_random_uuid());
         CREATE TABLE memberships (
            membership_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            tenant_id UUID NOT NULL REFERENCES tenants(tenant_id) ON DELETE CASCADE,
            workspace_id UUID REFERENCES workspaces(workspace_id) ON DELETE CASCADE,
            user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
            role VARCHAR(50) NOT NULL DEFAULT 'member',
            is_active BOOLEAN NOT NULL DEFAULT TRUE,
            joined_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            metadata JSONB,
            CONSTRAINT memberships_unique UNIQUE (user_id, tenant_id, workspace_id));",
    )
    .await
    .unwrap();
    Some(pool)
}

async fn apply_164(pool: &PgPool) {
    // The shipped files pin `search_path = public`; the scratch run keeps its own schema.
    for (name, sql) in [("164", MIGRATION_164), ("165", MIGRATION_165)] {
        let sql = sql.replace("SET search_path = public;", "");
        pool.execute(sql.as_str())
            .await
            .unwrap_or_else(|e| panic!("migration {name} applies: {e}"));
    }
}

// EC-158-19 membership_unique + migration idempotency, against real PostgreSQL.
#[tokio::test]
#[serial(spec158_pg)]
async fn migration_164_dedupes_and_enforces_null_workspace_uniqueness() {
    let Some(pool) = scratch_pool().await else {
        eprintln!("skip: set EDGEQUAKE_SPEC158_PG_URL to run PostgreSQL contract tests");
        return;
    };
    let (user, tenant) = (Uuid::new_v4(), Uuid::new_v4());
    sqlx::query("INSERT INTO users (user_id) VALUES ($1)")
        .bind(user)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO tenants (tenant_id) VALUES ($1)")
        .bind(tenant)
        .execute(&pool)
        .await
        .unwrap();
    // Pre-164 the NULL workspace made these distinct → duplicates were possible.
    for _ in 0..3 {
        sqlx::query("INSERT INTO memberships (tenant_id, user_id) VALUES ($1, $2)")
            .bind(tenant)
            .bind(user)
            .execute(&pool)
            .await
            .unwrap();
    }

    apply_164(&pool).await;
    apply_164(&pool).await; // idempotent

    let left: i64 = sqlx::query_scalar("SELECT count(*) FROM memberships")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(left, 1, "duplicates collapsed");
    let dup = sqlx::query("INSERT INTO memberships (tenant_id, user_id) VALUES ($1, $2)")
        .bind(tenant)
        .bind(user)
        .execute(&pool)
        .await;
    assert!(
        dup.is_err(),
        "NULLS NOT DISTINCT must reject a second tenant-wide membership"
    );
}

#[tokio::test]
#[serial(spec158_pg)]
async fn postgres_store_satisfies_contract() {
    let Some(pool) = scratch_pool().await else {
        eprintln!("skip: set EDGEQUAKE_SPEC158_PG_URL to run PostgreSQL contract tests");
        return;
    };
    apply_164(&pool).await;
    let user = Uuid::new_v4();
    sqlx::query("INSERT INTO users (user_id) VALUES ($1)")
        .bind(user)
        .execute(&pool)
        .await
        .unwrap();
    common::federation_contract::run(Arc::new(PgFederationStore::new(pool)), user).await;
}
