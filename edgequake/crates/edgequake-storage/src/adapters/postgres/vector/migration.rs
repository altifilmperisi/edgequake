//! Dimension migration helpers for [`super::PgVectorStorage`].
//!
//! # First principles (pgvector + SPEC-058)
//!
//! 1. **Schema is truth** — a `vector(n)` column cannot hold another model's width.
//! 2. **Empty ≠ data** — recreating an empty table is schema heal, not wipe.
//! 3. **Non-empty mismatch** — never silent `DROP` (SPEC-058) unless the operator
//!    sets `EDGEQUAKE_ALLOW_VECTOR_TABLE_REBUILD=1`.
//! 4. **Boot ≠ workspace write** — server start may **keep** an existing default
//!    table (`PreferExisting`) so Acc/other workspace tables stay reachable while
//!    the global embedding provider differs; workspace open stays **fail-closed**.

use crate::dimension_policy::{
    decide_dimension_action, DimensionAction, DimensionEnsureOutcome, DimensionReconcilePolicy,
};
use crate::error::{Result, StorageError};

use super::PgVectorStorage;

impl PgVectorStorage {
    /// Get the dimension of the vector column in the database table.
    pub async fn get_stored_dimension(&self) -> Result<Option<usize>> {
        let pool = match self.pool.get().await {
            Ok(p) => p,
            Err(_) => return Ok(None),
        };

        let (schema, table) = if self.table_name.contains('.') {
            let parts: Vec<&str> = self.table_name.split('.').collect();
            (parts[0], parts[1])
        } else {
            ("public", self.table_name.as_str())
        };

        let sql = r#"
            SELECT a.atttypmod
            FROM pg_attribute a
            JOIN pg_class c ON a.attrelid = c.oid
            JOIN pg_namespace n ON c.relnamespace = n.oid
            WHERE n.nspname = $1
              AND c.relname = $2
              AND a.attname = 'embedding'
              AND a.atttypmod > 0
        "#;

        let result: Option<(i32,)> = sqlx::query_as(sql)
            .bind(schema)
            .bind(table)
            .fetch_optional(&pool)
            .await
            .map_err(|e| {
                StorageError::Database(format!("Failed to get column dimension: {}", e))
            })?;

        match result {
            Some((dim,)) if dim > 0 => {
                tracing::debug!(
                    table = %self.table_name,
                    dimension = dim,
                    "Got column dimension from pg_attribute.atttypmod"
                );
                Ok(Some(dim as usize))
            }
            _ => {
                if !self.table_exists().await? {
                    return Ok(None);
                }
                let fallback_sql = format!(
                    "SELECT vector_dims(embedding) as dim FROM {} LIMIT 1",
                    self.table_name
                );

                let fallback_result: Option<(i32,)> = sqlx::query_as(&fallback_sql)
                    .fetch_optional(&pool)
                    .await
                    .ok()
                    .flatten();

                match fallback_result {
                    Some((dim,)) if dim > 0 => {
                        tracing::debug!(
                            table = %self.table_name,
                            dimension = dim,
                            "Got dimension from stored vector (fallback)"
                        );
                        Ok(Some(dim as usize))
                    }
                    _ => Ok(None),
                }
            }
        }
    }

    /// True when the vectors table is missing or has zero rows (safe to recreate).
    pub async fn vector_table_is_empty(&self) -> Result<bool> {
        if !self.table_exists().await? {
            return Ok(true);
        }
        let pool = self.pool.get().await?;
        let sql = format!("SELECT COUNT(*)::bigint FROM {}", self.table_name);
        let count: i64 = sqlx::query_scalar(&sql)
            .fetch_one(&pool)
            .await
            .map_err(|e| {
                StorageError::Database(format!(
                    "Failed to count rows on {}: {}",
                    self.table_name, e
                ))
            })?;
        Ok(count == 0)
    }

