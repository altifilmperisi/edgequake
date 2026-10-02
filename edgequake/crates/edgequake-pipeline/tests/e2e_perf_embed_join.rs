//! SPEC-156 — Parallel chunk/entity/relationship embeds under one shared budget.
//!
//! First principle: independent embed stages must overlap, but total in-flight
//! HTTP calls stay ≤ `embed_max_async` (one shared semaphore). These e2e tests
//! drive `Pipeline::process` so the join lives on the real production path.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use edgequake_pipeline::{
    chunker::{ChunkerConfig, TextChunk},
    extractor::{EntityExtractor, ExtractedEntity, ExtractedRelationship, ExtractionResult},
    Pipeline, PipelineConfig,
};

/// Extractor that yields many unique entities + relationships (no LLM).
struct FanoutExtractor;

#[async_trait]
impl EntityExtractor for FanoutExtractor {
    async fn extract(
        &self,
        chunk: &TextChunk,
    ) -> edgequake_pipeline::error::Result<ExtractionResult> {
        let mut result = ExtractionResult::new(&chunk.id);
        // Distinct names per chunk so unique-before-embed still leaves work.
        result.add_entity(ExtractedEntity::new(
            format!("ENTITY_A_{}", chunk.index),
            "CONCEPT",
            format!("description A for chunk {}", chunk.index),
        ));
        result.add_entity(ExtractedEntity::new(
            format!("ENTITY_B_{}", chunk.index),
            "CONCEPT",
            format!("description B for chunk {}", chunk.index),
        ));
        result.add_relationship(
            ExtractedRelationship::new(
                format!("ENTITY_A_{}", chunk.index),
                format!("ENTITY_B_{}", chunk.index),
                "RELATED_TO",
            )
            .with_description(format!("rel for chunk {}", chunk.index)),
        );
        // Wall-clock for SPEC-156 extraction_time_ms fill is proven in
        // e2e_perf_extraction_fanout; keep this extractor instantaneous.
        Ok(result)
    }

    fn name(&self) -> &str {
        "fanout-extractor"
    }
}

/// Latency embedder that records peak in-flight `embed` calls.
struct PeakLatencyEmbedder {
    delay: Duration,
    in_flight: AtomicUsize,
    peak: AtomicUsize,
    calls: AtomicUsize,
}

