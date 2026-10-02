//! PostgreSQL key-value storage using JSONB.
//!
//! Provides flexible key-value storage with full JSON query capabilities.
//!
//! ## Implements
//!
//! - [`FEAT0240`]: JSONB key-value storage
//! - [`FEAT0241`]: GIN indexing for fast JSON path queries
//! - [`FEAT0242`]: Atomic upsert operations
//!
//! ## Use Cases
//!
//! - [`UC0601`]: System stores document metadata
//! - [`UC0605`]: System retrieves chunks by ID
//!
//! ## Enforces
//!
//! - [`BR0240`]: Namespace isolation per tenant
//! - [`BR0241`]: Atomic batch operations

use std::collections::HashSet;
use std::sync::Arc;

use async_trait::async_trait;

use super::config::{qualified_kv_table_name, PostgresConfig};
use super::connection::PostgresPool;
use super::kv_relation_state::{KvRelationPresence, KvRelationState};
use crate::error::{Result, StorageError};
use crate::kv_keys;
use crate::traits::KVStorage;

/// SPEC-083 X-37: validate workspace-scoped KV keys in a write batch.
///
/// - Malformed `wsdoc:` / `staging:hash:` keys are rejected.
/// - A single upsert batch may not mix multiple embedded workspace ids
///   (defense-in-depth against accidental cross-tenant writes).
fn enforce_workspace_scoped_keys<'a>(keys: impl Iterator<Item = &'a str>) -> Result<()> {
    let mut seen_workspace: Option<&str> = None;
    for key in keys {
        if key.starts_with("wsdoc:") || key.starts_with("staging:hash:") {
            let Some(ws) = kv_keys::embedded_workspace_id(key) else {
                return Err(StorageError::InvalidInput(format!(
                    "Malformed workspace-scoped KV key: {key}"
                )));
            };
            match seen_workspace {
                None => seen_workspace = Some(ws),
                Some(prev) if prev != ws => {
                    return Err(StorageError::InvalidInput(format!(
                        "KV upsert mixes workspace scopes '{prev}' and '{ws}'"
                    )));
                }
                Some(_) => {}
            }
        }
    }
    Ok(())
}

/// PostgreSQL key-value storage using JSONB.
///
/// This implementation uses PostgreSQL's JSONB column type for flexible
/// value storage with full JSON query capabilities.
///
/// # Features
///
/// - JSONB storage for flexible schemas
/// - GIN indexing for fast JSON path queries
/// - Atomic upsert operations
/// - Namespace support for multi-tenancy
pub struct PostgresKVStorage {
    pool: PostgresPool,
    table_name: String,
    stats_table_name: String,
    namespace: String,
    /// SPEC-091 Doc 23: Absent → zero SQL to missing `eq_*_kv`.
    relation_state: Arc<KvRelationState>,
}

/// Key classification for the KV family router (SSOT — one matcher chain
/// shared by mode resolution AND the unclassified-key hazard gate; DRY).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KvKeyClass {
    /// `{doc}-chunk-{N}` — governed by the chunk-text authority flag.
    Chunk,
    /// A named cutover family (env-governed mode).
    Family(&'static str),
    /// No known family matched. Pre-drop these keys keep writing KV
    /// (fail-safe: the guarded drop refuses to run while KV rows remain).
    /// Post-drop they have NO typed home and must fail loudly rather than
    /// silently vanish (GAP-091-07).
    Unclassified,
}

impl PostgresKVStorage {
    /// Create a new PostgreSQL key-value storage.
    pub fn new(config: PostgresConfig) -> Self {
        Self::with_pool(PostgresPool::new(config.clone()), config)
    }

    /// Create KV storage using a shared connection pool (SPEC-011).
    pub fn with_pool(pool: PostgresPool, config: PostgresConfig) -> Self {
        let prefix = config.table_prefix();
        let table_name = qualified_kv_table_name(&prefix);
        let stats_table_name = format!("public.eq_{}_kv_stats", prefix);
        let namespace = config.namespace.clone();

        Self {
            pool,
            table_name,
            stats_table_name,
            namespace,
            relation_state: Arc::new(KvRelationState::new()),
        }
    }

    /// SPEC-091 Doc 23: seed relation posture from boot cutover census.
    pub fn seed_relation_from_dropped(&self, kv_store_dropped: bool) {
        self.relation_state.seed_from_dropped(kv_store_dropped);
    }

    /// Raw SQL attempts against the KV base/stats table (tests / soak).
    pub fn kv_raw_sql_attempts(&self) -> u64 {
        self.relation_state.sql_attempts()
    }

    pub fn reset_kv_raw_sql_attempts(&self) {
        self.relation_state.reset_sql_attempts();
    }

    /// True when the KV relation is known Absent (cached; no I/O).
    pub fn kv_relation_absent_cached(&self) -> bool {
        self.relation_state.cached() == Some(KvRelationPresence::Absent)
    }

    async fn kv_relation_is_absent(&self, pool: &sqlx::PgPool) -> Result<bool> {
        Ok(self
            .relation_state
            .get_or_probe(pool, &self.table_name)
            .await?
            == KvRelationPresence::Absent)
    }

    fn note_kv_undefined(&self, e: &sqlx::Error) {
        if Self::is_undefined_table(e) {
            self.relation_state.note_undefined_table();
        }
    }

    /// Get the underlying pool.
    pub fn pool(&self) -> &PostgresPool {
        &self.pool
    }

    /// SPEC-091 Wave C: whether the metadata/content/staging shell families
    /// read typed-first (`EDGEQUAKE_KV_FAMILY_METADATA=relational`).
    fn shell_family_reads_relational() -> bool {
        crate::kv_family_cutover::kv_family_mode_from_env(
            crate::kv_family_cutover::KV_FAMILY_METADATA,
        ) == crate::kv_family_cutover::KvFamilyMode::Relational
    }

    /// SPEC-091 Wave D: a dropped KV relation is empty by definition — the
    /// guarded drop migration only runs after every family is drained, so a
    /// missing table means "no rows", not "error". Map PostgreSQL 42P01
    /// (undefined_table) so legacy KV fallbacks degrade to empty instead of
    /// 500s after the drop wave lands.
    fn is_undefined_table(e: &sqlx::Error) -> bool {
        matches!(e, sqlx::Error::Database(db) if db.code().as_deref() == Some("42P01"))
    }

