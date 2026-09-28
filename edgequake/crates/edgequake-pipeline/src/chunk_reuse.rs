//! SPEC-151 — Chunk extraction reuse by content hash (LAW-151-4).

use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};

use crate::chunker::TextChunk;
use crate::extractor::ExtractionResult;

/// Index of prior extractions keyed by chunk id and content hash.
///
/// Dirty pages (overlapping `excluded_pages`) are never reused.
#[derive(Debug, Clone, Default)]
pub struct ChunkReuseIndex {
    by_id: HashMap<String, ExtractionResult>,
    by_content_hash: HashMap<String, ExtractionResult>,
    /// 1-indexed pages whose chunks must be re-extracted.
    excluded_pages: HashSet<u32>,
}

/// Stable error prefix for clean-page content-hash miss (no LLM; abort before retract).
pub const SPEC151_CLEAN_HASH_MISS: &str = "SPEC151_CLEAN_HASH_MISS";

/// Per-chunk extract decision under page-scope reprocess (LAW-151-4).
#[derive(Debug, Clone)]
pub enum ChunkExtractAction {
    /// Rebind a prior extraction — no LLM.
    Reuse(ExtractionResult),
    /// Chunk overlaps selected (dirty) pages — call LLM.
    ExtractFresh,
    /// Outside selection; content-hash miss — refuse extract (no LLM, abort job).
    FailClosed,
}

/// Decide whether this chunk may call the LLM.
///
/// When a [`ChunkReuseIndex`] is present, positional `resume_by_chunk_id` is
/// ignored (chunk ids shift when page text length changes).
pub fn decide_chunk_extract(
    reuse_index: Option<&ChunkReuseIndex>,
    resume_by_chunk_id: Option<&HashMap<String, ExtractionResult>>,
    chunk: &TextChunk,
) -> ChunkExtractAction {
    if let Some(idx) = reuse_index {
        if let Some(prior) = idx.lookup(chunk) {
            return ChunkExtractAction::Reuse(prior);
        }
        // Page-scoped hybrid: only dirty (overlapping) chunks may call the LLM.
        // Clean hash miss must not emit empty extractions (would drop entities on retract).
        if idx.has_excluded_pages() && !idx.overlaps_excluded(chunk) {
            return ChunkExtractAction::FailClosed;
        }
        return ChunkExtractAction::ExtractFresh;
    }
    if let Some(map) = resume_by_chunk_id {
        if let Some(prior) = map.get(&chunk.id) {
            return ChunkExtractAction::Reuse(prior.clone());
        }
    }
    ChunkExtractAction::ExtractFresh
}

