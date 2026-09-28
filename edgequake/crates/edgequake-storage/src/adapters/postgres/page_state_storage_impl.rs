//! PostgreSQL `document_page_states` (SPEC-151).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::error::{Result, StorageError};
use crate::page_state_storage::*;

pub struct PostgresPageStateStorage {
    pool: PgPool,
}

impl PostgresPageStateStorage {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct StateRow {
    page_state_id: Uuid,
    document_id: Uuid,
    workspace_id: Uuid,
    page_number: i32,
    parse_status: String,
    parse_error: Option<String>,
    parse_attempts: i32,
    parse_method: Option<String>,
    parse_model: Option<String>,
    raw_markdown: Option<String>,
    raw_sha256: Option<String>,
    figures_status: String,
    figures_count: i32,
    figures_error: Option<String>,
    entities_status: String,
    chunk_count: i32,
    failed_chunk_count: i32,
    entities_error: Option<String>,
    last_track_id: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<StateRow> for DocumentPageState {
    fn from(r: StateRow) -> Self {
        Self {
            page_state_id: r.page_state_id,
            document_id: r.document_id,
            workspace_id: r.workspace_id,
            page_number: r.page_number,
            parse_status: r.parse_status,
            parse_error: r.parse_error,
            parse_attempts: r.parse_attempts,
            parse_method: r.parse_method,
            parse_model: r.parse_model,
            raw_markdown: r.raw_markdown,
            raw_sha256: r.raw_sha256,
            figures_status: r.figures_status,
            figures_count: r.figures_count,
            figures_error: r.figures_error,
            entities_status: r.entities_status,
            chunk_count: r.chunk_count,
            failed_chunk_count: r.failed_chunk_count,
            entities_error: r.entities_error,
            last_track_id: r.last_track_id,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[async_trait]
impl PageStateStorage for PostgresPageStateStorage {
    async fn list_page_states(
        &self,
        document_id: Uuid,
        workspace_id: Uuid,
    ) -> Result<Vec<DocumentPageState>> {
        let rows = sqlx::query_as::<_, StateRow>(
            r#"
            SELECT *
            FROM document_page_states
            WHERE document_id = $1 AND workspace_id = $2
            ORDER BY page_number ASC
            "#,
        )
        .bind(document_id)
        .bind(workspace_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?;
        Ok(rows.into_iter().map(DocumentPageState::from).collect())
    }

    async fn upsert_parse_batch(
        &self,
        document_id: Uuid,
        workspace_id: Uuid,
        rows: &[UpsertPageParse],
    ) -> Result<()> {
        if rows.is_empty() {
            return Ok(());
        }
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| StorageError::Database(e.to_string()))?;
        for row in rows {
            let attempts_inc: i32 = if row.increment_attempts { 1 } else { 0 };
            sqlx::query(
                r#"
                INSERT INTO document_page_states (
                    document_id, workspace_id, page_number,
                    parse_status, parse_error, parse_attempts,
                    parse_method, parse_model, raw_markdown, raw_sha256,
                    last_track_id, updated_at
                ) VALUES (
                    $1, $2, $3,
                    $4, $5, $6,
                    $7, $8, $9, $10,
                    $11, NOW()
                )
                ON CONFLICT (document_id, page_number) DO UPDATE SET
                    parse_status = EXCLUDED.parse_status,
                    parse_error = EXCLUDED.parse_error,
                    parse_attempts = document_page_states.parse_attempts + EXCLUDED.parse_attempts,
                    parse_method = COALESCE(EXCLUDED.parse_method, document_page_states.parse_method),
                    parse_model = COALESCE(EXCLUDED.parse_model, document_page_states.parse_model),
                    raw_markdown = COALESCE(EXCLUDED.raw_markdown, document_page_states.raw_markdown),
                    raw_sha256 = COALESCE(EXCLUDED.raw_sha256, document_page_states.raw_sha256),
                    last_track_id = COALESCE(EXCLUDED.last_track_id, document_page_states.last_track_id),
                    updated_at = NOW()
                "#,
            )
            .bind(document_id)
            .bind(workspace_id)
            .bind(row.page_number)
            .bind(&row.status)
            .bind(&row.error)
            .bind(attempts_inc)
            .bind(&row.method)
            .bind(&row.model)
            .bind(&row.raw_markdown)
            .bind(&row.raw_sha256)
            .bind(&row.track_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| StorageError::Database(e.to_string()))?;
        }
        tx.commit()
            .await
            .map_err(|e| StorageError::Database(e.to_string()))?;
        Ok(())
    }

    async fn upsert_figures_batch(
        &self,
        document_id: Uuid,
        workspace_id: Uuid,
        rows: &[UpsertPageFigures],
    ) -> Result<()> {
        if rows.is_empty() {
            return Ok(());
        }
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| StorageError::Database(e.to_string()))?;
        for row in rows {
            sqlx::query(
                r#"
                INSERT INTO document_page_states (
                    document_id, workspace_id, page_number,
                    figures_status, figures_count, figures_error,
                    last_track_id, updated_at
                ) VALUES (
                    $1, $2, $3,
                    $4, $5, $6,
                    $7, NOW()
                )
                ON CONFLICT (document_id, page_number) DO UPDATE SET
                    figures_status = EXCLUDED.figures_status,
                    figures_count = EXCLUDED.figures_count,
                    figures_error = EXCLUDED.figures_error,
                    last_track_id = COALESCE(EXCLUDED.last_track_id, document_page_states.last_track_id),
                    updated_at = NOW()
                "#,
            )
            .bind(document_id)
            .bind(workspace_id)
            .bind(row.page_number)
            .bind(&row.status)
            .bind(row.count)
            .bind(&row.error)
            .bind(&row.track_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| StorageError::Database(e.to_string()))?;
        }
        tx.commit()
            .await
            .map_err(|e| StorageError::Database(e.to_string()))?;
        Ok(())
    }

    async fn upsert_entities_batch(
        &self,
        document_id: Uuid,
        workspace_id: Uuid,
        rows: &[UpsertPageEntities],
    ) -> Result<()> {
        if rows.is_empty() {
            return Ok(());
        }
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| StorageError::Database(e.to_string()))?;
        for row in rows {
            sqlx::query(
                r#"
                INSERT INTO document_page_states (
                    document_id, workspace_id, page_number,
                    entities_status, chunk_count, failed_chunk_count, entities_error,
                    last_track_id, updated_at
                ) VALUES (
                    $1, $2, $3,
                    $4, $5, $6, $7,
                    $8, NOW()
                )
                ON CONFLICT (document_id, page_number) DO UPDATE SET
                    entities_status = EXCLUDED.entities_status,
                    chunk_count = EXCLUDED.chunk_count,
                    failed_chunk_count = EXCLUDED.failed_chunk_count,
                    entities_error = EXCLUDED.entities_error,
                    last_track_id = COALESCE(EXCLUDED.last_track_id, document_page_states.last_track_id),
                    updated_at = NOW()
                "#,
            )
            .bind(document_id)
            .bind(workspace_id)
            .bind(row.page_number)
            .bind(&row.status)
            .bind(row.chunk_count)
            .bind(row.failed_chunk_count)
            .bind(&row.error)
            .bind(&row.track_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| StorageError::Database(e.to_string()))?;
        }
        tx.commit()
            .await
            .map_err(|e| StorageError::Database(e.to_string()))?;
        Ok(())
    }

    async fn delete_for_document(&self, document_id: Uuid, workspace_id: Uuid) -> Result<()> {
        sqlx::query(
            r#"
            DELETE FROM document_page_states
            WHERE document_id = $1 AND workspace_id = $2
            "#,
        )
        .bind(document_id)
        .bind(workspace_id)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?;
        Ok(())
    }
}
