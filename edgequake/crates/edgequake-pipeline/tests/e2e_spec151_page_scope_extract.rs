//! SPEC-151 — Page-scope extract: LLM only for dirty overlapping chunks.
//!
//! Proves `decide_chunk_extract` + `process_with_resilience_cancellable_reuse`
//! never call the extractor for clean hash hits, and clean hash misses fail
//! closed without LLM calls on those pages.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use edgequake_pipeline::{
    chunker::{ChunkStrategy, ChunkerConfig, TextChunk},
    extractor::{EntityExtractor, ExtractedEntity, ExtractionResult},
    ChunkReuseIndex, Pipeline, PipelineConfig, SPEC151_CLEAN_HASH_MISS,
};

struct CountingExtractor {
    calls: AtomicUsize,
    mark: String,
}

impl CountingExtractor {
    fn new(mark: &str) -> Self {
        Self {
            calls: AtomicUsize::new(0),
            mark: mark.to_string(),
        }
    }

    fn call_count(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl EntityExtractor for CountingExtractor {
    async fn extract(
        &self,
        chunk: &TextChunk,
    ) -> edgequake_pipeline::error::Result<ExtractionResult> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let mut result = ExtractionResult::new(&chunk.id);
        result.add_entity(ExtractedEntity::new(
            format!("ENTITY_{}", self.mark),
            "CONCEPT",
            format!("extracted for {}", self.mark),
        ));
        Ok(result)
    }

    fn name(&self) -> &str {
        "counting-extractor"
    }
}

fn pin_page_chunk_env() {
    // Keep hard page emit so each page is independently attributable.
    std::env::set_var("EDGEQUAKE_PDF_CROSS_PAGE_PACK", "0");
    std::env::set_var("EDGEQUAKE_PDF_PACK", "0");
}

fn pdf_config() -> PipelineConfig {
    PipelineConfig {
        enable_entity_extraction: true,
        enable_relationship_extraction: false,
        enable_chunk_embeddings: false,
        enable_entity_embeddings: false,
        chunk_strategy: ChunkStrategy::Pdf,
        chunker: ChunkerConfig {
            chunk_size: 500,
            chunk_overlap: 0,
            min_chunk_size: 10,
            ..ChunkerConfig::default()
        },
        ..Default::default()
    }
}

fn page_paragraph(label: &str) -> String {
    // Enough tokens that Recursive/Pdf inner keeps a per-page chunk.
    format!(
        "{label}. This paragraph is long enough to survive minimum chunk size \
         filters and remain attributed to a single PDF page during page-aware \
         chunking for SPEC-151 hybrid reuse tests. Extra words: river valley \
         mountain peak ocean tide forest meadow canyon plateau."
    )
}

fn three_page_md(p1: &str, p2: &str, p3: &str) -> String {
    format!(
        "<!-- edgequake-page:1 -->\n\n{p1}\n\n\
         <!-- edgequake-page:2 -->\n\n{p2}\n\n\
         <!-- edgequake-page:3 -->\n\n{p3}\n"
    )
}

fn overlaps_page(chunk: &TextChunk, page: u32) -> bool {
    let start = chunk.page_start.unwrap_or(0);
    let end = chunk.page_end.unwrap_or(start);
    start > 0 && (start..=end).contains(&page)
}

#[tokio::test]
async fn e2e_llm_only_for_dirty_overlapping_pages() {
    pin_page_chunk_env();
    let doc_id = "spec151-scope";
    let md = three_page_md(
        &page_paragraph("Alpha page one"),
        &page_paragraph("Beta page two"),
        &page_paragraph("Gamma page three"),
    );

    let seed_extractor = Arc::new(CountingExtractor::new("SEED"));
    let seed_pipeline = Pipeline::new(pdf_config()).with_extractor(seed_extractor.clone());
    let seeded = seed_pipeline
        .process(doc_id, &md)
        .await
        .expect("seed extract");

    let pages_present: Vec<_> = seeded.chunks.iter().filter_map(|c| c.page_start).collect();
    assert!(
        pages_present.contains(&1) && pages_present.contains(&2) && pages_present.contains(&3),
        "expected page-aware chunks for pages 1..=3, got page_starts={pages_present:?} count={}",
        seeded.stats.chunk_count
    );

    let reuse = ChunkReuseIndex::from_snapshot(&seeded.chunks, &seeded.extractions, [2u32]);
    assert!(
        !reuse.is_empty(),
        "clean pages must seed reuse index (chunks={}, extractions={})",
        seeded.chunks.len(),
        seeded.extractions.len()
    );

    let extractor = Arc::new(CountingExtractor::new("DIRTY"));
    let pipeline = Pipeline::new(pdf_config()).with_extractor(extractor.clone());
    let result = pipeline
        .process_with_resilience_cancellable_reuse(
            doc_id,
            &md,
            None,
            None,
            None,
            None,
            Some(reuse),
            None,
        )
        .await
        .expect("hybrid extract");

    let dirty_n = result.chunks.iter().filter(|c| overlaps_page(c, 2)).count();
    assert!(dirty_n >= 1, "must have at least one page-2 chunk");
    assert_eq!(
        extractor.call_count(),
        dirty_n,
        "LLM calls must equal dirty overlapping chunks only"
    );
    assert_eq!(result.stats.failed_chunks, 0);

    let has_seed = result
        .extractions
        .iter()
        .any(|e| e.entities.iter().any(|ent| ent.name.contains("SEED")));
    let has_dirty = result
        .extractions
        .iter()
        .any(|e| e.entities.iter().any(|ent| ent.name.contains("DIRTY")));
    assert!(has_seed, "clean pages must keep reused SEED entities");
    assert!(has_dirty, "dirty page must have fresh DIRTY entities");
}

#[tokio::test]
async fn e2e_clean_hash_miss_fails_closed_without_llm_on_clean() {
    pin_page_chunk_env();
    let doc_id = "spec151-clean-miss";
    let original = three_page_md(
        &page_paragraph("Stable original page one"),
        &page_paragraph("Page two selected for reprocess"),
        &page_paragraph("Stable original page three"),
    );

    let seed_extractor = Arc::new(CountingExtractor::new("SEED"));
    let seed_pipeline = Pipeline::new(pdf_config()).with_extractor(seed_extractor);
    let seeded = seed_pipeline
        .process(doc_id, &original)
        .await
        .expect("seed");

    let reuse = ChunkReuseIndex::from_snapshot(&seeded.chunks, &seeded.extractions, [2u32]);

    let spliced = three_page_md(
        &page_paragraph("CHANGED page one no longer matches snapshot"),
        &page_paragraph("Page two selected for reprocess"),
        &page_paragraph("Stable original page three"),
    );

    let extractor = Arc::new(CountingExtractor::new("DIRTY"));
    let pipeline = Pipeline::new(pdf_config()).with_extractor(extractor.clone());
    let result = pipeline
        .process_with_resilience_cancellable_reuse(
            doc_id,
            &spliced,
            None,
            None,
            None,
            None,
            Some(reuse),
            None,
        )
        .await;

    match result {
        Ok(r) => {
            let miss = r.stats.chunk_errors.as_ref().is_some_and(|errs| {
                errs.iter()
                    .any(|e| e.error_message.contains(SPEC151_CLEAN_HASH_MISS))
            });
            assert!(
                miss,
                "must surface SPEC151_CLEAN_HASH_MISS; errors={:?}",
                r.stats.chunk_errors
            );
            let dirty_n = r.chunks.iter().filter(|c| overlaps_page(c, 2)).count();
            assert!(
                extractor.call_count() <= dirty_n,
                "LLM must not run for clean hash-miss (calls={}, dirty={})",
                extractor.call_count(),
                dirty_n
            );
            // Clean page-1 must not have been LLM'd as DIRTY for that page alone:
            // all DIRTY calls are attributable to dirty overlap only.
            assert_eq!(extractor.call_count(), dirty_n);
        }
        Err(e) => {
            let msg = e.to_string();
            assert!(
                msg.contains(SPEC151_CLEAN_HASH_MISS) || msg.contains("failed extraction"),
                "unexpected err: {msg}"
            );
        }
    }
}
