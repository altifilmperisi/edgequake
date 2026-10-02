//! Parallel and resilient chunk extraction.
//!
//! ## Implements
//! - **FEAT0019**: Chunk-level progress tracking with callbacks
//! - **FEAT0020**: Chunk-level resilience and error isolation

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;

use futures::stream::{self, StreamExt};
use tokio_util::sync::CancellationToken;

use crate::error::Result;
use crate::extractor::EntityExtractor;

use super::{ChunkProgressCallback, ChunkProgressUpdate, Pipeline};

impl Pipeline {
    /// Extract entities from chunks in parallel using a semaphore.
    pub(super) async fn extract_parallel(
        &self,
        chunks: &[crate::chunker::TextChunk],
        extractor: &Arc<dyn EntityExtractor>,
    ) -> Result<Vec<crate::extractor::ExtractionResult>> {
        // Delegate to extract_parallel_with_progress with no callback
        self.extract_parallel_with_progress(chunks, extractor, None)
            .await
    }

    /// Extract entities from chunks in parallel with optional progress callback.
    ///
    /// ## Implements
    /// - **FEAT0019**: Chunk-level progress tracking
    /// - **UC2304**: System reports per-chunk progress during extraction
    pub(super) async fn extract_parallel_with_progress(
        &self,
        chunks: &[crate::chunker::TextChunk],
        extractor: &Arc<dyn EntityExtractor>,
        progress_callback: Option<ChunkProgressCallback>,
    ) -> Result<Vec<crate::extractor::ExtractionResult>> {
        // SPEC-156: buffer_unordered alone bounds concurrency (no redundant Semaphore).
        let total_chunks = chunks.len();

        // Atomic counters for cumulative tracking across concurrent extractions
        let cumulative_time_ms = Arc::new(AtomicU64::new(0));
        let cumulative_input_tokens = Arc::new(AtomicU64::new(0));
        let cumulative_output_tokens = Arc::new(AtomicU64::new(0));
        let completed_chunks = Arc::new(AtomicU32::new(0));

        // Get model pricing for cost calculation
        let pricing = crate::progress::default_model_pricing();
        let model_name = extractor.model_name();
        let model_pricing = pricing
            .get(model_name)
            .cloned()
            .unwrap_or_else(|| crate::progress::ModelPricing::new("gpt-4.1-nano", 0.00015, 0.0006));
        let model_pricing = Arc::new(model_pricing);

        // SPEC-047 P3a: own chunk clones (O(C) data), then stream futures lazily so
        // only `max_concurrent` async state machines exist at once (Send-safe).
        // Do NOT stream over a borrowed chunk slice across await — that breaks
        // TaskProcessor: Send. Own the clones first, then stream::iter(owned).
        let owned: Vec<(usize, crate::chunker::TextChunk)> =
            chunks.iter().cloned().enumerate().collect();
        let results: Vec<Result<crate::extractor::ExtractionResult>> = stream::iter(owned)
            .map(|(chunk_index, chunk)| {
                let extractor = extractor.clone();
                let progress_callback = progress_callback.clone();
                let cumulative_time_ms = cumulative_time_ms.clone();
                let cumulative_input_tokens = cumulative_input_tokens.clone();
                let cumulative_output_tokens = cumulative_output_tokens.clone();
                let completed_chunks = completed_chunks.clone();
                let model_pricing = model_pricing.clone();

                async move {
                    let chunk_start = std::time::Instant::now();
                    // Extract entities from this chunk
                    let mut result = extractor.extract(&chunk).await?;

                    // SPEC-156: production extractors leave extraction_time_ms=0.
                    let time_ms = chunk_start.elapsed().as_millis() as u64;
                    result.extraction_time_ms = time_ms;
                    let in_tokens = result.input_tokens;
                    let out_tokens = result.output_tokens;

                    cumulative_time_ms.fetch_add(time_ms, Ordering::Relaxed);
                    cumulative_input_tokens.fetch_add(in_tokens as u64, Ordering::Relaxed);
                    cumulative_output_tokens.fetch_add(out_tokens as u64, Ordering::Relaxed);
                    let completed = completed_chunks.fetch_add(1, Ordering::Relaxed) + 1;

                    // Calculate cost for this chunk
                    let chunk_cost = model_pricing.calculate_cost(in_tokens, out_tokens);

                    // Emit progress update if callback is provided
                    if let Some(ref callback) = progress_callback {
                        let total_time = cumulative_time_ms.load(Ordering::Relaxed);
                        let total_in = cumulative_input_tokens.load(Ordering::Relaxed);
                        let total_out = cumulative_output_tokens.load(Ordering::Relaxed);

                        // Calculate average time per chunk and ETA
                        let avg_time_ms = if completed > 0 {
                            total_time as f64 / completed as f64
                        } else {
                            0.0
                        };
                        let remaining = total_chunks.saturating_sub(completed as usize);
                        let eta_seconds = ((avg_time_ms * remaining as f64) / 1000.0) as u64;

                        // Calculate cumulative cost
                        let cumulative_cost =
                            model_pricing.calculate_cost(total_in as usize, total_out as usize);

                        // Truncate chunk preview to 100 chars (OODA-02: Fixed UTF-8 char boundary panic)
                        let chunk_preview = if chunk.content.len() > 100 {
                            // Use char_indices() to ensure we don't split multi-byte UTF-8 characters
                            let truncate_at = chunk
                                .content
                                .char_indices()
                                .nth(97)
                                .map(|(idx, _)| idx)
                                .unwrap_or(chunk.content.len());
                            format!("{}...", &chunk.content[..truncate_at])
                        } else {
                            chunk.content.clone()
                        };

                        let update = ChunkProgressUpdate {
                            chunk_index,
                            total_chunks,
                            chunk_preview,
                            processing_time_ms: time_ms,
                            input_tokens: in_tokens,
                            output_tokens: out_tokens,
                            chunk_cost_usd: chunk_cost,
                            cumulative_input_tokens: total_in,
                            cumulative_output_tokens: total_out,
                            cumulative_cost_usd: cumulative_cost,
                            avg_time_per_chunk_ms: avg_time_ms,
                            eta_seconds,
                            phase: super::ChunkProgressPhase::Completed,
                            attempt: 1,
                            completed_chunks: chunk_index + 1,
                            gate_wait_ms: 0,
                        };

                        callback(update);
                    }

                    Ok(result)
                }
            })
            .buffer_unordered(self.config.max_concurrent_extractions)
            .collect()
            .await;

        // Collect results, propagating first error
        results.into_iter().collect()
    }

