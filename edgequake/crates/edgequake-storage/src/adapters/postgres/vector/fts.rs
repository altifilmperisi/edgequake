//! PostgreSQL native FTS for chunk sparse retrieval (SPEC-023 I10).
//!
//! Uses GIN-indexed `content_tsv` + `ts_rank_cd` (cover-density ranking — **not**
//! Okapi BM25; see SPEC-083 X-05) instead of re-scoring vector candidates in
//! application memory.
//!
//! SPEC-105 / LAW-L3: KV LEFT JOIN is **era-aware** — only when
//! `chunk_kv_table_exists` (≤0.22 mid-upgrade). Post-125 census, content comes
//! from `content_tsv` / metadata only (typed chunks SSOT via serving path).
//!
//! #405 / SPEC-091: under typed vector backend, FTS reads `public.chunks.content_tsv`
//! (migration 136) — never retired `eq_*_vectors`. Legacy path keeps
//! `EDGEQUAKE_FTS_LANGUAGE` for rollback only.

use sqlx::Row;
use uuid::Uuid;

use super::typed_read::resolve_workspace_uuid;
use super::PgVectorStorage;
use crate::adapters::postgres::schema;
use crate::error::{Result, StorageError};
use crate::traits::{MetadataFilter, VectorSearchResult};

/// Env key for Postgres text-search configuration name (X-05).
pub const FTS_LANGUAGE_ENV: &str = "EDGEQUAKE_FTS_LANGUAGE";

/// Default Postgres text-search config when env is unset.
pub const DEFAULT_FTS_LANGUAGE: &str = "english";

/// Sanitize a Postgres `regconfig` name: lowercase ASCII letters only.
///
/// Rejects anything that could break out of a SQL string literal.
pub fn sanitize_fts_language(raw: &str) -> String {
    let lower = raw.trim().to_ascii_lowercase();
    if lower.is_empty()
        || !lower.chars().all(|c| c.is_ascii_lowercase() || c == '_')
        || lower.len() > 32
    {
        return DEFAULT_FTS_LANGUAGE.to_string();
    }
    lower
}

/// Resolve FTS language from `EDGEQUAKE_FTS_LANGUAGE` (default `english`).
pub fn fts_language_from_env() -> String {
    match std::env::var(FTS_LANGUAGE_ENV) {
        Ok(v) => sanitize_fts_language(&v),
        Err(_) => DEFAULT_FTS_LANGUAGE.to_string(),
    }
}

fn fts_content_expr(join_kv: bool, lang: &str) -> String {
    if join_kv {
        format!(
            "coalesce(NULLIF(v.content_tsv, ''::tsvector), to_tsvector('{lang}', coalesce(v.metadata->>'content', k.value->>'content', '')))"
        )
    } else {
        format!(
            "coalesce(NULLIF(v.content_tsv, ''::tsvector), to_tsvector('{lang}', coalesce(v.metadata->>'content', '')))"
        )
    }
}

/// SQL for typed chunk FTS (migration 136 `chunks.content_tsv`). Exposed for
/// contract tests — must not reference legacy `eq_*_vectors`.
pub(crate) const TYPED_CHUNKS_FTS_SQL: &str = r#"
            SELECT coalesce(c.metadata->>'legacy_chunk_key', c.id::text) AS id,
                   c.metadata,
                   ts_rank_cd(
                       c.content_tsv,
                       websearch_to_tsquery('english', $1)
                   )::float4 AS score
            FROM public.chunks c
            JOIN public.documents d ON d.id = c.document_id
            WHERE c.content_tsv @@ websearch_to_tsquery('english', $1)
              AND ($2::uuid IS NULL
                   OR d.workspace_id = $2
                   OR (d.workspace_id IS NULL AND d.metadata->>'workspace_id' = $3))
              AND ($4::uuid[] IS NULL OR c.document_id = ANY($4))
              AND ($5::uuid IS NULL OR c.tenant_id = $5)
              AND ($6::text[] IS NULL OR c.metadata->>'modality' = ANY($6))
              AND (
                    $7::text[] IS NULL
                    OR c.id::text = ANY($7)
                    OR c.metadata->>'legacy_chunk_key' = ANY($7)
                  )
            ORDER BY score DESC
            LIMIT $8
            "#;

impl PgVectorStorage {
    pub(crate) async fn chunk_kv_table_exists_cached(&self) -> Result<bool> {
        if let Some(exists) = self.chunk_kv_table_exists.get() {
            return Ok(*exists);
        }

        let pool = self.pool.get().await?;
        let exists = schema::relation_exists(&pool, &self.chunk_kv_table_name).await?;
        let _ = self.chunk_kv_table_exists.set(exists);
        Ok(exists)
    }