    /// SPEC-091 Wave D write-stop: whether this key's family still writes KV.
    /// Families whose flag flipped to relational write typed-only — the typed
    /// writer (shell/cache upsert here, upstream persister for chunks,
    /// API-side stores for dedup/sidecars) is the single authority.
    fn key_family_writes_kv(key: &str) -> bool {
        Self::family_mode_for_key(key) == crate::kv_family_cutover::KvFamilyMode::Kv
    }

    /// Single matcher chain for every family (see [`KvKeyClass`]).
    fn classify_key(key: &str) -> KvKeyClass {
        use crate::kv_family_cutover::{
            KV_FAMILY_ARTIFACT, KV_FAMILY_CACHE, KV_FAMILY_CHECKPOINT,
            KV_FAMILY_COMPENSATION_QUARANTINE, KV_FAMILY_DOC_HASH, KV_FAMILY_INJECTION,
            KV_FAMILY_METADATA, KV_FAMILY_WSDOC,
        };
        if kv_keys::parse_doc_chunk(key).is_some() {
            return KvKeyClass::Chunk;
        }
        if key.starts_with("doc:hash:") || key.starts_with("staging:hash:") {
            return KvKeyClass::Family(KV_FAMILY_DOC_HASH);
        }
        if key.starts_with("wsdoc:") {
            return KvKeyClass::Family(KV_FAMILY_WSDOC);
        }
        if key.starts_with("compensation_quarantine:") {
            return KvKeyClass::Family(KV_FAMILY_COMPENSATION_QUARANTINE);
        }
        if key.starts_with("injection::") {
            return KvKeyClass::Family(KV_FAMILY_INJECTION);
        }
        if key.ends_with("-pipeline-checkpoint")
            || key.ends_with("-extraction-snapshot")
            // SPEC-156: mid-extract partial chunk checkpoint (same family as
            // pipeline-checkpoint / extraction-snapshot).
            || key.ends_with("-chunk-extract-partial")
        {
            return KvKeyClass::Family(KV_FAMILY_CHECKPOINT);
        }
        if key.ends_with("-lineage")
            || key.ends_with("-multimodal-manifest")
            || key.ends_with("-multimodal-chunks")
        {
            return KvKeyClass::Family(KV_FAMILY_ARTIFACT);
        }
        if super::llm_cache::is_cache_key(key) {
            return KvKeyClass::Family(KV_FAMILY_CACHE);
        }
        // Shell families (metadata / content / staging shells) — checked last
        // because their suffixes (`-metadata`, `-content`) overlap with the
        // more specific families above (e.g. `injection::…-metadata`).
        if super::document_shell::parse_shell_key(key).is_some() {
            return KvKeyClass::Family(KV_FAMILY_METADATA);
        }
        KvKeyClass::Unclassified
    }

    /// Resolve the governing cutover mode for a key (SSOT — one classifier
    /// shared by read dispatch, write-stop and DDL gating; DRY).
    fn family_mode_for_key(key: &str) -> crate::kv_family_cutover::KvFamilyMode {
        use crate::kv_family_cutover::{kv_family_mode_from_env, KvFamilyMode};
        match Self::classify_key(key) {
            KvKeyClass::Chunk => {
                if crate::chunk_text_authority::chunk_text_authority_writes_kv(
                    crate::chunk_text_authority::chunk_text_authority_from_env(),
                ) {
                    KvFamilyMode::Kv
                } else {
                    KvFamilyMode::Relational
                }
            }
            KvKeyClass::Family(family) => kv_family_mode_from_env(family),
            // Unknown families keep writing KV until classified (fail-safe
            // pre-drop; post-drop the upsert path errors loudly — GAP-091-07).
            KvKeyClass::Unclassified => KvFamilyMode::Kv,
        }
    }

    /// Single-key KV fetch (extracted for SPEC-091 authority switching).
    async fn kv_value_by_id(
        &self,
        pool: &sqlx::PgPool,
        id: &str,
    ) -> Result<Option<serde_json::Value>> {
        if self.kv_relation_is_absent(pool).await? {
            return Ok(None);
        }
        self.relation_state.record_sql_attempt();
        let sql = crate::dataop::sql_comment(
            crate::dataop::DATA_PG_KV_GET_BY_ID_075,
            &format!("SELECT value FROM {} WHERE key = $1", self.table_name),
        );

        let row: Option<(serde_json::Value,)> =
            match sqlx::query_as(&sql).bind(id).fetch_optional(pool).await {
                Ok(row) => row,
                Err(e) if Self::is_undefined_table(&e) => {
                    self.note_kv_undefined(&e);
                    None
                }
                Err(e) => return Err(StorageError::Database(format!("KV get failed: {}", e))),
            };

        Ok(row.map(|(v,)| v))
    }

    /// Ordered KV fetch (extracted for SPEC-091 authority switching).
    async fn kv_values_ordered(
        &self,
        pool: &sqlx::PgPool,
        ids: &[String],
    ) -> Result<Vec<Option<serde_json::Value>>> {
        if self.kv_relation_is_absent(pool).await? {
            return Ok(ids.iter().map(|_| None).collect());
        }
        self.relation_state.record_sql_attempt();
        let sql = format!(
            "SELECT kv.value \
             FROM unnest($1::text[]) WITH ORDINALITY AS u(key, ord) \
             LEFT JOIN {table} kv ON kv.key = u.key \
             ORDER BY u.ord",
            table = self.table_name
        );

        let rows: Vec<(Option<serde_json::Value>,)> =
            match sqlx::query_as(&sql).bind(ids).fetch_all(pool).await {
                Ok(rows) => rows,
                Err(e) if Self::is_undefined_table(&e) => {
                    self.note_kv_undefined(&e);
                    return Ok(ids.iter().map(|_| None).collect());
                }
                Err(e) => {
                    return Err(StorageError::Database(format!(
                        "KV get_by_ids_ordered failed: {}",
                        e
                    )))
                }
            };

        Ok(rows.into_iter().map(|(v,)| v).collect())
    }
}

#[async_trait]
impl KVStorage for PostgresKVStorage {
    fn namespace(&self) -> &str {
        &self.namespace
    }