    // ═══════════════════════════════════════════════════════════════════════════
    //                    RESILIENT PARALLEL EXTRACTION (MAP-REDUCE)
    // ═══════════════════════════════════════════════════════════════════════════
    //
    // WHY RESILIENT EXTRACTION?
    // ────────────────────────────
    // The original extract_parallel_with_progress fails fast on the first error.
    // This is problematic for large documents where:
    // - A single chunk timeout shouldn't discard 99 successful extractions
    // - Users expect partial results with clear reporting of failures
    // - Retry logic should be at chunk level, not document level
    //
    // ARCHITECTURE (MAP-REDUCE PATTERN):
    //
    //   Document (N chunks)
    //        │
    //        ▼
    //   ┌────┬────┬────┬────┬────┐
    //   │ C1 │ C2 │ C3 │ C4 │ CN │   (parallel LLM calls with semaphore)
    //   └─┬──┴─┬──┴─┬──┴─┬──┴─┬──┘
    //     ▼    ▼    ▼    ▼    ▼
    //   ┌───┐┌───┐┌───┐┌───┐┌───┐
    //   │ ✓ ││ ✗ ││ ✓ ││ ✓ ││ ✓ │   (each with per-chunk retry + timeout)
    //   └───┘└───┘└───┘└───┘└───┘
    //     │                   │
    //     ▼                   ▼
    //   Successes: [C1,C3,C4,CN]   Failures: [C2]
    //
    // RETRY STRATEGY (PER CHUNK):
    //   Attempt 1 → Attempt 2 (2x delay) → Attempt 3 (4x delay) → Failed