    /// Typed-authority FTS over `public.chunks.content_tsv` (#405 / SPEC-091).
    ///
    /// Regconfig is fixed to `english` (STORED generated column in migration 136).
    /// Returns legacy-shaped ids via `metadata.legacy_chunk_key`.
    pub(crate) async fn typed_chunks_text_search_filtered(
        &self,
        query_text: &str,
        top_k: usize,
        filter_ids: Option<&[String]>,
        metadata_filter: Option<&MetadataFilter>,
    ) -> Result<Vec<VectorSearchResult>> {
        if query_text.trim().is_empty() || top_k == 0 {
            return Ok(Vec::new());
        }

        let mf = metadata_filter.cloned().unwrap_or_default();
        if let Some(vtype) = mf.vector_type.as_deref() {
            if !vtype.eq_ignore_ascii_case("chunk") {
                // Non-chunk vector_type short-circuit: no entity/rel tsvector.
                return Ok(Vec::new());
            }
        }

        let pool = self.pool.get().await?;

        let workspace_uuid = if let Some(wid) = mf.workspace_id.as_deref() {
            match resolve_workspace_uuid(&pool, wid).await? {
                Some(u) => Some(u),
                None => return Ok(Vec::new()),
            }
        } else {
            None
        };
        let workspace_text = workspace_uuid.map(|u| u.to_string());

        let document_ids = match &mf.document_ids {
            Some(ids) if !ids.is_empty() => {
                let parsed: Vec<Uuid> = ids
                    .iter()
                    .filter_map(|id| Uuid::parse_str(id).ok())
                    .collect();
                if parsed.is_empty() {
                    return Ok(Vec::new());
                }
                Some(parsed)
            }
            _ => None,
        };

        let tenant_id = match mf.tenant_id.as_deref() {
            Some(tid) => match Uuid::parse_str(tid) {
                Ok(u) => Some(u),
                Err(_) => return Ok(Vec::new()),
            },
            None => None,
        };

        let modalities = mf.modalities.clone().filter(|m| !m.is_empty());
        let id_filter = filter_ids
            .filter(|ids| !ids.is_empty())
            .map(|ids| ids.to_vec());

        let rows = sqlx::query(TYPED_CHUNKS_FTS_SQL)
            .bind(query_text)
            .bind(workspace_uuid)
            .bind(workspace_text)
            .bind(document_ids.as_deref())
            .bind(tenant_id)
            .bind(modalities.as_deref())
            .bind(id_filter.as_deref())
            .bind(top_k as i32)
            .fetch_all(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("Typed chunk FTS query failed: {e}")))?;