    async fn initialize(&self) -> Result<()> {
        self.pool.initialize().await?;
        // SPEC-091 Wave D (complete): the generic KV relation is never created
        // at runtime. Every family defaults to relational authority; this
        // adapter is a typed-routing facade whose legacy KV reads tolerate
        // 42P01 (dropped relation == empty). Schema changes ship only via
        // sqlx migrations (Code is Law).
        // SPEC-091 Doc 23: seed Absent/Present once at boot (LAW-KVH2).
        if self.relation_state.cached().is_none() {
            let pool = self.pool.get().await?;
            let _ = self
                .relation_state
                .get_or_probe(&pool, &self.table_name)
                .await?;
        }
        tracing::debug!(
            table = %self.table_name,
            presence = ?self.relation_state.cached(),
            "SPEC-091: KV runtime DDL removed — adapter runs in typed-routing mode"
        );
        Ok(())
    }

    async fn finalize(&self) -> Result<()> {
        Ok(())
    }

    /**
     * @dataop      DATA-PG-KV-GET-BY-ID-075
     * @engine      postgres
     * @intent      Single-key JSONB lookup by primary key.
     * @tables      eq_{ns}_kv(key, value jsonb)
     * @indexes     PRIMARY KEY (key)
     * @complexity  time: O(log N); space: O(1); io: 1 index + heap
     * @limits      - Prefer get_by_ids for batches (avoid N+1)
     * @scaling     Log N to full table
     * @tests       tests/data_layer/data_layer_limits.rs
     * @pgversions  16: ok | 17: ok | 18: ok
     * @docs        specs/088-data-layer/postgres.md#data-pg-kv-get-by-id-075
     */
    async fn get_by_id(&self, id: &str) -> Result<Option<serde_json::Value>> {
        let _timer = crate::TimedStorageOp::start_dataop(crate::dataop::DATA_PG_KV_GET_BY_ID_075);
        let pool = self.pool.get().await?;

        // SPEC-091 W1 cutover: chunk keys dispatch on the authority flag
        // (single dispatch SSOT — all single-key readers inherit the cutover).
        let authority = crate::chunk_text_authority::chunk_text_authority_from_env();
        let is_chunk_key = kv_keys::parse_doc_chunk(id).is_some();
        match authority {
            crate::chunk_text_authority::ChunkTextAuthority::Relational if is_chunk_key => {
                return crate::chunk_text_dual_read::relational_value_by_key(&pool, id).await;
            }
            crate::chunk_text_authority::ChunkTextAuthority::Dual if is_chunk_key => {
                let value = self.kv_value_by_id(&pool, id).await?;
                crate::chunk_text_dual_read::shadow_compare(
                    &pool,
                    &[id.to_string()],
                    std::slice::from_ref(&value),
                )
                .await;
                return Ok(value);
            }
            _ => {}
        }

        // SPEC-091 Wave C cutover: metadata/content/staging shell keys read
        // typed-first (`EDGEQUAKE_KV_FAMILY_METADATA=relational`), KV fallback
        // on any gap (no row / empty shell during dual-write transition).
        if Self::shell_family_reads_relational()
            && super::document_shell::parse_shell_key(id).is_some()
        {
            if let Some(value) = super::document_shell::shell_value_by_key(&pool, id).await? {
                return Ok(Some(value));
            }
        }

        // SPEC-091 Wave D cutover: cache keys read typed-first
        // (`EDGEQUAKE_KV_FAMILY_CACHE=relational`), KV fallback for entries
        // written before the cutover (miss costs one recomputation only).
        if super::llm_cache::is_cache_key(id) && !Self::key_family_writes_kv(id) {
            if let Some(value) = super::llm_cache::cache_get(&pool, &self.namespace, id).await? {
                return Ok(Some(value));
            }
        }

        self.kv_value_by_id(&pool, id).await
    }

    /**
     * @dataop      DATA-PG-KV-GET-BY-IDS-076
     * @engine      postgres
     * @intent      Ordered multi-key fetch via UNNEST + PK join (one RT).
     * @tables      eq_{ns}_kv
     * @indexes     PRIMARY KEY (key)
     * @complexity  time: O(K log N); space: O(K)
     * @limits      - K bounded by app; Postgres bind param ceiling 65535
     * @scaling     Linear in K
     * @tests       tests/data_layer/data_layer_limits.rs
     * @pgversions  16: ok | 17: ok | 18: ok
     * @docs        specs/088-data-layer/postgres.md#data-pg-kv-get-by-ids-076
     */
    async fn get_by_ids(&self, ids: &[String]) -> Result<Vec<serde_json::Value>> {
        let _timer = crate::TimedStorageOp::start_dataop(crate::dataop::DATA_PG_KV_GET_BY_IDS_076);
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        // SPEC-091 IW0 (GAP-091-04, DRY): route through the same typed-first
        // merge pipeline as `get_by_ids_ordered` (cache → shell → chunk → KV
        // fallback). The legacy KV-only INNER JOIN read 404'd document
        // downloads on post-125 databases where shell/cache/chunk homes are
        // typed tables and `eq_*_kv` is dropped. The `flatten` preserves the
        // historical "present values in input order" compaction contract.
        Ok(self
            .get_by_ids_ordered(ids)
            .await?
            .into_iter()
            .flatten()
            .collect())
    }

    async fn get_by_ids_ordered(&self, ids: &[String]) -> Result<Vec<Option<serde_json::Value>>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let authority = crate::chunk_text_authority::chunk_text_authority_from_env();
        let pool = self.pool.get().await?;

        // SPEC-091 Wave C/D unified typed-first merge pipeline (DRY — every
        // family reader shares the "None = not mine / miss" contract):
        //   1. cache keys   → public.llm_cache        (family flag relational)
        //   2. shell keys   → public.documents        (family flag relational)
        //   3. chunk keys   → public.chunks           (authority relational)
        //   4. everything left → legacy KV fallback (covers pre-cutover rows)
        //   5. dual-mode chunk shadow compare on the merged result
        let mut out: Vec<Option<serde_json::Value>> = vec![None; ids.len()];