    /// Extract entities from chunks with resilient error handling.
    ///
    /// Unlike `extract_parallel`, this method does NOT fail fast on errors.
    /// Instead, it processes all chunks and returns both successes and failures,
    /// allowing partial results to be used.
    ///
    /// ## Implements
    /// - **FEAT0020**: Chunk-level resilience and error isolation
    /// - **UC2305**: System continues processing when individual chunks fail
    #[tracing::instrument(
        name = "pipeline_chunk_extraction",
        skip(
            self,
            chunks,
            extractor,
            progress_callback,
            cancel_token,
            resume_by_chunk_id,
            reuse_index,
            on_chunk_extracted
        ),
        fields(
            chunk_count = chunks.len(),
            error.code = tracing::field::Empty,
            langfuse.observation.type = %edgequake_observability::OBSERVATION_TYPE_SPAN,
            langfuse.observation.input = tracing::field::Empty,
            langfuse.observation.output = tracing::field::Empty,
            gen_ai.prompt = tracing::field::Empty,
            gen_ai.completion = tracing::field::Empty,
        )
    )]
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn resilient_extract_parallel(
        &self,
        chunks: &[crate::chunker::TextChunk],
        extractor: &Arc<dyn EntityExtractor>,
        progress_callback: Option<ChunkProgressCallback>,
        cancel_token: Option<CancellationToken>,
        resume_by_chunk_id: Option<
            std::collections::HashMap<String, crate::extractor::ExtractionResult>,
        >,
        reuse_index: Option<crate::chunk_reuse::ChunkReuseIndex>,
        on_chunk_extracted: Option<super::ChunkExtractedCallback>,
    ) -> crate::error::ResilientExtractionResult {
        edgequake_observability::record_observation_type_span();
        use crate::error::{ChunkExtractionOutcome, ChunkFailure, ResilientExtractionResult};
        let resume_by_chunk_id = std::sync::Arc::new(resume_by_chunk_id.unwrap_or_default());
        let reuse_index = std::sync::Arc::new(reuse_index);
        let on_chunk_extracted = on_chunk_extracted.clone();

        // SPEC-156: buffer_unordered alone bounds concurrency (no redundant Semaphore).
        let total_chunks = chunks.len();
        edgequake_observability::record_pipeline_chunk_extraction_io(
            total_chunks,
            None,
            None,
            None,
        );
        let timeout_secs = self.config.chunk_extraction_timeout_secs;
        // C-18: treat max_retries=0 as one attempt (config name means retries,
        // but a zero loop would silently skip extraction).
        let max_retries = self.config.chunk_max_retries.max(1);
        let initial_delay_ms = self.config.initial_retry_delay_ms;

        // Atomic counters for cumulative tracking
        let cumulative_time_ms = Arc::new(AtomicU64::new(0));
        let cumulative_input_tokens = Arc::new(AtomicU64::new(0));
        let cumulative_output_tokens = Arc::new(AtomicU64::new(0));
        let completed_chunks = Arc::new(AtomicU32::new(0));

        // Get model pricing for cost calculation
        let pricing = crate::progress::default_model_pricing();
        let model_name = extractor.model_name();
        let model_pricing = pricing
            .get(model_name)
            .cloned()
            .unwrap_or_else(|| crate::progress::ModelPricing::new("gpt-4.1-nano", 0.00015, 0.0006));
        let model_pricing = Arc::new(model_pricing);

        // ═══════════════════════════════════════════════════════════════════════
        //                           MAP PHASE
        // ═══════════════════════════════════════════════════════════════════════

        // SPEC-047 P3a: own clones, then lazy futures (Send-safe; see extract_parallel_with_progress).
        let owned: Vec<(usize, crate::chunker::TextChunk)> =
            chunks.iter().cloned().enumerate().collect();
        let outcomes: Vec<ChunkExtractionOutcome> = stream::iter(owned)
            .map(|(chunk_index, chunk)| {
                let extractor = extractor.clone();
                let progress_callback = progress_callback.clone();
                let cumulative_time_ms = cumulative_time_ms.clone();
                let cumulative_input_tokens = cumulative_input_tokens.clone();
                let cumulative_output_tokens = cumulative_output_tokens.clone();
                let completed_chunks = completed_chunks.clone();
                let model_pricing = model_pricing.clone();
                let cancel_token = cancel_token.clone();
                let resume_by_chunk_id = resume_by_chunk_id.clone();
                let reuse_index = reuse_index.clone();
                let on_chunk_extracted = on_chunk_extracted.clone();

                async move {
                    let chunk_start = std::time::Instant::now();

                    // ── CANCELLATION CHECK: skip pending chunks when cancelled ──
                    if let Some(ref token) = cancel_token {
                        if token.is_cancelled() {
                            return ChunkExtractionOutcome::Failed(ChunkFailure {
                                chunk_index,
                                chunk_id: chunk.id.clone(),
                                error: "Task cancelled".to_string(),
                                retry_attempts: 0,
                                was_timeout: false,
                                processing_time_ms: 0,
                            });
                        }
                    }

                    // SPEC-151: page-scope gate — LLM only for dirty (overlapping) pages.
                    // When a reuse index is present, decide_chunk_extract ignores positional id resume.
                    let action = crate::chunk_reuse::decide_chunk_extract(
                        reuse_index.as_ref().as_ref(),
                        Some(&*resume_by_chunk_id),
                        &chunk,
                    );
                    match action {
                        crate::chunk_reuse::ChunkExtractAction::Reuse(prior) => {
                            let completed =
                                completed_chunks.fetch_add(1, Ordering::Relaxed) + 1;
                            if let Some(ref callback) = progress_callback {
                                callback(ChunkProgressUpdate {
                                    chunk_index,
                                    total_chunks,
                                    chunk_preview: "[resumed]".to_string(),
                                    processing_time_ms: 0,
                                    input_tokens: prior.input_tokens,
                                    output_tokens: prior.output_tokens,
                                    chunk_cost_usd: 0.0,
                                    cumulative_input_tokens: cumulative_input_tokens
                                        .load(Ordering::Relaxed),
                                    cumulative_output_tokens: cumulative_output_tokens
                                        .load(Ordering::Relaxed),
                                    cumulative_cost_usd: 0.0,
                                    avg_time_per_chunk_ms: 0.0,
                                    eta_seconds: 0,
                                    phase: super::ChunkProgressPhase::Completed,
                                    attempt: 0,
                                    completed_chunks: completed as usize,
                                    gate_wait_ms: 0,
                                });
                            }
                            return ChunkExtractionOutcome::Success {
                                chunk_index,
                                result: prior,
                            };
                        }
                        crate::chunk_reuse::ChunkExtractAction::FailClosed => {
                            tracing::error!(
                                chunk_id = %chunk.id,
                                page_start = ?chunk.page_start,
                                page_end = ?chunk.page_end,
                                "SPEC-151: clean-page content-hash miss — refuse extract (no LLM)"
                            );
                            return ChunkExtractionOutcome::Failed(ChunkFailure {
                                chunk_index,
                                chunk_id: chunk.id.clone(),
                                error: format!(
                                    "{}: clean page content changed; refuse empty extraction \
                                     to avoid entity loss on retract. Run a full document \
                                     reprocess, or re-select pages after chunk boundaries stabilize.",
                                    crate::chunk_reuse::SPEC151_CLEAN_HASH_MISS
                                ),
                                retry_attempts: 0,
                                was_timeout: false,
                                processing_time_ms: chunk_start.elapsed().as_millis() as u64,
                            });
                        }
                        crate::chunk_reuse::ChunkExtractAction::ExtractFresh => {
                            // Fall through to LLM retry loop.
                        }
                    }

                    // SPEC-156: concurrency bounded solely by buffer_unordered below.

                    // ─────────────────────────────────────────────────────────
                    // PER-CHUNK RETRY LOOP
                    // ─────────────────────────────────────────────────────────
                    let mut last_error = String::new();
                    let mut was_timeout = false;

                    for attempt in 1..=max_retries {
                        // ── CANCELLATION CHECK: abort retry loop when cancelled ──
                        if let Some(ref token) = cancel_token {
                            if token.is_cancelled() {
                                last_error = "Task cancelled".to_string();
                                break;
                            }
                        }

                        // Heartbeat: in-flight / retry so UI does not freeze at 0%.
                        if let Some(ref callback) = progress_callback {
                            let done = completed_chunks.load(Ordering::Relaxed) as usize;
                            let total_time = cumulative_time_ms.load(Ordering::Relaxed);
                            let avg_time_ms = if done > 0 {
                                total_time as f64 / done as f64
                            } else {
                                0.0
                            };
                            let remaining = total_chunks.saturating_sub(done);
                            let eta_seconds = if avg_time_ms > 0.0 {
                                ((avg_time_ms * remaining as f64) / 1000.0) as u64
                            } else {
                                0
                            };
                            let preview = if chunk.content.len() > 100 {
                                let truncate_at = chunk
                                    .content
                                    .char_indices()
                                    .nth(97)
                                    .map(|(idx, _)| idx)
                                    .unwrap_or(chunk.content.len());
                                format!("{}...", &chunk.content[..truncate_at])
                            } else {
                                chunk.content.clone()
                            };
                            let phase = if attempt == 1 {
                                super::ChunkProgressPhase::Started
                            } else {
                                super::ChunkProgressPhase::Retrying
                            };
                            callback(ChunkProgressUpdate {
                                chunk_index,
                                total_chunks,
                                chunk_preview: preview,
                                processing_time_ms: chunk_start.elapsed().as_millis() as u64,
                                input_tokens: 0,
                                output_tokens: 0,
                                chunk_cost_usd: 0.0,
                                cumulative_input_tokens: cumulative_input_tokens
                                    .load(Ordering::Relaxed),
                                cumulative_output_tokens: cumulative_output_tokens
                                    .load(Ordering::Relaxed),
                                cumulative_cost_usd: 0.0,
                                avg_time_per_chunk_ms: avg_time_ms,
                                eta_seconds,
                                phase,
                                attempt,
                                completed_chunks: done,
                                gate_wait_ms: 0,
                            });
                        }

                        let extraction_future = extractor.extract(&chunk);
                        let timeout_duration = tokio::time::Duration::from_secs(timeout_secs);

                        // Abort in-flight LLM extract when cancel fires (drops HTTP future).
                        let timed = if let Some(ref token) = cancel_token {
                            tokio::select! {
                                biased;
                                _ = token.cancelled() => None,
                                result = tokio::time::timeout(timeout_duration, extraction_future) => {
                                    Some(result)
                                }
                            }
                        } else {
                            Some(tokio::time::timeout(timeout_duration, extraction_future).await)
                        };

                        let Some(timed) = timed else {
                            last_error = "Task cancelled".to_string();
                            break;
                        };

                        match timed {
                            Ok(Ok(mut result)) => {
                                // SUCCESS PATH — SPEC-156: wall-clock when extractor omits time.
                                let time_ms = chunk_start.elapsed().as_millis() as u64;
                                result.extraction_time_ms = time_ms;
                                let in_tokens = result.input_tokens;
                                let out_tokens = result.output_tokens;

                                cumulative_time_ms.fetch_add(time_ms, Ordering::Relaxed);
                                cumulative_input_tokens
                                    .fetch_add(in_tokens as u64, Ordering::Relaxed);
                                cumulative_output_tokens
                                    .fetch_add(out_tokens as u64, Ordering::Relaxed);
                                let completed =
                                    completed_chunks.fetch_add(1, Ordering::Relaxed) + 1;

                                // Emit progress update if callback is provided
                                if let Some(ref callback) = progress_callback {
                                    let total_time = cumulative_time_ms.load(Ordering::Relaxed);
                                    let total_in = cumulative_input_tokens.load(Ordering::Relaxed);
                                    let total_out =
                                        cumulative_output_tokens.load(Ordering::Relaxed);

                                    let avg_time_ms = if completed > 0 {
                                        total_time as f64 / completed as f64
                                    } else {
                                        0.0
                                    };
                                    let remaining = total_chunks.saturating_sub(completed as usize);
                                    let eta_seconds =
                                        ((avg_time_ms * remaining as f64) / 1000.0) as u64;

                                    let cumulative_cost = model_pricing
                                        .calculate_cost(total_in as usize, total_out as usize);

                                    let chunk_preview = if chunk.content.len() > 100 {
                                        let truncate_at = chunk
                                            .content
                                            .char_indices()
                                            .nth(97)
                                            .map(|(idx, _)| idx)
                                            .unwrap_or(chunk.content.len());
                                        format!("{}...", &chunk.content[..truncate_at])
                                    } else {
                                        chunk.content.clone()
                                    };

                                    let chunk_cost =
                                        model_pricing.calculate_cost(in_tokens, out_tokens);

                                    callback(ChunkProgressUpdate {
                                        chunk_index,
                                        total_chunks,
                                        chunk_preview,
                                        processing_time_ms: time_ms,
                                        input_tokens: in_tokens,
                                        output_tokens: out_tokens,
                                        chunk_cost_usd: chunk_cost,
                                        cumulative_input_tokens: total_in,
                                        cumulative_output_tokens: total_out,
                                        cumulative_cost_usd: cumulative_cost,
                                        avg_time_per_chunk_ms: avg_time_ms,
                                        eta_seconds,
                                        phase: super::ChunkProgressPhase::Completed,
                                        attempt,
                                        completed_chunks: completed as usize,
                                        gate_wait_ms: 0,
                                    });
                                }

                                if let Some(ref sink) = on_chunk_extracted {
                                    sink(chunk.id.clone(), result.clone());
                                }
                                return ChunkExtractionOutcome::Success {
                                    chunk_index,
                                    result,
                                };
                            }
                            Ok(Err(e)) => {
                                // Extraction error (not timeout)
                                last_error = format!("{}", e);
                                was_timeout = false;
                                tracing::warn!(
                                    chunk_index = chunk_index,
                                    chunk_id = %chunk.id,
                                    attempt = attempt,
                                    max_retries = max_retries,
                                    error = %e,
                                    "Chunk extraction failed, will retry"
                                );
                            }
                            Err(_) => {
                                // Timeout
                                last_error = format!(
                                    "Timeout after {}s (attempt {}/{})",
                                    timeout_secs, attempt, max_retries
                                );
                                was_timeout = true;
                                tracing::warn!(
                                    chunk_index = chunk_index,
                                    chunk_id = %chunk.id,
                                    attempt = attempt,
                                    max_retries = max_retries,
                                    timeout_secs = timeout_secs,
                                    "Chunk extraction timed out, will retry"
                                );
                            }
                        }

                        let err_class = crate::pipeline::classify_extract_error(&last_error);
                        let budget = crate::pipeline::extract_retry_budget(err_class, max_retries);
                        let reason = match err_class {
                            crate::pipeline::ExtractErrorClass::Transient if was_timeout => {
                                "timeout"
                            }
                            crate::pipeline::ExtractErrorClass::Transient => "network",
                            crate::pipeline::ExtractErrorClass::Parse => "parse",
                            crate::pipeline::ExtractErrorClass::Permanent => "permanent",
                        };
                        edgequake_observability::metrics::record_extract_retry(reason);

                        // Permanent / exhausted parse budget → stop early.
                        if err_class == crate::pipeline::ExtractErrorClass::Permanent
                            || attempt >= budget
                        {
                            break;
                        }

                        // Exponential backoff before retry (longer for local overload)
                        let delay_ms = crate::pipeline::retry_delay_ms_for_chunk_error(
                            initial_delay_ms,
                            attempt,
                            &last_error,
                        );
                        if crate::pipeline::is_local_provider_overload_error(&last_error) {
                            tracing::warn!(
                                chunk_index = chunk_index,
                                delay_ms = delay_ms,
                                attempt = attempt,
                                "Local LLM overload detected — backing off before retry"
                            );
                        }
                        tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                    }

                    // FAILURE PATH (all retries exhausted)
                    completed_chunks.fetch_add(1, Ordering::Relaxed);

                    ChunkExtractionOutcome::Failed(ChunkFailure {
                        chunk_index,
                        chunk_id: chunk.id.clone(),
                        error: last_error,
                        retry_attempts: max_retries,
                        was_timeout,
                        processing_time_ms: chunk_start.elapsed().as_millis() as u64,
                    })
                }
            })
            .buffer_unordered(self.config.max_concurrent_extractions)
            .collect()
            .await;

        let result = ResilientExtractionResult::from_outcomes(outcomes);
        let entity_count: usize = result
            .successful_extractions
            .iter()
            .map(|e| e.entities.len())
            .sum();
        edgequake_observability::record_pipeline_chunk_extraction_io(
            total_chunks,
            Some(result.successful_extractions.len()),
            Some(result.failed_chunks.len()),
            Some(entity_count),
        );
        result
    }
}
