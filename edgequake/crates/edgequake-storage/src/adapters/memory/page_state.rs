//! In-memory page state storage (SPEC-151 tests).

use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::RwLock;
use uuid::Uuid;

use crate::error::Result;
use crate::page_state_storage::*;

use super::lock::map_lock_err;

#[derive(Debug, Default)]
pub struct MemoryPageStateStorage {
    rows: RwLock<HashMap<(Uuid, i32), DocumentPageState>>,
}

impl MemoryPageStateStorage {
    pub fn new() -> Self {
        Self::default()
    }
}

fn blank_row(document_id: Uuid, workspace_id: Uuid, page_number: i32) -> DocumentPageState {
    let now = Utc::now();
    DocumentPageState {
        page_state_id: Uuid::new_v4(),
        document_id,
        workspace_id,
        page_number,
        parse_status: PAGE_STAGE_PENDING.to_string(),
        parse_error: None,
        parse_attempts: 0,
        parse_method: None,
        parse_model: None,
        raw_markdown: None,
        raw_sha256: None,
        figures_status: PAGE_STAGE_PENDING.to_string(),
        figures_count: 0,
        figures_error: None,
        entities_status: PAGE_STAGE_PENDING.to_string(),
        chunk_count: 0,
        failed_chunk_count: 0,
        entities_error: None,
        last_track_id: None,
        created_at: now,
        updated_at: now,
    }
}

#[async_trait]
impl PageStateStorage for MemoryPageStateStorage {
    async fn list_page_states(
        &self,
        document_id: Uuid,
        workspace_id: Uuid,
    ) -> Result<Vec<DocumentPageState>> {
        let rows = self.rows.read().map_err(map_lock_err)?;
        let mut out: Vec<_> = rows
            .values()
            .filter(|r| r.document_id == document_id && r.workspace_id == workspace_id)
            .cloned()
            .collect();
        out.sort_by_key(|r| r.page_number);
        Ok(out)
    }

    async fn upsert_parse_batch(
        &self,
        document_id: Uuid,
        workspace_id: Uuid,
        rows: &[UpsertPageParse],
    ) -> Result<()> {
        let mut map = self.rows.write().map_err(map_lock_err)?;
        for row in rows {
            let key = (document_id, row.page_number);
            let mut cur = map
                .remove(&key)
                .unwrap_or_else(|| blank_row(document_id, workspace_id, row.page_number));
            cur.parse_status = row.status.clone();
            cur.parse_error = row.error.clone();
            if row.increment_attempts {
                cur.parse_attempts += 1;
            }
            if row.method.is_some() {
                cur.parse_method = row.method.clone();
            }
            if row.model.is_some() {
                cur.parse_model = row.model.clone();
            }
            if row.raw_markdown.is_some() {
                cur.raw_markdown = row.raw_markdown.clone();
            }
            if row.raw_sha256.is_some() {
                cur.raw_sha256 = row.raw_sha256.clone();
            }
            if row.track_id.is_some() {
                cur.last_track_id = row.track_id.clone();
            }
            cur.updated_at = Utc::now();
            map.insert(key, cur);
        }
        Ok(())
    }

    async fn upsert_figures_batch(
        &self,
        document_id: Uuid,
        workspace_id: Uuid,
        rows: &[UpsertPageFigures],
    ) -> Result<()> {
        let mut map = self.rows.write().map_err(map_lock_err)?;
        for row in rows {
            let key = (document_id, row.page_number);
            let mut cur = map
                .remove(&key)
                .unwrap_or_else(|| blank_row(document_id, workspace_id, row.page_number));
            cur.figures_status = row.status.clone();
            cur.figures_count = row.count;
            cur.figures_error = row.error.clone();
            if row.track_id.is_some() {
                cur.last_track_id = row.track_id.clone();
            }
            cur.updated_at = Utc::now();
            map.insert(key, cur);
        }
        Ok(())
    }

    async fn upsert_entities_batch(
        &self,
        document_id: Uuid,
        workspace_id: Uuid,
        rows: &[UpsertPageEntities],
    ) -> Result<()> {
        let mut map = self.rows.write().map_err(map_lock_err)?;
        for row in rows {
            let key = (document_id, row.page_number);
            let mut cur = map
                .remove(&key)
                .unwrap_or_else(|| blank_row(document_id, workspace_id, row.page_number));
            cur.entities_status = row.status.clone();
            cur.chunk_count = row.chunk_count;
            cur.failed_chunk_count = row.failed_chunk_count;
            cur.entities_error = row.error.clone();
            if row.track_id.is_some() {
                cur.last_track_id = row.track_id.clone();
            }
            cur.updated_at = Utc::now();
            map.insert(key, cur);
        }
        Ok(())
    }

    async fn delete_for_document(&self, document_id: Uuid, workspace_id: Uuid) -> Result<()> {
        let mut map = self.rows.write().map_err(map_lock_err)?;
        map.retain(|_, r| !(r.document_id == document_id && r.workspace_id == workspace_id));
        Ok(())
    }
}