        // 1. Cache family.
        let cache_idx: Vec<usize> = ids
            .iter()
            .enumerate()
            .filter(|(_, id)| super::llm_cache::is_cache_key(id) && !Self::key_family_writes_kv(id))
            .map(|(i, _)| i)
            .collect();
        if !cache_idx.is_empty() {
            let cache_ids: Vec<String> = cache_idx.iter().map(|&i| ids[i].clone()).collect();
            let cache_rows =
                super::llm_cache::cache_values_ordered(&pool, &self.namespace, &cache_ids).await?;
            for (pos, value) in cache_idx.into_iter().zip(cache_rows) {
                out[pos] = value;
            }
        }

        // 2. Shell families (metadata/content/staging).
        if Self::shell_family_reads_relational() {
            let shell_rows = super::document_shell::shell_values_ordered(&pool, ids).await?;
            for (pos, value) in shell_rows.into_iter().enumerate() {
                if out[pos].is_none() {
                    out[pos] = value;
                }
            }
        }

        // 3. Chunk family (relational authority only).
        if matches!(
            authority,
            crate::chunk_text_authority::ChunkTextAuthority::Relational
        ) {
            let chunk_rows =
                crate::chunk_text_dual_read::relational_values_ordered(&pool, ids).await?;
            for (pos, value) in chunk_rows.into_iter().enumerate() {
                if out[pos].is_none() {
                    out[pos] = value;
                }
            }
        }

        // 4. KV fallback for anything still unresolved (also the entire read
        //    path while every family flag is `kv`).
        let kv_idx: Vec<usize> = out
            .iter()
            .enumerate()
            .filter(|(_, v)| v.is_none())
            .map(|(i, _)| i)
            .collect();
        if !kv_idx.is_empty() {
            let kv_ids: Vec<String> = kv_idx.iter().map(|&i| ids[i].clone()).collect();
            let kv_rows = self.kv_values_ordered(&pool, &kv_ids).await?;
            for (pos, value) in kv_idx.into_iter().zip(kv_rows) {
                out[pos] = value;
            }
        }

        // 5. Dual-mode chunk shadow compare (KV still authoritative for chunks).
        if matches!(
            authority,
            crate::chunk_text_authority::ChunkTextAuthority::Dual
        ) {
            crate::chunk_text_dual_read::shadow_compare(&pool, ids, &out).await;
        }