impl PeakLatencyEmbedder {
    fn new(delay_ms: u64) -> Self {
        Self {
            delay: Duration::from_millis(delay_ms),
            in_flight: AtomicUsize::new(0),
            peak: AtomicUsize::new(0),
            calls: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl edgequake_llm::traits::EmbeddingProvider for PeakLatencyEmbedder {
    fn name(&self) -> &str {
        // Cloud-scoped so embed_max_async stays > 1 (not local clamp).
        "mistral"
    }
    fn model(&self) -> &str {
        "peak-latency-embed"
    }
    fn dimension(&self) -> usize {
        4
    }
    fn max_tokens(&self) -> usize {
        100_000
    }
    fn max_batch_size(&self) -> usize {
        // Force one text per HTTP call so peak ≈ concurrent stages × sub-batches.
        1
    }
    async fn embed(&self, texts: &[String]) -> edgequake_llm::Result<Vec<Vec<f32>>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let now = self.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
        self.peak.fetch_max(now, Ordering::SeqCst);
        tokio::time::sleep(self.delay).await;
        self.in_flight.fetch_sub(1, Ordering::SeqCst);
        Ok(texts.iter().map(|_| vec![0.1, 0.2, 0.3, 0.4]).collect())
    }
}

fn multi_chunk_doc(paragraphs: usize) -> String {
    (0..paragraphs)
        .map(|i| {
            format!(
                "Paragraph {i}. River valley {i} meets mountain peak {i} where ocean tide {i} \
                 reaches forest meadow {i} below canyon plateau {i}."
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn pipeline_config() -> PipelineConfig {
    PipelineConfig {
        enable_entity_extraction: true,
        enable_relationship_extraction: true,
        enable_chunk_embeddings: true,
        enable_entity_embeddings: true,
        enable_relationship_embeddings: true,
        max_concurrent_extractions: 4,
        // Pin embed fan-out via env in the test body; config field may be unused.
        chunker: ChunkerConfig {
            chunk_size: 40,
            chunk_overlap: 0,
            min_chunk_size: 5,
            ..ChunkerConfig::default()
        },
        ..Default::default()
    }
}

#[tokio::test]
async fn e2e_pipeline_parallel_embeds_populate_all_stages_under_shared_budget() {
    // Cap shared slots at 3 so peak cannot exceed that across try_join stages.
    std::env::set_var("EDGEQUAKE_EMBED_MAX_ASYNC", "3");

    let embedder = Arc::new(PeakLatencyEmbedder::new(25));
    let pipeline = Pipeline::new(pipeline_config())
        .with_extractor(Arc::new(FanoutExtractor))
        .with_embedding_provider(
            embedder.clone() as Arc<dyn edgequake_llm::traits::EmbeddingProvider>
        );

    let doc = multi_chunk_doc(12);
    let start = Instant::now();
    let result = pipeline
        .process("spec156-embed-join", &doc)
        .await
        .expect("pipeline");
    let elapsed = start.elapsed();

    assert!(result.chunks.len() >= 6, "need multiple chunks");
    for chunk in &result.chunks {
        assert!(
            chunk.embedding.is_some(),
            "chunk {} missing embedding after join",
            chunk.id
        );
    }
    let mut entity_emb = 0usize;
    let mut rel_emb = 0usize;
    for ext in &result.extractions {
        for e in &ext.entities {
            if e.embedding.is_some() {
                entity_emb += 1;
            }
        }
        for r in &ext.relationships {
            if r.embedding.is_some() {
                rel_emb += 1;
            }
        }
    }
    assert!(
        entity_emb >= 6,
        "entity embeddings missing (got {entity_emb})"
    );
    assert!(
        rel_emb >= 3,
        "relationship embeddings missing (got {rel_emb})"
    );

    let peak = embedder.peak.load(Ordering::SeqCst);
    let calls = embedder.calls.load(Ordering::SeqCst);
    assert!(calls > 3, "expected many embed HTTP calls, got {calls}");
    assert!(
        peak > 1 && peak <= 3,
        "shared embed budget must bound peak in-flight to ≤3 (peak={peak})"
    );
    // Serial 3 stages × many single-text calls at 25ms would be much slower;
    // joined path under 3 slots should finish well under a naive serial floor.
    let serial_floor = Duration::from_millis(25 * calls as u64 * 8 / 10);
    assert!(
        elapsed < serial_floor,
        "joined embeds should overlap: elapsed={elapsed:?} serial_floor={serial_floor:?} calls={calls}"
    );

    std::env::remove_var("EDGEQUAKE_EMBED_MAX_ASYNC");
}

/// Wiring contracts: production code must keep the SPEC-156 shape.
#[test]
fn contract_spec156_wiring_ssot() {
    let emb = include_str!("../src/pipeline/helpers/embeddings.rs");
    let prod_emb = emb.split("#[cfg(test)]").next().expect("prod embeddings");
    assert!(
        prod_emb.contains("tokio::try_join!"),
        "generate_all_embeddings must try_join! chunk/entity/relationship embeds"
    );
    assert!(
        prod_emb.contains("Semaphore::new(max_async"),
        "shared embed semaphore must be sized from embed_max_async"
    );

    let extract = include_str!("../src/pipeline/extraction.rs");
    let prod_extract = extract.split("#[cfg(test)]").next().expect("prod extract");
    assert!(
        !prod_extract.contains("Semaphore::new("),
        "extraction must not wrap buffer_unordered in a redundant Semaphore"
    );
    assert!(
        prod_extract.contains("buffer_unordered(self.config.max_concurrent_extractions)"),
        "extraction concurrency SSOT is buffer_unordered(max_concurrent_extractions)"
    );
    assert!(
        prod_extract.contains("result.extraction_time_ms = time_ms"),
        "resilient extract must stamp wall-clock extraction_time_ms"
    );

    let glean = include_str!("../src/extractor/gleaning.rs");
    let prod_glean = glean.split("mod tests").next().expect("prod glean");
    assert!(
        !prod_glean.contains("always_glean"),
        "dead always_glean must stay removed"
    );
    assert!(
        prod_glean.contains("fail-open") || prod_glean.contains("keeping base extraction"),
        "gleaning must fail-open on LLM error"
    );
    assert!(
        prod_glean.contains("early stop") || prod_glean.contains("Early stop"),
        "gleaning must early-stop on empty iteration"
    );
}
