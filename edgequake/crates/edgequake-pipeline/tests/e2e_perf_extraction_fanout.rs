//! Ingestion performance — extraction fan-out is provider-scoped and effective.
//!
//! First principle: the concurrency limit for a document is a property of the
//! LLM that serves *that document*, not of the process default provider. These
//! tests prove (a) fan-out turns into wall-clock speedup when the upstream is
//! latency-bound, and (b) a cloud workspace keeps full fan-out even when the
//! process default provider is a local one.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use edgequake_pipeline::{
    chunker::{ChunkerConfig, TextChunk},
    extractor::{EntityExtractor, ExtractedEntity, ExtractionResult},
    Pipeline, PipelineConfig, DEFAULT_MAX_CONCURRENT_EXTRACTIONS, LOCAL_MAX_CONCURRENT_EXTRACTIONS,
};

/// Extractor with injected latency that records peak in-flight calls.
struct LatencyExtractor {
    delay: Duration,
    in_flight: AtomicUsize,
    peak: AtomicUsize,
    calls: AtomicUsize,
}

impl LatencyExtractor {
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
impl EntityExtractor for LatencyExtractor {
    async fn extract(
        &self,
        chunk: &TextChunk,
    ) -> edgequake_pipeline::error::Result<ExtractionResult> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let now = self.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
        self.peak.fetch_max(now, Ordering::SeqCst);
        tokio::time::sleep(self.delay).await;
        self.in_flight.fetch_sub(1, Ordering::SeqCst);

        let mut result = ExtractionResult::new(&chunk.id);
        result.add_entity(ExtractedEntity::new(
            format!("ENTITY_{}", chunk.index),
            "CONCEPT",
            "latency test entity",
        ));
        Ok(result)
    }

    fn name(&self) -> &str {
        "latency-extractor"
    }
}

fn many_chunk_document(paragraphs: usize) -> String {
    (0..paragraphs)
        .map(|i| {
            format!(
                "Paragraph {i}. The river valley {i} meets the mountain peak {i} where the \
                 ocean tide {i} reaches the forest meadow {i} below the canyon plateau {i}, \
                 and travellers record every landmark in their journals."
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn config_with(max_concurrent: usize) -> PipelineConfig {
    PipelineConfig {
        enable_entity_extraction: true,
        enable_relationship_extraction: false,
        enable_chunk_embeddings: false,
        enable_entity_embeddings: false,
        enable_relationship_embeddings: false,
        max_concurrent_extractions: max_concurrent,
        chunker: ChunkerConfig {
            chunk_size: 40,
            chunk_overlap: 0,
            min_chunk_size: 5,
            ..ChunkerConfig::default()
        },
        ..Default::default()
    }
}

/// Run one document and return (wall-clock, chunk count, peak in-flight).
async fn run(max_concurrent: usize) -> (Duration, usize, usize) {
    let extractor = Arc::new(LatencyExtractor::new(40));
    let pipeline = Pipeline::new(config_with(max_concurrent)).with_extractor(extractor.clone());
    let doc = many_chunk_document(24);

    let start = Instant::now();
    let result = pipeline
        .process_with_resilience("perf-fanout", &doc, None)
        .await
        .expect("pipeline run");
    let elapsed = start.elapsed();

    assert_eq!(result.stats.failed_chunks, 0);
    assert_eq!(
        extractor.calls.load(Ordering::SeqCst),
        result.chunks.len(),
        "every chunk is extracted exactly once"
    );
    // SPEC-156: wall-clock extraction_time_ms (production extractors leave 0).
    for ext in &result.extractions {
        assert!(
            ext.extraction_time_ms > 0,
            "extraction_time_ms must be stamped from wall clock (got 0 for {})",
            ext.source_chunk_id
        );
    }
    (
        elapsed,
        result.chunks.len(),
        extractor.peak.load(Ordering::SeqCst),
    )
}

#[tokio::test]
async fn e2e_extraction_fanout_turns_into_wall_clock_speedup() {
    let (serial, chunks, serial_peak) = run(1).await;
    let (parallel, _, parallel_peak) = run(8).await;

    assert!(chunks >= 12, "need enough chunks to measure, got {chunks}");
    assert_eq!(serial_peak, 1, "fan-out 1 must stay strictly serial");
    assert!(
        parallel_peak > 1 && parallel_peak <= 8,
        "fan-out 8 must overlap calls but never exceed the bound (peak={parallel_peak})"
    );
    assert!(
        parallel * 3 < serial,
        "8-way extraction must be >3x faster: serial={serial:?} parallel={parallel:?} chunks={chunks}"
    );
}

/// The regression behind slow cloud ingestion: the process default provider is
/// local (Ollama) but the workspace uses a cloud LLM. The per-document config
/// must follow the workspace provider.
#[test]
fn contract_workspace_provider_wins_over_process_default() {
    // Single test in this binary touches these env vars (no cross-test races).
    std::env::set_var("EDGEQUAKE_DEFAULT_LLM_PROVIDER", "ollama");
    std::env::remove_var("EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS");
    std::env::remove_var("EDGEQUAKE_ALLOW_LOCAL_HIGH_CONCURRENCY");

    let cloud = PipelineConfig::from_env_for_provider("mistral");
    assert_eq!(
        cloud.max_concurrent_extractions, DEFAULT_MAX_CONCURRENT_EXTRACTIONS,
        "cloud workspace must keep full fan-out under a local process default"
    );

    let local = PipelineConfig::from_env_for_provider("ollama");
    assert_eq!(
        local.max_concurrent_extractions,
        LOCAL_MAX_CONCURRENT_EXTRACTIONS
    );

    // Even an operator pin of 32 must not storm a local server.
    std::env::set_var("EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS", "32");
    let local_pinned = PipelineConfig::from_env_for_provider("lmstudio");
    assert_eq!(
        local_pinned.max_concurrent_extractions,
        LOCAL_MAX_CONCURRENT_EXTRACTIONS
    );
    std::env::remove_var("EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS");
    std::env::remove_var("EDGEQUAKE_DEFAULT_LLM_PROVIDER");
}