        Ok(out)
    }

    async fn filter_keys(&self, keys: HashSet<String>) -> Result<HashSet<String>> {
        if keys.is_empty() {
            return Ok(HashSet::new());
        }

        if self.kv_relation_absent_cached() {
            return Ok(keys);
        }

        let pool = self.pool.get().await?;
        if self.kv_relation_is_absent(&pool).await? {
            return Ok(keys);
        }
        let keys_vec: Vec<String> = keys.iter().cloned().collect();

        self.relation_state.record_sql_attempt();
        let sql = format!("SELECT key FROM {} WHERE key = ANY($1)", self.table_name);

        let rows: Vec<(String,)> = match sqlx::query_as(&sql).bind(&keys_vec).fetch_all(&pool).await
        {
            Ok(rows) => rows,
            // SPEC-091 (EC-30): a dropped generic-KV relation means nothing is persisted
            // there, so every candidate key is "missing" — not an error.
            Err(e) if Self::is_undefined_table(&e) => {
                self.note_kv_undefined(&e);
                return Ok(keys);
            }
            Err(e) => {
                return Err(StorageError::Database(format!(
                    "KV filter_keys failed: {}",
                    e
                )))
            }
        };

        let existing: HashSet<String> = rows.into_iter().map(|(k,)| k).collect();

        // Return keys that do NOT exist
        Ok(keys.difference(&existing).cloned().collect())
    }

    /**
     * @dataop      DATA-PG-KV-UPSERT-079
     * @engine      postgres
     * @intent      Atomic batch upsert of JSONB values (UNNEST + ON CONFLICT).
     * @tables      eq_{ns}_kv
     * @indexes     PRIMARY KEY (key)
     * @complexity  time: O(B log N) per chunk B<=1000; space: O(B)
     * @limits      - Chunk size 1000; multi-workspace keys rejected (fail-closed)
     *              - Single transaction for full batch
     * @scaling     Linear in total keys
     * @tests       tests/data_layer/data_layer_limits.rs
     * @pgversions  16: ok | 17: ok | 18: ok
     * @docs        specs/088-data-layer/postgres.md#data-pg-kv-upsert-079
     */
    async fn upsert(&self, data: &[(String, serde_json::Value)]) -> Result<()> {
        let _timer = crate::TimedStorageOp::start_dataop(crate::dataop::DATA_PG_KV_UPSERT_079);
        if data.is_empty() {
            return Ok(());
        }

        // SPEC-083 X-37: reject malformed workspace-scoped keys; when a batch
        // mixes multiple embedded workspace ids, fail closed (cross-tenant write).
        enforce_workspace_scoped_keys(data.iter().map(|(k, _)| k.as_str()))?;

        // SPEC-091 Wave D write-stop: partition out keys whose family flipped
        // to relational — those write typed-only (shell upsert below, upstream
        // persister for chunks, API-side stores for dedup).
        let kv_data: Vec<&(String, serde_json::Value)> = data
            .iter()
            .filter(|(k, _)| Self::key_family_writes_kv(k))
            .collect();
        let metadata_relational = !data.is_empty()
            && data.iter().any(|(k, _)| {
                super::document_shell::parse_shell_key(k).is_some()
                    && !Self::key_family_writes_kv(k)
            });

        let pool = self.pool.get().await?;

        if !kv_data.is_empty() {
            // SPEC-091 Doc 23: short-circuit before begin/SQL when Absent.
            let mut kv_table_dropped = self.kv_relation_is_absent(&pool).await?;
            if !kv_table_dropped {
                // C-22: all batches commit atomically — mid-batch failure must not
                // leave a partial KV write set.
                let mut tx = pool.begin().await.map_err(|e| {
                    StorageError::Database(format!("KV upsert begin failed: {}", e))
                })?;
                const BATCH_SIZE: usize = 1000;

                // SPEC-091 Wave D (EC-30): the generic KV relation may have been
                // dropped. A stale `dual`/`kv` rollback flag pointing at the
                // dropped table must degrade to a typed-only no-op (42P01), never
                // abort the upsert — reads already tolerate the missing table.
                for chunk in kv_data.chunks(BATCH_SIZE) {
                    let keys: Vec<String> = chunk.iter().map(|(k, _)| k.clone()).collect();
                    let values: Vec<serde_json::Value> =
                        chunk.iter().map(|(_, v)| (*v).clone()).collect();

                    let sql = crate::dataop::sql_comment(
                        crate::dataop::DATA_PG_KV_UPSERT_079,
                        &format!(
                            r#"
                    INSERT INTO {} (key, value, updated_at)
                    SELECT k, v, NOW()
                    FROM unnest($1::text[], $2::jsonb[]) AS batch(k, v)
                    ON CONFLICT (key) DO UPDATE SET
                        value = EXCLUDED.value,
                        updated_at = NOW()
                    "#,
                            self.table_name
                        ),
                    );

                    self.relation_state.record_sql_attempt();
                    match sqlx::query(&sql)
                        .bind(&keys)
                        .bind(&values)
                        .execute(&mut *tx)
                        .await
                    {
                        Ok(_) => {}
                        Err(e) if Self::is_undefined_table(&e) => {
                            self.note_kv_undefined(&e);
                            kv_table_dropped = true;
                            break;
                        }
                        Err(e) => {
                            return Err(StorageError::Database(format!("KV upsert failed: {}", e)))
                        }
                    }
                }

                if kv_table_dropped {
                    // The tx is aborted (42P01); roll it back and skip the raw KV
                    // write. Typed authority (chunks/documents/llm_cache) below is
                    // unaffected, so processing still completes.
                    let _ = tx.rollback().await;
                } else {
                    tx.commit().await.map_err(|e| {
                        StorageError::Database(format!("KV upsert commit failed: {}", e))
                    })?;
                }
            } // !kv_table_dropped (short-circuit / probe Absent)

            if kv_table_dropped {
                // GAP-091-07 (SPEC-091 IW0, fail-closed): keys whose family is
                // UNCLASSIFIED have no typed home — skipping them post-drop
                // would silently discard caller data. Error loudly and name
                // every offending key. Classified families behind a stale
                // `dual`/`kv` rollback flag keep the EC-30 degrade (typed-only
                // no-op + warn).
                let unclassified: Vec<&str> = kv_data
                    .iter()
                    .map(|(k, _)| k.as_str())
                    .filter(|k| Self::classify_key(k) == KvKeyClass::Unclassified)
                    .collect();
                if !unclassified.is_empty() {
                    return Err(StorageError::Database(format!(
                        "KV relation {} was dropped (migration 125) but the batch contains                          {} unclassified key(s) with no typed home: [{}]. Classify them in                          kv.rs::classify_key (typed home) or restore the KV relation —                          refusing to silently discard writes (GAP-091-07).",
                        self.table_name,
                        unclassified.len(),
                        unclassified.join(", ")
                    )));
                }

                tracing::warn!(
                    table = %self.table_name,
                    "SPEC-091 Wave D: KV relation dropped — skipping raw KV upsert (typed authority);                      set EDGEQUAKE_CHUNK_TEXT_AUTHORITY/family flags to relational"
                );
            }
        }

        // SPEC-091 Wave C/D: typed document-shell write for the
        // metadata/content/staging families — warn-only while dual-writing,
        // authoritative (error-propagating) once the family flips relational.
        super::document_shell::dual_write_shell_upserts(&pool, data, metadata_relational).await?;

        // SPEC-091 Wave D: cache family typed write (`public.llm_cache`) when
        // `EDGEQUAKE_KV_FAMILY_CACHE=relational` — authoritative (the KV write
        // set above already excluded these keys).
        let cache_pairs: Vec<(String, serde_json::Value)> = data
            .iter()
            .filter(|(k, _)| super::llm_cache::is_cache_key(k) && !Self::key_family_writes_kv(k))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        if !cache_pairs.is_empty() {
            super::llm_cache::cache_upsert(&pool, &self.namespace, &cache_pairs).await?;
        }

        Ok(())
    }

    async fn delete(&self, ids: &[String]) -> Result<()> {
        if ids.is_empty() {
            return Ok(());
        }

        // SPEC-091 Wave D write-stop: keys whose family flipped relational are
        // deleted typed-side (FK cascades + explicit sidecar deletes); nothing
        // remains in KV to delete.
        let kv_ids: Vec<&String> = ids
            .iter()
            .filter(|k| Self::key_family_writes_kv(k))
            .collect();

        // Cache family: typed delete for keys routed to `public.llm_cache`.
        let cache_ids: Vec<String> = ids
            .iter()
            .filter(|k| super::llm_cache::is_cache_key(k) && !Self::key_family_writes_kv(k))
            .cloned()
            .collect();
        if !cache_ids.is_empty() {
            let pool = self.pool.get().await?;
            super::llm_cache::cache_delete(&pool, &self.namespace, &cache_ids).await?;
        }

        if kv_ids.is_empty() {
            return Ok(());
        }

        if self.kv_relation_absent_cached() {
            return Ok(());
        }

        let pool = self.pool.get().await?;
        if self.kv_relation_is_absent(&pool).await? {
            return Ok(());
        }

        self.relation_state.record_sql_attempt();
        let sql = format!("DELETE FROM {} WHERE key = ANY($1)", self.table_name);

        match sqlx::query(&sql).bind(&kv_ids).execute(&pool).await {
            Ok(_) => Ok(()),
            Err(e) if Self::is_undefined_table(&e) => {
                self.note_kv_undefined(&e);
                Ok(())
            }
            Err(e) => Err(StorageError::Database(format!("KV delete failed: {}", e))),
        }
    }

    async fn is_empty(&self) -> Result<bool> {
        if self.kv_relation_absent_cached() {
            return Ok(true);
        }
        let pool = self.pool.get().await?;
        if self.kv_relation_is_absent(&pool).await? {
            return Ok(true);
        }

        self.relation_state.record_sql_attempt();
        let sql = format!(
            "SELECT NOT EXISTS (SELECT 1 FROM {} LIMIT 1) AS is_empty",
            self.table_name
        );

        match sqlx::query_as::<_, (bool,)>(&sql).fetch_one(&pool).await {
            Ok(row) => Ok(row.0),
            // Dropped KV relation == empty by definition (Wave D).
            Err(e) if Self::is_undefined_table(&e) => {
                self.note_kv_undefined(&e);
                Ok(true)
            }
            Err(e) => Err(StorageError::Database(format!("KV is_empty failed: {}", e))),
        }
    }

    async fn count(&self) -> Result<usize> {
        if self.kv_relation_absent_cached() {
            return Ok(0);
        }
        let pool = self.pool.get().await?;
        if self.kv_relation_is_absent(&pool).await? {
            return Ok(0);
        }

        // O(1): read maintained counter — never `SELECT COUNT(*) FROM kv` (SPEC-011).
        self.relation_state.record_sql_attempt();
        let sql = format!(
            "SELECT row_count FROM {} WHERE id = 1",
            self.stats_table_name
        );

        let row: Option<(i64,)> = match sqlx::query_as(&sql).fetch_optional(&pool).await {
            Ok(row) => row,
            // Dropped stats relation == zero rows (Wave D).
            Err(e) if Self::is_undefined_table(&e) => {
                self.note_kv_undefined(&e);
                return Ok(0);
            }
            Err(e) => return Err(StorageError::Database(format!("KV count failed: {}", e))),
        };

        if let Some((count,)) = row {
            return Ok(count as usize);
        }

        // Stats row missing (pre-drop deployment) — exact COUNT(*) fallback.
        // Wave D removed the self-heal: stats DDL is never created at runtime.
        self.relation_state.record_sql_attempt();
        let fallback = format!("SELECT COUNT(*) as count FROM {}", self.table_name);
        match sqlx::query_as::<_, (i64,)>(&fallback)
            .fetch_one(&pool)
            .await
        {
            Ok(row) => Ok(row.0 as usize),
            Err(e) if Self::is_undefined_table(&e) => {
                self.note_kv_undefined(&e);
                Ok(0)
            }
            Err(e) => Err(StorageError::Database(format!(
                "KV count fallback failed: {}",
                e
            ))),
        }
    }

    async fn ping(&self) -> Result<()> {
        // SPEC-091 Doc 23 / LAW-KVH1: Absent → zero SQL (health must not burn pool).
        if self.kv_relation_absent_cached() {
            return Ok(());
        }
        let pool = self.pool.get().await?;
        if self.kv_relation_is_absent(&pool).await? {
            return Ok(());
        }

        self.relation_state.record_sql_attempt();
        let sql = format!("SELECT 1 FROM {} LIMIT 1", self.table_name);

        match sqlx::query(&sql).fetch_optional(&pool).await {
            Ok(_) => Ok(()),
            // Post-drop the KV adapter is a typed-routing facade — ping stays up.
            Err(e) if Self::is_undefined_table(&e) => {
                self.note_kv_undefined(&e);
                Ok(())
            }
            Err(e) => Err(StorageError::Database(format!("KV ping failed: {}", e))),
        }
    }

    /// SPEC-087 / Issue #334: O(1) round-trip chunk-key count (no payload fetch).
    async fn count_embedded_chunks_for_docs(&self, doc_ids: &[String]) -> Result<usize> {
        if doc_ids.is_empty() {
            return Ok(0);
        }

        let pool = self.pool.get().await?;
        let relational = matches!(
            crate::chunk_text_authority::chunk_text_authority_from_env(),
            crate::chunk_text_authority::ChunkTextAuthority::Relational
        ) || self.kv_relation_is_absent(&pool).await?;

        if relational {
            let mut uuids = Vec::with_capacity(doc_ids.len());
            for id in doc_ids {
                match uuid::Uuid::parse_str(id) {
                    Ok(u) => uuids.push(u),
                    Err(_) => continue,
                }
            }
            if uuids.is_empty() {
                return Ok(0);
            }
            let row: (i64,) = sqlx::query_as(
                "SELECT COUNT(*)::bigint FROM public.chunks WHERE document_id = ANY($1::uuid[])",
            )
            .bind(&uuids)
            .fetch_one(&pool)
            .await
            .map_err(|e| {
                StorageError::Database(format!(
                    "relational count_embedded_chunks_for_docs failed: {e}"
                ))
            })?;
            return Ok(row.0 as usize);
        }

        // Escape LIKE meta in each doc id so `%`/`_` in ids cannot widen the match.
        let patterns: Vec<String> = doc_ids
            .iter()
            .map(|id| format!("{}-chunk-%", escape_like_meta(id)))
            .collect();

        self.relation_state.record_sql_attempt();
        let sql = format!(
            "SELECT COUNT(*)::bigint FROM {} WHERE key LIKE ANY($1::text[])",
            self.table_name
        );

        let row: (i64,) = match sqlx::query_as(&sql).bind(&patterns).fetch_one(&pool).await {
            Ok(row) => row,
            Err(e) if Self::is_undefined_table(&e) => {
                self.note_kv_undefined(&e);
                return Ok(0);
            }
            Err(e) => {
                return Err(StorageError::Database(format!(
                    "KV count_embedded_chunks_for_docs failed: {e}"
                )))
            }
        };

        Ok(row.0 as usize)
    }

    async fn keys_like(&self, pattern: &str) -> Result<Vec<String>> {
        // SPEC-070: never unbounded fetch_all — safety LIMIT on the wire.
        const SAFETY_CAP: usize = 100_000;
        if self.kv_relation_absent_cached() {
            return Ok(Vec::new());
        }
        let pool = self.pool.get().await?;
        if self.kv_relation_is_absent(&pool).await? {
            return Ok(Vec::new());
        }
        self.relation_state.record_sql_attempt();
        let sql = format!(
            "SELECT key FROM {} WHERE key LIKE $1 LIMIT $2",
            self.table_name
        );
        let rows: Vec<(String,)> = match sqlx::query_as(&sql)
            .bind(pattern)
            .bind(i64::try_from(SAFETY_CAP).unwrap_or(100_000))
            .fetch_all(&pool)
            .await
        {
            Ok(rows) => rows,
            Err(e) if Self::is_undefined_table(&e) => {
                self.note_kv_undefined(&e);
                return Ok(Vec::new());
            }
            Err(e) => {
                return Err(StorageError::Database(format!(
                    "KV keys_like failed: {}",
                    e
                )))
            }
        };
        if rows.len() >= SAFETY_CAP {
            tracing::warn!(
                pattern,
                cap = SAFETY_CAP,
                "KV keys_like hit safety cap — prefer keys_with_prefix_limited (SPEC-070)"
            );
        }
        Ok(rows.into_iter().map(|(k,)| k).collect())
    }

    async fn keys_with_prefix(&self, prefix: &str) -> Result<Vec<String>> {
        // SPEC-070: delegate to limited path (O(limit), not unbounded SeqScan risk).
        const SAFETY_CAP: usize = 100_000;
        let (keys, truncated) = self.keys_with_prefix_limited(prefix, SAFETY_CAP).await?;
        if truncated {
            tracing::warn!(
                prefix,
                cap = SAFETY_CAP,
                "KV keys_with_prefix hit safety cap — prefer keys_with_prefix_limited (SPEC-070)"
            );
        }
        Ok(keys)
    }

    async fn keys_with_prefix_limited(
        &self,
        prefix: &str,
        limit: usize,
    ) -> Result<(Vec<String>, bool)> {
        // Clamp before i64 cast — usize::MAX as i64 is -1 on 64-bit targets.
        let limit = limit.clamp(1, 1_000_000);
        let pool = self.pool.get().await?;

        // SPEC-091 W1 relational cutover: `{doc}-chunk-` prefix scans resolve
        // from the `chunks` spine (keys synthesized in the legacy format so
        // every prefix-scan consumer migrates transparently).
        if matches!(
            crate::chunk_text_authority::chunk_text_authority_from_env(),
            crate::chunk_text_authority::ChunkTextAuthority::Relational
        ) {
            if let Some(doc_uuid) = prefix
                .strip_suffix("-chunk-")
                .and_then(|raw| uuid::Uuid::parse_str(raw).ok())
            {
                let fetch_limit = i64::try_from(limit).unwrap_or(1_000_000).saturating_add(1);
                let rows: Vec<(String,)> = sqlx::query_as(
                    "SELECT document_id::text || '-chunk-' || chunk_index \
                     FROM chunks WHERE document_id = $1 \
                     ORDER BY chunk_index LIMIT $2",
                )
                .bind(doc_uuid)
                .bind(fetch_limit)
                .fetch_all(&pool)
                .await
                .map_err(|e| {
                    StorageError::Database(format!("relational chunk prefix scan failed: {e}"))
                })?;
                let truncated = rows.len() > limit;
                return Ok((
                    rows.into_iter().take(limit).map(|(k,)| k).collect(),
                    truncated,
                ));
            }
        }

        if self.kv_relation_is_absent(&pool).await? {
            return Ok((Vec::new(), false));
        }

        let like_pattern = format!("{}%", escape_like_meta(prefix));
        // Fetch limit+1 so we can report truncation without a second COUNT query.
        let fetch_limit = i64::try_from(limit).unwrap_or(1_000_000).saturating_add(1);

        // No ORDER BY: with `key text_pattern_ops` the planner can Index Scan
        // and stop after LIMIT (O(limit)). ORDER BY key forced Sort/SeqScan
        // under en_US.utf8 before the pattern index existed.
        self.relation_state.record_sql_attempt();
        let sql = format!(
            "SELECT key FROM {} WHERE key LIKE $1 LIMIT $2",
            self.table_name
        );

        let rows: Vec<(String,)> = match sqlx::query_as(&sql)
            .bind(&like_pattern)
            .bind(fetch_limit)
            .fetch_all(&pool)
            .await
        {
            Ok(rows) => rows,
            Err(e) if Self::is_undefined_table(&e) => {
                self.note_kv_undefined(&e);
                return Ok((Vec::new(), false));
            }
            Err(e) => {
                return Err(StorageError::Database(format!(
                    "KV keys_with_prefix_limited failed: {}",
                    e
                )))
            }
        };

        let truncated = rows.len() > limit;
        Ok((
            rows.into_iter().take(limit).map(|(k,)| k).collect(),
            truncated,
        ))
    }

    async fn keys_with_suffix(&self, suffix: &str) -> Result<Vec<String>> {
        // SPEC-011 + SPEC-070: reverse-key index + safety LIMIT (no unbounded fetch).
        const SAFETY_CAP: usize = 100_000;
        let (keys, truncated) = self.keys_with_suffix_limited(suffix, SAFETY_CAP).await?;
        if truncated {
            tracing::warn!(
                suffix,
                cap = SAFETY_CAP,
                "KV keys_with_suffix hit safety cap — prefer keys_with_suffix_limited (SPEC-070)"
            );
        }
        Ok(keys)
    }

    async fn keys_with_suffix_limited(
        &self,
        suffix: &str,
        limit: usize,
    ) -> Result<(Vec<String>, bool)> {
        let limit = limit.clamp(1, 1_000_000);
        if self.kv_relation_absent_cached() {
            return Ok((Vec::new(), false));
        }
        let pool = self.pool.get().await?;
        if self.kv_relation_is_absent(&pool).await? {
            return Ok((Vec::new(), false));
        }
        let reversed: String = escape_like_meta(suffix).chars().rev().collect();
        let like_pattern = format!("{reversed}%");
        let fetch_limit = i64::try_from(limit).unwrap_or(1_000_000).saturating_add(1);

        // No ORDER BY key: that forced Sort over the full reverse-index match
        // set before LIMIT. Unordered LIMIT lets the bitmap/index path stop early.
        self.relation_state.record_sql_attempt();
        let sql = format!(
            "SELECT key FROM {} WHERE reverse(key) LIKE $1 LIMIT $2",
            self.table_name
        );

        let rows: Vec<(String,)> = match sqlx::query_as(&sql)
            .bind(&like_pattern)
            .bind(fetch_limit)
            .fetch_all(&pool)
            .await
        {
            Ok(rows) => rows,
            Err(e) if Self::is_undefined_table(&e) => {
                self.note_kv_undefined(&e);
                return Ok((Vec::new(), false));
            }
            Err(e) => {
                return Err(StorageError::Database(format!(
                    "KV keys_with_suffix_limited failed: {}",
                    e
                )))
            }
        };

        let truncated = rows.len() > limit;
        Ok((
            rows.into_iter().take(limit).map(|(k,)| k).collect(),
            truncated,
        ))
    }

    async fn keys(&self) -> Result<Vec<String>> {
        if self.kv_relation_absent_cached() {
            return Ok(Vec::new());
        }
        let pool = self.pool.get().await?;
        if self.kv_relation_is_absent(&pool).await? {
            return Ok(Vec::new());
        }

        self.relation_state.record_sql_attempt();
        let sql = format!("SELECT key FROM {}", self.table_name);

        let rows: Vec<(String,)> = match sqlx::query_as(&sql).fetch_all(&pool).await {
            Ok(rows) => rows,
            Err(e) if Self::is_undefined_table(&e) => {
                self.note_kv_undefined(&e);
                return Ok(Vec::new());
            }
            Err(e) => return Err(StorageError::Database(format!("KV keys failed: {}", e))),
        };

        Ok(rows.into_iter().map(|(k,)| k).collect())
    }

    async fn clear(&self) -> Result<()> {
        if self.kv_relation_absent_cached() {
            return Ok(());
        }
        let pool = self.pool.get().await?;
        if self.kv_relation_is_absent(&pool).await? {
            return Ok(());
        }

        // TRUNCATE is faster than DELETE; row triggers don't fire — reset stats explicitly.
        self.relation_state.record_sql_attempt();
        let sql = format!("TRUNCATE {}", self.table_name);

        match sqlx::query(&sql).execute(&pool).await {
            Ok(_) => {}
            // Post-drop there is nothing to clear (Wave D).
            Err(e) if Self::is_undefined_table(&e) => {
                self.note_kv_undefined(&e);
                return Ok(());
            }
            Err(e) => return Err(StorageError::Database(format!("KV clear failed: {}", e))),
        }

        Ok(())
    }

    /// Atomically transition document status if current status matches expected.
    ///
    /// @implements FIX-RACE-01: Prevent TOCTOU race conditions
    ///
    /// # WHY: Atomic Compare-And-Swap
    ///
    /// Uses PostgreSQL's atomic UPDATE with WHERE clause to ensure only one
    /// process can successfully transition the status. The affected row count
    /// tells us if the transition succeeded (1) or failed (0).
    ///
    /// SQL: UPDATE ... SET value = jsonb_set(...) WHERE key = $1 AND value->>'status' = $2
    ///
    /// This is atomic at the database level - no race window possible.
    async fn transition_if_status(
        &self,
        key: &str,
        expected_status: &str,
        new_status: &str,
    ) -> Result<bool> {
        // SPEC-091 Wave D: shell keys transition on `documents.metadata` when
        // the METADATA family is relational (same single-statement CAS).
        if Self::shell_family_reads_relational() {
            let pool = self.pool.get().await?;
            if let Some(transitioned) = super::document_shell::shell_transition_status(
                &pool,
                key,
                expected_status,
                new_status,
            )
            .await?
            {
                return Ok(transitioned);
            }
        }

        if self.kv_relation_absent_cached() {
            return Ok(false);
        }
        let pool = self.pool.get().await?;
        if self.kv_relation_is_absent(&pool).await? {
            return Ok(false);
        }

        // Atomic update: only succeeds if current status matches expected
        // jsonb_set updates the 'status' field within the JSONB value
        self.relation_state.record_sql_attempt();
        let sql = format!(
            r#"
            UPDATE {}
            SET value = jsonb_set(value, '{{status}}', to_jsonb($3::text)),
                updated_at = NOW()
            WHERE key = $1 AND value->>'status' = $2
            "#,
            self.table_name
        );

        let result = match sqlx::query(&sql)
            .bind(key)
            .bind(expected_status)
            .bind(new_status)
            .execute(&pool)
            .await
        {
            Ok(result) => result,
            // Dropped KV relation: nothing to transition (Wave D).
            Err(e) if Self::is_undefined_table(&e) => {
                self.note_kv_undefined(&e);
                return Ok(false);
            }
            Err(e) => {
                return Err(StorageError::Database(format!(
                    "KV transition_if_status failed: {}",
                    e
                )))
            }
        };

        // rows_affected = 1 means transition succeeded
        // rows_affected = 0 means status didn't match (or key not found)
        Ok(result.rows_affected() == 1)
    }
}