        Ok(rows
            .iter()
            .map(|row| VectorSearchResult {
                id: row.get("id"),
                score: row.get::<f32, _>("score"),
                metadata: row.get("metadata"),
            })
            .collect())
    }

    /// Full-text search with `ts_rank_cd` over legacy vector chunk content.
    ///
    /// Rollback path when `EDGEQUAKE_VECTOR_BACKEND=legacy_tables`.
    pub(crate) async fn postgres_text_search_filtered(
        &self,
        query_text: &str,
        top_k: usize,
        filter_ids: Option<&[String]>,
        metadata_filter: Option<&MetadataFilter>,
    ) -> Result<Vec<VectorSearchResult>> {
        if query_text.trim().is_empty() || top_k == 0 {
            return Ok(Vec::new());
        }

        let lang = fts_language_from_env();
        let pool = self.pool.get().await?;
        let join_kv = self.chunk_kv_table_exists_cached().await?;
        let content_expr = fts_content_expr(join_kv, &lang);
        let mf = metadata_filter.cloned().unwrap_or_default();
        let has_id_filter = filter_ids.map(|ids| !ids.is_empty()).unwrap_or(false);
        let filter_sql = mf.build_sql_with_alias(has_id_filter, 2, Some("v"));

        let mut conditions = vec![format!(
            "{content_expr} @@ websearch_to_tsquery('{lang}', $1)"
        )];
        conditions.extend(filter_sql.conditions);

        let where_clause = format!("WHERE {}", conditions.join(" AND "));

        let kv_join = if join_kv {
            format!(
                "LEFT JOIN {} k ON k.key = coalesce(v.metadata->>'content_ref', v.id)",
                self.chunk_kv_table_name
            )
        } else {
            String::new()
        };

        let sql = format!(
            r#"
            SELECT v.id, v.metadata,
                   ts_rank_cd(
                       {content_expr},
                       websearch_to_tsquery('{lang}', $1)
                   )::float4 AS score
            FROM {vectors} v
            {kv_join}
            {where_clause}
            ORDER BY score DESC
            LIMIT ${limit_param}
            "#,
            content_expr = content_expr,
            lang = lang,
            vectors = self.table_name,
            kv_join = kv_join,
            where_clause = where_clause,
            limit_param = filter_sql.next_param
        );

        use sqlx::postgres::PgArguments;
        use sqlx::Arguments;

        let mut args = PgArguments::default();
        args.add(query_text)
            .map_err(|e| StorageError::Database(format!("Failed to bind FTS query text: {}", e)))?;

        if let Some(ids) = filter_ids {
            if !ids.is_empty() {
                let id_vec: Vec<String> = ids.to_vec();
                args.add(&id_vec).map_err(|e| {
                    StorageError::Database(format!("Failed to bind filter_ids: {}", e))
                })?;
            }
        }

        if let Some(doc_ids) = &mf.document_ids {
            args.add(&doc_ids.clone()).map_err(|e| {
                StorageError::Database(format!("Failed to bind document_ids: {}", e))
            })?;
        }
        if let Some(tid) = &mf.tenant_id {
            args.add(tid)
                .map_err(|e| StorageError::Database(format!("Failed to bind tenant_id: {}", e)))?;
        }
        if let Some(wid) = &mf.workspace_id {
            args.add(wid).map_err(|e| {
                StorageError::Database(format!("Failed to bind workspace_id: {}", e))
            })?;
        }
        if let Some(vtype) = &mf.vector_type {
            args.add(vtype).map_err(|e| {
                StorageError::Database(format!("Failed to bind vector_type: {}", e))
            })?;
        }
        if let Some(modalities) = &mf.modalities {
            let mods: Vec<String> = modalities.clone();
            args.add(&mods)
                .map_err(|e| StorageError::Database(format!("Failed to bind modalities: {}", e)))?;
        }

        args.add(top_k as i32)
            .map_err(|e| StorageError::Database(format!("Failed to bind top_k: {}", e)))?;

        let rows = sqlx::query_with(&sql, args)
            .fetch_all(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("Postgres FTS query failed: {}", e)))?;

        Ok(rows
            .iter()
            .map(|row| VectorSearchResult {
                id: row.get("id"),
                score: row.get::<f32, _>("score"),
                metadata: row.get("metadata"),
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_typed_chunks_fts_sql_shape() {
        let sql = TYPED_CHUNKS_FTS_SQL;
        assert!(sql.contains("c.content_tsv"));
        assert!(sql.contains("legacy_chunk_key"));
        assert!(sql.contains("websearch_to_tsquery('english'"));
        assert!(sql.contains("ts_rank_cd"));
        assert!(
            !sql.contains("eq_") && !sql.contains("self.table_name"),
            "typed FTS must not reference legacy vectors tables"
        );
        assert!(
            !sql.contains("metadata->>'type'"),
            "typed FTS must not filter metadata type=chunk (relational rows omit it)"
        );
    }

    #[test]
    fn e2e_fts_language_config() {
        assert_eq!(sanitize_fts_language("french"), "french");
        assert_eq!(sanitize_fts_language("SIMPLE"), "simple");
        assert_eq!(
            sanitize_fts_language("english'; drop table x"),
            DEFAULT_FTS_LANGUAGE
        );
        assert_eq!(sanitize_fts_language(""), DEFAULT_FTS_LANGUAGE);
        assert_eq!(sanitize_fts_language("fr-FR"), DEFAULT_FTS_LANGUAGE);

        std::env::remove_var(FTS_LANGUAGE_ENV);
        assert_eq!(fts_language_from_env(), DEFAULT_FTS_LANGUAGE);

        std::env::set_var(FTS_LANGUAGE_ENV, "french");
        assert_eq!(fts_language_from_env(), "french");
        let expr = fts_content_expr(false, &fts_language_from_env());
        assert!(
            expr.contains("to_tsvector('french'"),
            "FTS content expr must use configured language: {expr}"
        );
        assert!(
            !expr.contains("BM25"),
            "X-05: postgres FTS path must not claim BM25"
        );
        std::env::remove_var(FTS_LANGUAGE_ENV);
    }
}