impl ChunkReuseIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_excluded_pages(pages: impl IntoIterator<Item = u32>) -> Self {
        Self {
            excluded_pages: pages.into_iter().collect(),
            ..Self::default()
        }
    }

    /// True when this index gates page-scoped reprocess (non-empty dirty set).
    pub fn has_excluded_pages(&self) -> bool {
        !self.excluded_pages.is_empty()
    }

    /// True when the chunk's page span overlaps any dirty (excluded) page.
    pub fn overlaps_excluded(&self, chunk: &TextChunk) -> bool {
        self.chunk_overlaps_excluded(chunk)
    }

    /// Normalize whitespace then SHA-256 hex (stable reuse key).
    pub fn content_hash(content: &str) -> String {
        let normalized: String = content.split_whitespace().collect::<Vec<_>>().join(" ");
        let mut hasher = Sha256::new();
        hasher.update(normalized.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Ingest prior chunks + matching extractions (by source_chunk_id).
    pub fn ingest_snapshot(&mut self, chunks: &[TextChunk], extractions: &[ExtractionResult]) {
        let by_source: HashMap<&str, &ExtractionResult> = extractions
            .iter()
            .map(|e| (e.source_chunk_id.as_str(), e))
            .collect();
        for chunk in chunks {
            let Some(ext) = by_source.get(chunk.id.as_str()) else {
                continue;
            };
            if self.chunk_overlaps_excluded(chunk) {
                continue;
            }
            let hash = Self::content_hash(&chunk.content);
            self.by_id.insert(chunk.id.clone(), (*ext).clone());
            self.by_content_hash.insert(hash, (*ext).clone());
        }
    }

    /// Build from snapshot with excluded pages.
    pub fn from_snapshot(
        chunks: &[TextChunk],
        extractions: &[ExtractionResult],
        excluded_pages: impl IntoIterator<Item = u32>,
    ) -> Self {
        let mut idx = Self::with_excluded_pages(excluded_pages);
        idx.ingest_snapshot(chunks, extractions);
        idx
    }

    /// Also seed resume-by-id map (mid-doc crash checkpoint).
    pub fn seed_by_id(&mut self, map: HashMap<String, ExtractionResult>) {
        for (id, ext) in map {
            self.by_id.insert(id, ext);
        }
    }

    fn chunk_overlaps_excluded(&self, chunk: &TextChunk) -> bool {
        if self.excluded_pages.is_empty() {
            return false;
        }
        let start = chunk.page_start.unwrap_or(0);
        let end = chunk.page_end.unwrap_or(start);
        if start == 0 && end == 0 {
            return false;
        }
        (start..=end).any(|p| self.excluded_pages.contains(&p))
    }

    /// Lookup reusable extraction for a new chunk; rebinds source_chunk_id.
    ///
    /// When page-scoped (`has_excluded_pages`), match **only by content hash**
    /// (LAW-151-4). Positional chunk ids (`doc-chunk-N`) stay stable when page
    /// text length is unchanged but content differs — id hits would wrongly
    /// reuse stale extractions and skip FailClosed.
    pub fn lookup(&self, chunk: &TextChunk) -> Option<ExtractionResult> {
        if self.chunk_overlaps_excluded(chunk) {
            return None;
        }
        let hash = Self::content_hash(&chunk.content);
        if let Some(prior) = self.by_content_hash.get(&hash) {
            return Some(prior.clone().rebind_chunk_id(&chunk.id));
        }
        if self.has_excluded_pages() {
            return None;
        }
        self.by_id
            .get(&chunk.id)
            .map(|prior| prior.clone().rebind_chunk_id(&chunk.id))
    }

    pub fn len(&self) -> usize {
        self.by_content_hash.len().max(self.by_id.len())
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty() && self.by_content_hash.is_empty()
    }

    /// Convert to the legacy resume map (by chunk id only).
    pub fn into_by_id_map(self) -> HashMap<String, ExtractionResult> {
        self.by_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(id: &str, content: &str, page: u32) -> TextChunk {
        let mut c = TextChunk::new(id, content, 0, 0, content.len());
        c.page_start = Some(page);
        c.page_end = Some(page);
        c
    }

    #[test]
    fn reuses_by_content_hash_across_id_shift() {
        let old = chunk("doc-chunk-0", "Hello world", 1);
        let ext = ExtractionResult::new("doc-chunk-0");
        let idx = ChunkReuseIndex::from_snapshot(&[old], &[ext], []);
        let new_chunk = chunk("doc-chunk-5", "Hello   world", 1);
        let hit = idx.lookup(&new_chunk).expect("reuse");
        assert_eq!(hit.source_chunk_id, "doc-chunk-5");
    }

    #[test]
    fn excluded_pages_never_reuse() {
        let old = chunk("doc-chunk-0", "Hello world", 3);
        let ext = ExtractionResult::new("doc-chunk-0");
        let idx = ChunkReuseIndex::from_snapshot(&[old], &[ext], [3]);
        let new_chunk = chunk("doc-chunk-0", "Hello world", 3);
        assert!(idx.lookup(&new_chunk).is_none());
    }

    #[test]
    fn cross_page_chunk_dirty_if_any_overlap() {
        let mut old = chunk("doc-chunk-0", "span", 2);
        old.page_end = Some(3);
        let ext = ExtractionResult::new("doc-chunk-0");
        let idx = ChunkReuseIndex::from_snapshot(&[old.clone()], &[ext], [3]);
        // Snapshot ingest skipped excluded; lookup of overlapping new chunk also None
        let mut neu = chunk("doc-chunk-9", "span", 2);
        neu.page_end = Some(3);
        assert!(idx.lookup(&neu).is_none());
        assert!(idx.overlaps_excluded(&neu));
    }

    #[test]
    fn decide_selected_page_forces_extract_fresh() {
        let old = chunk("doc-chunk-0", "Hello world", 1);
        let ext = ExtractionResult::new("doc-chunk-0");
        let idx = ChunkReuseIndex::from_snapshot(&[old], &[ext], [2]);
        let dirty = chunk("doc-chunk-1", "new dirty text", 2);
        match decide_chunk_extract(Some(&idx), None, &dirty) {
            ChunkExtractAction::ExtractFresh => {}
            other => panic!("expected ExtractFresh, got {other:?}"),
        }
    }

    #[test]
    fn decide_clean_hash_miss_fails_closed_no_llm() {
        let old = chunk("doc-chunk-0", "stable page one", 1);
        let ext = ExtractionResult::new("doc-chunk-0");
        let idx = ChunkReuseIndex::from_snapshot(&[old], &[ext], [2]);
        let clean_miss = chunk("doc-chunk-9", "boundary shifted text", 1);
        match decide_chunk_extract(Some(&idx), None, &clean_miss) {
            ChunkExtractAction::FailClosed => {}
            other => panic!("expected FailClosed, got {other:?}"),
        }
    }

    #[test]
    fn decide_ignores_positional_id_resume_when_reuse_index_set() {
        let old = chunk("doc-chunk-0", "stable", 1);
        let ext = ExtractionResult::new("doc-chunk-0");
        let idx = ChunkReuseIndex::from_snapshot(&[old], &[ext], [2]);
        let mut resume = HashMap::new();
        // Wrong content under positional id — must not be used for dirty page.
        resume.insert(
            "doc-chunk-1".to_string(),
            ExtractionResult::new("doc-chunk-1"),
        );
        let dirty = chunk("doc-chunk-1", "selected page text", 2);
        match decide_chunk_extract(Some(&idx), Some(&resume), &dirty) {
            ChunkExtractAction::ExtractFresh => {}
            other => panic!("id resume must not skip dirty extract, got {other:?}"),
        }
        // Clean miss must FailClosed even if resume has a positional hit.
        let clean_miss = chunk("doc-chunk-1", "unrelated clean text", 1);
        match decide_chunk_extract(Some(&idx), Some(&resume), &clean_miss) {
            ChunkExtractAction::FailClosed => {}
            other => panic!("id resume must not apply under reuse index, got {other:?}"),
        }
    }

    #[test]
    fn decide_without_reuse_index_uses_id_resume() {
        let mut resume = HashMap::new();
        resume.insert(
            "doc-chunk-3".to_string(),
            ExtractionResult::new("doc-chunk-3"),
        );
        let c = chunk("doc-chunk-3", "anything", 5);
        match decide_chunk_extract(None, Some(&resume), &c) {
            ChunkExtractAction::Reuse(r) => assert_eq!(r.source_chunk_id, "doc-chunk-3"),
            other => panic!("expected Reuse from id resume, got {other:?}"),
        }
    }
}