impl std::fmt::Debug for PostgresKVStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PostgresKVStorage")
            .field("namespace", &self.namespace)
            .field("table_name", &self.table_name)
            .finish()
    }
}

/// Escape `%`, `_`, and `\` for PostgreSQL `LIKE` patterns (literal match).
fn escape_like_meta(raw: &str) -> String {
    raw.chars()
        .flat_map(|c| match c {
            '%' | '_' | '\\' => vec!['\\', c],
            _ => vec![c],
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kv_storage_creation() {
        let config = PostgresConfig::default().with_namespace("test");
        let storage = PostgresKVStorage::new(config);

        // Table name includes schema prefix for PostgreSQL
        assert_eq!(storage.table_name, "public.eq_eq_test_kv");
    }

    #[test]
    fn escape_like_meta_escapes_wildcards() {
        assert_eq!(escape_like_meta("wsdoc:ab"), "wsdoc:ab");
        assert_eq!(escape_like_meta("a%b_c\\d"), "a\\%b\\_c\\\\d");
        assert_eq!(escape_like_meta("-metadata"), "-metadata");
    }

    #[test]
    fn enforce_workspace_scoped_keys_rejects_mixed_workspaces() {
        let keys = ["wsdoc:ws-a:doc1", "wsdoc:ws-b:doc2"];
        let err = enforce_workspace_scoped_keys(keys.into_iter()).unwrap_err();
        assert!(err.to_string().contains("mixes workspace"));
    }

    #[test]
    fn enforce_workspace_scoped_keys_rejects_malformed() {
        let keys = ["wsdoc:missing-doc-id"];
        let err = enforce_workspace_scoped_keys(keys.into_iter()).unwrap_err();
        assert!(err.to_string().contains("Malformed"));
    }

    #[test]
    fn enforce_workspace_scoped_keys_allows_same_workspace() {
        let keys = ["wsdoc:ws-a:doc1", "staging:hash:ws-a:abc", "doc1-metadata"];
        assert!(enforce_workspace_scoped_keys(keys.into_iter()).is_ok());
    }

    #[test]
    fn e2e_kv_upsert_all_or_nothing() {
        // C-22 / matrix: upsert uses a single transaction (begin → batches → commit).
        let src = include_str!("kv.rs");
        assert!(src.contains("pool.begin()"));
        assert!(src.contains("tx.commit()"));
        assert!(src.contains("C-22"));
        assert!(src.contains("all batches commit atomically"));
    }
}