    /// Reconcile stored vs required dimension under a policy (DRY entry point).
    pub async fn reconcile_dimension(
        &self,
        required_dimension: usize,
        policy: DimensionReconcilePolicy,
    ) -> Result<DimensionEnsureOutcome> {
        self.pool.initialize().await?;

        let stored_dim = self.get_stored_dimension().await?;
        let table_empty = match stored_dim {
            Some(dim) if dim != required_dimension => self.vector_table_is_empty().await?,
            _ => true,
        };
        let allow_rebuild = allow_vector_table_rebuild();
        let action = decide_dimension_action(
            stored_dim,
            required_dimension,
            table_empty,
            allow_rebuild,
            policy,
        );

        match action {
            DimensionAction::Match => {
                tracing::debug!(
                    table = %self.table_name,
                    dimension = required_dimension,
                    "Vector table dimension matches, no recreation needed"
                );
                Ok(DimensionEnsureOutcome::Matched)
            }
            DimensionAction::CreateLater => {
                tracing::debug!(
                    table = %self.table_name,
                    dimension = required_dimension,
                    "Vector table empty or not exists, will create on initialize"
                );
                Ok(DimensionEnsureOutcome::Matched)
            }
            DimensionAction::RecreateEmpty | DimensionAction::RecreateAllowed => {
                let stored = stored_dim.expect("recreate requires stored dim");
                tracing::warn!(
                    table = %self.table_name,
                    old_dimension = stored,
                    new_dimension = required_dimension,
                    empty = matches!(action, DimensionAction::RecreateEmpty),
                    allow_rebuild = matches!(action, DimensionAction::RecreateAllowed),
                    "Vector dimension mismatch — recreating table"
                );
                self.drop_table().await?;
                self.create_table().await?;
                tracing::info!(
                    table = %self.table_name,
                    dimension = required_dimension,
                    "Vector table recreated with new dimension"
                );
                Ok(DimensionEnsureOutcome::Recreated)
            }
            DimensionAction::KeepExisting => {
                let stored = stored_dim.expect("keep-existing requires stored dim");
                tracing::warn!(
                    table = %self.table_name,
                    stored_dimension = stored,
                    required_dimension,
                    "Vector dimension mismatch — keeping existing schema (PreferExisting / SPEC-058). \
                     Rebind default storage to stored dim; switch embedding provider to match, \
                     re-embed into a new workspace, or set EDGEQUAKE_ALLOW_VECTOR_TABLE_REBUILD=1."
                );
                Ok(DimensionEnsureOutcome::KeptExisting {
                    stored,
                    required: required_dimension,
                })
            }
            DimensionAction::FailClosed => {
                let stored = stored_dim.expect("fail-closed requires stored dim");
                crate::compensation::record_vector_dim_mismatch_rejected();
                Err(StorageError::InvalidQuery(format!(
                    "Vector dimension mismatch on {}: stored={stored}, required={required_dimension}. \
                     Refusing DROP TABLE (SPEC-058). Re-embed into a new workspace, or set \
                     EDGEQUAKE_ALLOW_VECTOR_TABLE_REBUILD=1 to wipe and recreate.",
                    self.table_name
                )))
            }
        }
    }

    /// Workspace / write path: fail-closed on non-empty mismatch (SPEC-058).
    ///
    /// Empty schema-only mismatch recreates without the allow flag.
    /// Returns `true` when the table was recreated.
    pub async fn ensure_dimension(&self, required_dimension: usize) -> Result<bool> {
        match self
            .reconcile_dimension(required_dimension, DimensionReconcilePolicy::FailClosed)
            .await?
        {
            DimensionEnsureOutcome::Recreated => Ok(true),
            DimensionEnsureOutcome::Matched => Ok(false),
            DimensionEnsureOutcome::KeptExisting { stored, required } => {
                Err(StorageError::InvalidConfig(format!(
                    "internal invariant: FailClosed returned KeptExisting \
                     (stored={stored}, required={required})"
                )))
            }
        }
    }
}

/// Opt-in destructive recreate on embedding dimension mismatch (SPEC-058).
pub fn allow_vector_table_rebuild() -> bool {
    matches!(
        std::env::var("EDGEQUAKE_ALLOW_VECTOR_TABLE_REBUILD")
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "yes" | "on"
    )
}
