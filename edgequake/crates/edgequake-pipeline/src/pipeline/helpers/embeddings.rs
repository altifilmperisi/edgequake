//! Embedding generation helpers and token-budget batching.

use std::sync::Arc;

use futures::stream::{self, StreamExt};

use crate::chunker::TextChunk;
use crate::error::Result;
use crate::extractor::ExtractionResult;

use super::super::{
    CostBreakdownStats, EmbedProgressCallback, EmbedProgressUpdate, Pipeline, ProcessingStats,
};
use super::unique_embed::{unique_entities_for_embed, unique_relationships_for_embed};

// ─────────────────────────────────────────────────────────────────────────────
//                       EMBEDDING GENERATION HELPERS
// ─────────────────────────────────────────────────────────────────────────────

/// Conservative chars-per-true-token for dense technical content.
///
/// WHY 2.5: The chunker uses 4 chars/token (English prose).
/// Scientific PDFs contain tables with numbers, gene IDs, p-values, and
/// formulas where tokenizers split aggressively — real density can reach
/// 1.5–2.0 chars/token. Using 2.5 provides a safe intermediate buffer.
const EMBED_CHARS_PER_TOKEN: f64 = 2.5;

/// Safety headroom factor applied to the embedding context limit.
///
/// WHY 0.85: Leaves 15% slack for tokenizer variance, whitespace tokens,
/// and any prompt overhead the embedding endpoint may add.
const EMBED_SAFETY_FACTOR: f64 = 0.85;

/// Fallback maximum characters when `provider.max_tokens()` returns 0 (unknown).
///
/// 6 000 chars ≈ 2 400 tokens at 2.5 chars/token, keeping chunks well within
/// the 2 048-token limit of models like embeddinggemma.
const EMBED_FALLBACK_MAX_CHARS: usize = 6_000;

/// Compute the maximum safe character count for a single embedding input.
///
/// When the provider exposes its context limit, we derive the char cap from it.
/// When the limit is unknown (0), we fall back to `EMBED_FALLBACK_MAX_CHARS`.
fn embed_max_chars(max_tokens: usize) -> usize {
    if max_tokens == 0 {
        EMBED_FALLBACK_MAX_CHARS
    } else {
        (max_tokens as f64 * EMBED_CHARS_PER_TOKEN * EMBED_SAFETY_FACTOR) as usize
    }
}

/// Truncate `s` to at most `max_bytes`, preserving UTF-8 character boundaries.
#[inline]
fn truncate_at_char_boundary(s: &str, max_bytes: usize) -> &str {
    // Local name kept for embed-guard call sites / unit tests; SSOT is utf8_prefix.
    edgequake_observability::utf8_prefix(s, max_bytes)
}

/// Policy when an embedding input exceeds the provider-safe character cap (OPS-P1.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EmbeddingTruncationPolicy {
    /// Truncate and continue (historical default — partial embedding > hard fail).
    #[default]
    Truncate,
    /// Fail the embed batch so operators fix chunk_size / model.
    Fail,
}

/// Parse `EDGEQUAKE_EMBED_TRUNCATE_POLICY` (pure — pass raw for non-flaky tests).
///
/// Default: truncate. `fail` / `error` / `strict` → Fail.
pub fn parse_embedding_truncation_policy(raw: &str) -> EmbeddingTruncationPolicy {
    match raw.trim().to_ascii_lowercase().as_str() {
        "fail" | "error" | "strict" | "abort" => EmbeddingTruncationPolicy::Fail,
        _ => EmbeddingTruncationPolicy::Truncate,
    }
}

fn embedding_truncation_policy_from_env() -> EmbeddingTruncationPolicy {
    parse_embedding_truncation_policy(
        &std::env::var("EDGEQUAKE_EMBED_TRUNCATE_POLICY").unwrap_or_default(),
    )
}

/// Outcome of guarding a text batch for embedding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddingGuardResult {
    pub texts: Vec<String>,
    /// True when at least one input was truncated.
    pub truncated: bool,
}

/// Guard a text batch before sending to the embedding provider.
///
/// Truncates (or fails) any string that exceeds `max_chars`.
fn guard_for_embedding(
    texts: &[String],
    max_chars: usize,
    policy: EmbeddingTruncationPolicy,
) -> crate::error::Result<EmbeddingGuardResult> {
    let mut out = Vec::with_capacity(texts.len());
    let mut truncated = false;
    for (i, text) in texts.iter().enumerate() {
        if text.len() > max_chars {
            truncated = true;
            match policy {
                EmbeddingTruncationPolicy::Fail => {
                    return Err(crate::error::PipelineError::EmbeddingError(format!(
                        "Embedding input[{i}] exceeds safe limit ({max_chars} chars, got {}); \
                         set EDGEQUAKE_EMBED_TRUNCATE_POLICY=truncate to allow truncation, \
                         or reduce chunk_size / use a larger embedding model",
                        text.len()
                    )));
                }
                EmbeddingTruncationPolicy::Truncate => {
                    tracing::warn!(
                        input_index = i,
                        original_chars = text.len(),
                        cap_chars = max_chars,
                        "Embedding input truncated: text exceeds the safe token limit for the \
                         embedding model. Consider reducing chunk_size in PipelineConfig or \
                         switching to an embedding model with a larger context window."
                    );
                    out.push(truncate_at_char_boundary(text, max_chars).to_string());
                }
            }
        } else {
            out.push(text.clone());
        }
    }
    Ok(EmbeddingGuardResult {
        texts: out,
        truncated,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
//                   TOKEN-AWARE EMBEDDING BATCH HELPER
// ─────────────────────────────────────────────────────────────────────────────

/// Embed `texts` with automatic token-aware sub-batching.
///
/// ## Problem
///
/// `EmbeddingProvider::embed_batched` splits inputs only by **count** (default
/// 2 048 texts per API call). This is insufficient for providers like Mistral
/// whose API enforces an **8 192-token TOTAL budget per request** regardless of
/// the number of individual texts. 142 entity descriptions from a dense
/// technical PDF can easily exceed 8 192 total tokens, producing:
///
/// ```text
/// 400 Bad Request: "Too many tokens overall, split into more batches." (code 3210)
/// ```
///
/// ## Fix
///
/// Before delegating to `embed_batched`, accumulate texts into sub-batches
/// whose estimated total token count stays within
/// `provider.max_tokens() * EMBED_SAFETY_FACTOR`. When the budget would be
/// exceeded by the next text, flush the current sub-batch first.
///
/// Token estimation uses `EMBED_CHARS_PER_TOKEN = 2.5` (conservative for
/// dense technical content) — the same constant as `guard_for_embedding`.
///
/// When `provider.max_tokens()` returns 0 (limit unknown) we fall back to
/// `embed_batched` directly because there is no budget to split against.
/// Embed a batch with exponential backoff on transient provider limits (SPEC-045 EC-045-09).
async fn embed_batched_with_retry(
    provider: &Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
    batch: &[String],
    cancel: Option<&tokio_util::sync::CancellationToken>,
) -> crate::error::Result<Vec<Vec<f32>>> {
    const MAX_ATTEMPTS: u32 = 3;
    let mut attempt = 0u32;
    loop {
        if cancel.map(|t| t.is_cancelled()).unwrap_or(false) {
            return Err(crate::error::PipelineError::EmbeddingError(
                "Task cancelled".to_string(),
            ));
        }

        let result = if let Some(token) = cancel {
            tokio::select! {
                biased;
                _ = token.cancelled() => {
                    return Err(crate::error::PipelineError::EmbeddingError(
                        "Task cancelled".to_string(),
                    ));
                }
                result = provider.embed_batched(batch) => result,
            }
        } else {
            provider.embed_batched(batch).await
        };

        match result {
            Ok(embeddings) => return Ok(embeddings),
            Err(e) => {
                let msg = e.to_string();
                attempt += 1;
                // X-06/X-07: typed retry_strategy only — no substring "429" matching.
                if e.retry_strategy().should_retry() && attempt < MAX_ATTEMPTS {
                    let base_ms = 500u64.saturating_mul(1u64 << (attempt - 1).min(4));
                    // Full jitter (AWS): uniform in [0, base] prevents thundering herd.
                    let delay_ms = {
                        use std::collections::hash_map::DefaultHasher;
                        use std::hash::{Hash, Hasher};
                        let mut h = DefaultHasher::new();
                        (attempt, batch.len(), base_ms).hash(&mut h);
                        h.finish() % (base_ms.max(1))
                    };
                    tracing::warn!(
                        attempt,
                        delay_ms,
                        error = %msg,
                        batch_size = batch.len(),
                        "Transient embedding provider error — retrying with jittered backoff"
                    );
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                    continue;
                }
                return Err(crate::error::PipelineError::EmbeddingError(msg));
            }
        }
    }
}

/// Local-provider ceiling for parallel embed sub-batches (unless opt-out).
///
/// X-07: transient classification is solely `LlmError::retry_strategy()`.
pub const LOCAL_EMBED_MAX_ASYNC: usize = 1;

/// Max concurrent embedding API sub-batches for `provider_name`
/// (LightRAG `embedding_func_max_async` ≈ 8).
///
/// Override with `EDGEQUAKE_EMBED_MAX_ASYNC` (clamped 1..=32). The local-provider
/// cap ([`LOCAL_EMBED_MAX_ASYNC`]) is decided by the **embedding provider that is
/// actually called** — never by the process-default LLM provider — so a cloud
/// embedder keeps full fan-out even when the default LLM is Ollama.
pub fn embed_max_async_for(provider_name: &str) -> usize {
    let requested =
        parse_embed_max_async(&std::env::var("EDGEQUAKE_EMBED_MAX_ASYNC").unwrap_or_default());
    apply_local_embed_async_clamp(requested, provider_name)
}

/// Pure parser for `EDGEQUAKE_EMBED_MAX_ASYNC` (testable without env mutation).
pub fn parse_embed_max_async(raw: &str) -> usize {
    raw.trim()
        .parse::<usize>()
        .ok()
        .filter(|&n| n > 0)
        .map(|n| n.min(32))
        .unwrap_or(8)
}

/// Cap embed fan-out for capacity-bound local providers (SSOT helper).
///
/// Returns the effective concurrency (may be lower than `requested`).
pub fn apply_local_embed_async_clamp(requested: usize, provider_name: &str) -> usize {
    let bounded = requested.clamp(1, 32);
    let effective =
        crate::pipeline::cap_for_local_provider(provider_name, bounded, LOCAL_EMBED_MAX_ASYNC);
    if effective < bounded {
        tracing::info!(
            provider = provider_name,
            requested = bounded,
            effective,
            "Local embed concurrency clamped (set EDGEQUAKE_ALLOW_LOCAL_HIGH_CONCURRENCY=1 to override)"
        );
    }
    effective
}

/// Plan token/count-aware sub-batches as `(start_index, end_index)` half-open ranges.
///
/// Pure function — shared by sequential and parallel embed paths (DRY).
pub(crate) fn plan_embed_sub_batches(
    texts: &[String],
    max_tokens: usize,
    max_batch_count: usize,
) -> Vec<(usize, usize)> {
    if texts.is_empty() {
        return Vec::new();
    }
    if max_tokens == 0 {
        return vec![(0, texts.len())];
    }

    let token_budget = (max_tokens as f64 * EMBED_SAFETY_FACTOR) as usize;
    let max_batch_count = max_batch_count.max(1);
    let mut ranges = Vec::new();
    let mut batch_start = 0usize;
    let mut batch_tokens = 0usize;

    for (i, text) in texts.iter().enumerate() {
        let text_tokens = ((text.len() as f64) / EMBED_CHARS_PER_TOKEN).ceil() as usize;
        let current_count = i - batch_start;
        let token_overflow = batch_tokens + text_tokens > token_budget;
        let count_overflow = current_count >= max_batch_count;
        if (token_overflow || count_overflow) && i > batch_start {
            ranges.push((batch_start, i));
            batch_start = i;
            batch_tokens = 0;
        }
        batch_tokens += text_tokens;
    }
    if batch_start < texts.len() {
        ranges.push((batch_start, texts.len()));
    }
    ranges
}

async fn embed_with_token_budget(
    provider: &Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
    texts: &[String],
    progress: Option<(&EmbedProgressCallback, &'static str)>,
    cancel: Option<&tokio_util::sync::CancellationToken>,
) -> crate::error::Result<Vec<Vec<f32>>> {
    let max_async = embed_max_async_for(provider.name());
    embed_with_token_budget_at(provider, texts, max_async, None, progress, cancel).await
}

/// Token-budgeted embedding at an explicit fan-out (`max_async` sub-batches).
///
/// Split from [`embed_with_token_budget`] so the concurrency decision (policy)
/// is separate from the fan-out mechanism and testable with injected latency.
///
/// `slots`, when set, is a shared semaphore across concurrent embed stages
/// (SPEC-156 chunk/entity/relationship join) so total in-flight HTTP calls
/// stay ≤ `embed_max_async`.
async fn embed_with_token_budget_at(
    provider: &Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
    texts: &[String],
    max_async: usize,
    slots: Option<&Arc<tokio::sync::Semaphore>>,
    progress: Option<(&EmbedProgressCallback, &'static str)>,
    cancel: Option<&tokio_util::sync::CancellationToken>,
) -> crate::error::Result<Vec<Vec<f32>>> {
    if texts.is_empty() {
        return Ok(Vec::new());
    }

    let emit = |current: usize| {
        if let Some((cb, stage)) = progress {
            cb(EmbedProgressUpdate {
                stage,
                current,
                total: texts.len(),
            });
        }
    };

    let ranges = plan_embed_sub_batches(texts, provider.max_tokens(), provider.max_batch_size());
    let concurrency = max_async.max(1).min(ranges.len().max(1));

    // Single sub-batch: no fan-out overhead.
    if ranges.len() <= 1 {
        let _permit = if let Some(sem) = slots {
            Some(sem.acquire().await.map_err(|_| {
                crate::error::PipelineError::EmbeddingError("embed semaphore closed".into())
            })?)
        } else {
            None
        };
        let batch_result = embed_batched_with_retry(provider, texts, cancel).await?;
        emit(batch_result.len());
        return Ok(batch_result);
    }

    // Parallel sub-batches (LightRAG asyncio.gather + embedding_func_max_async).
    // Preserve order by tagging each result with its start index.
    tracing::debug!(
        sub_batches = ranges.len(),
        concurrency,
        texts = texts.len(),
        "Embedding sub-batches in parallel"
    );

    let provider = Arc::clone(provider);
    let texts_owned: Vec<String> = texts.to_vec();
    let cancel_owned = cancel.cloned();
    // SPEC-156: optional shared permit pool across chunk/entity/relationship joins.
    let shared_slots = slots.cloned();

    // X-18: per-sub-batch Result — do not fail-fast the whole collect on one error.
    // Preserve range identity on Err so we can retry / skip individually.
    type SubBatchOk = (usize, Vec<Vec<f32>>);
    type SubBatchErr = (usize, usize, crate::error::PipelineError);
    let results: Vec<std::result::Result<SubBatchOk, SubBatchErr>> = stream::iter(ranges)
        .map(|(start, end)| {
            let provider = Arc::clone(&provider);
            let batch = texts_owned[start..end].to_vec();
            let cancel = cancel_owned.clone();
            let slots = shared_slots.clone();
            async move {
                let _permit = if let Some(sem) = slots.as_ref() {
                    match sem.acquire().await {
                        Ok(p) => Some(p),
                        Err(_) => {
                            return Err((
                                start,
                                end,
                                crate::error::PipelineError::EmbeddingError(
                                    "embed semaphore closed".into(),
                                ),
                            ));
                        }
                    }
                } else {
                    None
                };
                match embed_batched_with_retry(&provider, &batch, cancel.as_ref()).await {
                    Ok(emb) => Ok((start, emb)),
                    Err(e) => Err((start, end, e)),
                }
            }
        })
        .buffer_unordered(concurrency)
        .collect()
        .await;

    let mut indexed: Vec<(usize, Vec<Vec<f32>>)> = Vec::with_capacity(results.len());
    let mut failed_ranges: Vec<(usize, usize, crate::error::PipelineError)> = Vec::new();
    let mut last_error_msg: Option<String> = None;
    for r in results {
        match r {
            Ok(v) => indexed.push(v),
            Err((start, end, e)) => {
                last_error_msg = Some(e.to_string());
                failed_ranges.push((start, end, e));
            }
        }
    }

    // One sequential retry pass for failed sub-batches (transient blips).
    for (start, end, first_err) in failed_ranges {
        if cancel_owned
            .as_ref()
            .map(|t| t.is_cancelled())
            .unwrap_or(false)
        {
            return Err(crate::error::PipelineError::EmbeddingError(
                "Task cancelled".to_string(),
            ));
        }
        let batch = &texts_owned[start..end];
        match embed_batched_with_retry(&provider, batch, cancel_owned.as_ref()).await {
            Ok(emb) => {
                tracing::info!(
                    start,
                    end,
                    recovered = emb.len(),
                    "X-18: embed sub-batch recovered on retry"
                );
                indexed.push((start, emb));
            }
            Err(e) => {
                last_error_msg = Some(e.to_string());
                tracing::warn!(
                    start,
                    end,
                    first_error = %first_err,
                    retry_error = %e,
                    "X-18: embed sub-batch failed after retry — tolerating partial batch"
                );
            }
        }
    }

    if indexed.is_empty() && !texts.is_empty() {
        let detail = last_error_msg.unwrap_or_else(|| "unknown".to_string());
        return Err(crate::error::PipelineError::EmbeddingError(format!(
            "All embedding sub-batches failed: {detail}"
        )));
    }

    indexed.sort_by_key(|(start, _)| *start);
    let mut all_embeddings: Vec<Vec<f32>> = Vec::with_capacity(texts.len());
    let mut cursor = 0usize;
    for (start, emb) in indexed {
        if start > cursor {
            tracing::warn!(
                skipped_from = cursor,
                skipped_to = start,
                "X-18: gap in embed sub-batches after partial failure"
            );
        }
        cursor = start + emb.len();
        let n = all_embeddings.len() + emb.len();
        all_embeddings.extend(emb);
        emit(n.min(texts.len()));
    }

    if all_embeddings.len() != texts.len() {
        return Err(crate::error::PipelineError::EmbeddingError(format!(
            "Embedding count mismatch: expected {}, got {} (partial sub-batch failure)",
            texts.len(),
            all_embeddings.len()
        )));
    }

    Ok(all_embeddings)
}

// ─────────────────────────────────────────────────────────────────────────────
//                       SAFE EMBEDDING HELPER
// ─────────────────────────────────────────────────────────────────────────────

/// Guard `texts`, embed with token-budget batching, and validate result count.
///
/// Encapsulates the three-step pattern repeated for every embeddable item kind
/// (chunks, entities, relationships):
/// 1. `guard_for_embedding` — truncate inputs that exceed the provider's
///    character limit to avoid 400 "input too long" errors.
/// 2. `embed_with_token_budget` — split into sub-batches respecting BOTH the
///    token budget and the provider's input-count limit.
/// 3. Count mismatch warning — if the provider silently drops embeddings,
///    log a warning so that operators notice orphaned graph nodes.
///
/// Returns the embeddings in the same order as `texts`, or an empty `Vec` when
/// `texts` is empty (short-circuit avoids a provider round-trip).
async fn safe_embed(
    provider: &Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
    texts: &[String],
    max_chars: usize,
    kind: &str,
    slots: Option<&Arc<tokio::sync::Semaphore>>,
    progress: Option<(&EmbedProgressCallback, &'static str)>,
    cancel: Option<&tokio_util::sync::CancellationToken>,
) -> crate::error::Result<Vec<Vec<f32>>> {
    if texts.is_empty() {
        return Ok(Vec::new());
    }
    if cancel.map(|t| t.is_cancelled()).unwrap_or(false) {
        return Err(crate::error::PipelineError::EmbeddingError(
            "Task cancelled".to_string(),
        ));
    }
    let guarded = guard_for_embedding(texts, max_chars, embedding_truncation_policy_from_env())?;
    if guarded.truncated {
        tracing::info!(
            kind,
            truncated = true,
            "SPEC-046 OPS-P1.7: embedding inputs truncated under Truncate policy"
        );
    }
    let mut embeddings = edgequake_observability::with_rag_embedding_span(
        "embed-chunks",
        provider.model(),
        provider.name(),
        async {
            let emb = match slots {
                Some(sem) => {
                    let max_async = embed_max_async_for(provider.name());
                    embed_with_token_budget_at(
                        provider,
                        &guarded.texts,
                        max_async,
                        Some(sem),
                        progress,
                        cancel,
                    )
                    .await?
                }
                None => embed_with_token_budget(provider, &guarded.texts, progress, cancel).await?,
            };
            let dim = emb.first().map(|v| v.len());
            edgequake_observability::record_embedding_io(kind, guarded.texts.len(), emb.len(), dim);
            Ok::<_, crate::error::PipelineError>(emb)
        },
    )
    .await?;
    if embeddings.len() != texts.len() {
        return Err(crate::error::PipelineError::EmbeddingError(format!(
            "{kind} embedding count mismatch: expected {}, got {}",
            texts.len(),
            embeddings.len()
        )));
    }
    // X-10: L2-normalize on write (same algorithm as Embedding::normalize).
    for vector in &mut embeddings {
        l2_normalize_inplace(vector);
    }
    Ok(embeddings)
}

/// L2-normalize a vector in place (SSOT mirror of `Embedding::normalize`).
fn l2_normalize_inplace(vector: &mut [f32]) {
    let norm: f32 = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        vector.iter_mut().for_each(|x| *x /= norm);
    }
}

/// Estimate the number of embedding tokens for a character count.
///
/// Uses `EMBED_CHARS_PER_TOKEN` — the same conservative constant as the
/// batch-splitting logic — so that cost estimates and batch boundaries share a
/// single denominator (DRY). Previously the cost loops used a hardcoded `/ 4`
/// (4 chars/token) which diverged from the `2.5` used in sub-batch sizing.
fn estimate_embed_tokens(char_count: usize) -> usize {
    (char_count as f64 / EMBED_CHARS_PER_TOKEN).ceil() as usize
}

impl Pipeline {
    fn emit_embed_progress(
        progress: Option<&EmbedProgressCallback>,
        stage: &'static str,
        current: usize,
        total: usize,
    ) {
        if let Some(cb) = progress {
            cb(EmbedProgressUpdate {
                stage,
                current,
                total,
            });
        }
    }

    /// Generate embeddings for chunks, entities, and relationships.
    ///
    /// WHY UNIFIED: All three processing methods shared identical embedding
    /// logic (~120 lines each). This single implementation handles:
    /// - Chunk embeddings (content → vector)
    /// - Entity embeddings (name: description → vector)
    /// - Relationship embeddings (keywords + source→target + description → vector)
    /// - Embedding cost calculation
    ///
    /// `progress` is invoked **at the start and end of each sub-stage** so
    /// callers can surface real-time progress while embeddings are generated.
    /// Pass `None` when no progress reporting is needed.
    pub(in crate::pipeline) async fn generate_all_embeddings(
        &self,
        chunks: &mut [TextChunk],
        extractions: &mut [ExtractionResult],
        stats: &mut ProcessingStats,
        progress: Option<&EmbedProgressCallback>,
        cancel: Option<&tokio_util::sync::CancellationToken>,
    ) -> Result<()> {
        let provider = match &self.embedding_provider {
            Some(p) => p,
            None => return Ok(()),
        };

        // Capture embedding model and provider info
        // @implements SPEC-032/OODA-226: Provider tracking in ProcessingStats
        stats.embedding_model = Some(provider.model().to_string());
        stats.embedding_provider = Some(provider.name().to_string());
        stats.embedding_dimensions = Some(provider.dimension());

        let max_chars = embed_max_chars(provider.max_tokens());
        let max_async = embed_max_async_for(provider.name());
        // SPEC-156: one shared budget across chunk/entity/relationship embeds.
        let slots = Arc::new(tokio::sync::Semaphore::new(max_async.max(1)));

        let do_chunks = self.config.enable_chunk_embeddings;
        let do_entities = self.config.enable_entity_embeddings;
        let do_relationships = self.config.enable_relationship_embeddings;

        let chunk_texts: Option<Vec<String>> = if do_chunks {
            Some(chunks.iter().map(|c| c.content.clone()).collect())
        } else {
            None
        };
        // Unique-before-embed once; reuse for cost estimate (SPEC-047 P6 / SPEC-156).
        let unique_ents = if do_entities {
            Some(unique_entities_for_embed(extractions))
        } else {
            None
        };
        let unique_rels = if do_relationships {
            Some(unique_relationships_for_embed(extractions))
        } else {
            None
        };
        let entity_texts: Option<Vec<String>> = unique_ents
            .as_ref()
            .map(|u| u.iter().map(|e| e.text.clone()).collect());
        let relationship_texts: Option<Vec<String>> = unique_rels
            .as_ref()
            .map(|u| u.iter().map(|e| e.text.clone()).collect());

        if let Some(ref texts) = chunk_texts {
            Self::emit_embed_progress(progress, "chunks", 0, texts.len());
        }
        if let Some(ref u) = unique_ents {
            tracing::info!(
                unique_entities = u.len(),
                mention_entities = u.iter().map(|e| e.mentions.len()).sum::<usize>(),
                "Entity embed: unique-before-embed (SPEC-047 P6)"
            );
            Self::emit_embed_progress(progress, "entities", 0, u.len());
        }
        if let Some(ref u) = unique_rels {
            tracing::info!(
                unique_relationships = u.len(),
                mention_relationships = u.iter().map(|e| e.mentions.len()).sum::<usize>(),
                "Relationship embed: unique-before-embed (SPEC-047 P6)"
            );
            Self::emit_embed_progress(progress, "relationships", 0, u.len());
        }

        let provider = Arc::clone(provider);
        let cancel_owned = cancel.cloned();
        let slots_c = Arc::clone(&slots);
        let slots_e = Arc::clone(&slots);
        let slots_r = Arc::clone(&slots);
        let progress_c = progress.cloned();
        let progress_e = progress.cloned();
        let progress_r = progress.cloned();

        // Box::pin each arm so try_join! holds three pointers, not three full
        // embed FSMs (same pattern as SPEC-047 hybrid/mix — debug-build stack
        // overflow on large document ingest).
        let (chunk_embeddings, entity_embeddings, relationship_embeddings) = tokio::try_join!(
            Box::pin(async {
                match chunk_texts.as_ref() {
                    Some(texts) => {
                        let emb = safe_embed(
                            &provider,
                            texts,
                            max_chars,
                            "Chunk",
                            Some(&slots_c),
                            progress_c.as_ref().map(|cb| (cb, "chunks")),
                            cancel_owned.as_ref(),
                        )
                        .await?;
                        Ok::<_, crate::error::PipelineError>(Some(emb))
                    }
                    None => Ok(None),
                }
            }),
            Box::pin(async {
                match entity_texts.as_ref() {
                    Some(texts) => {
                        let emb = safe_embed(
                            &provider,
                            texts,
                            max_chars,
                            "Entity",
                            Some(&slots_e),
                            progress_e.as_ref().map(|cb| (cb, "entities")),
                            cancel_owned.as_ref(),
                        )
                        .await?;
                        Ok(Some(emb))
                    }
                    None => Ok(None),
                }
            }),
            Box::pin(async {
                match relationship_texts.as_ref() {
                    Some(texts) => {
                        let emb = safe_embed(
                            &provider,
                            texts,
                            max_chars,
                            "Relationship",
                            Some(&slots_r),
                            progress_r.as_ref().map(|cb| (cb, "relationships")),
                            cancel_owned.as_ref(),
                        )
                        .await?;
                        Ok(Some(emb))
                    }
                    None => Ok(None),
                }
            }),
        )?;

        if let (Some(texts), Some(embeddings)) = (chunk_texts.as_ref(), chunk_embeddings) {
            for (chunk, embedding) in chunks.iter_mut().zip(embeddings) {
                chunk.embedding = Some(embedding);
            }
            Self::emit_embed_progress(progress, "chunks", texts.len(), texts.len());
        }
        if let (Some(unique), Some(embeddings)) = (unique_ents.as_ref(), entity_embeddings) {
            for (embedding, entry) in embeddings.into_iter().zip(unique.iter()) {
                if let Some(&(ext_idx, ent_idx)) = entry.mentions.first() {
                    extractions[ext_idx].entities[ent_idx].embedding = Some(embedding);
                }
            }
            Self::emit_embed_progress(progress, "entities", unique.len(), unique.len());
        }
        if let (Some(unique), Some(embeddings)) = (unique_rels.as_ref(), relationship_embeddings) {
            for (embedding, entry) in embeddings.into_iter().zip(unique.iter()) {
                if let Some(&(ext_idx, rel_idx)) = entry.mentions.first() {
                    extractions[ext_idx].relationships[rel_idx].embedding = Some(embedding);
                }
            }
            Self::emit_embed_progress(progress, "relationships", unique.len(), unique.len());
        }

        // Embedding cost from the same unique texts we sent (DRY / SPEC-156).
        let mut total_embed_tokens = 0usize;
        if let Some(ref texts) = chunk_texts {
            let chunk_text_len: usize = texts.iter().map(|t| t.len()).sum();
            total_embed_tokens += estimate_embed_tokens(chunk_text_len);
        }
        if let Some(ref texts) = entity_texts {
            for t in texts {
                total_embed_tokens += estimate_embed_tokens(t.len());
            }
        }
        if let Some(ref texts) = relationship_texts {
            for t in texts {
                total_embed_tokens += estimate_embed_tokens(t.len());
            }
        }

        let embed_model_name = provider.model();
        let pricing = crate::progress::default_model_pricing();
        let embed_pricing = pricing.get(embed_model_name).cloned().unwrap_or_else(|| {
            crate::progress::ModelPricing::new("text-embedding-3-small", 0.00002, 0.0)
        });

        let embedding_cost = embed_pricing.calculate_cost(total_embed_tokens, 0);
        stats.cost_usd += embedding_cost;

        if let Some(ref mut breakdown) = stats.cost_breakdown {
            breakdown.embedding_cost_usd = embedding_cost;
            breakdown.embedding_tokens = total_embed_tokens;
        } else {
            let breakdown = CostBreakdownStats {
                embedding_cost_usd: embedding_cost,
                embedding_tokens: total_embed_tokens,
                ..CostBreakdownStats::default()
            };
            stats.cost_breakdown = Some(breakdown);
        }

        Ok(())
    }

    /// Re-generate embeddings for a `ProcessingResult` (slim-checkpoint resume).
    ///
    /// WHY (SPEC-047 P5): Checkpoints omit embeddings to stay under Postgres
    /// jsonb size limits. On resume we skip LLM extraction but must re-embed
    /// before merge/persist.
    pub async fn ensure_embeddings(
        &self,
        result: &mut crate::pipeline::ProcessingResult,
        progress: Option<&EmbedProgressCallback>,
    ) -> Result<()> {
        self.generate_all_embeddings(
            &mut result.chunks,
            &mut result.extractions,
            &mut result.stats,
            progress,
            None,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── embed_max_chars ────────────────────────────────────────────────────

    #[test]
    fn test_embed_max_chars_with_known_limit() {
        // 8192 tokens × 2.5 chars/token × 0.85 safety = 17 408 chars
        let expected = (8192_f64 * EMBED_CHARS_PER_TOKEN * EMBED_SAFETY_FACTOR) as usize;
        assert_eq!(embed_max_chars(8192), expected);
    }

    #[test]
    fn test_embed_max_chars_fallback_when_zero() {
        assert_eq!(embed_max_chars(0), EMBED_FALLBACK_MAX_CHARS);
    }

    // ── truncate_at_char_boundary ─────────────────────────────────────────

    #[test]
    fn test_truncate_exact_boundary() {
        let s = "hello world";
        assert_eq!(truncate_at_char_boundary(s, 5), "hello");
    }

    #[test]
    fn test_truncate_within_multibyte_char() {
        // "é" is 2 bytes (U+00E9). Truncating at byte 1 must walk back to byte 0.
        let s = "aéb";
        let truncated = truncate_at_char_boundary(s, 2);
        assert!(s.is_char_boundary(truncated.len()));
        assert_eq!(truncated, "a");
    }

    #[test]
    fn test_truncate_no_op_when_within_limit() {
        let s = "short";
        assert_eq!(truncate_at_char_boundary(s, 100), "short");
    }

    // ── guard_for_embedding ──────────────────────────────────────────────

    #[test]
    fn test_guard_preserves_short_texts() {
        let texts = vec!["hello".to_string(), "world".to_string()];
        let result = guard_for_embedding(&texts, 100, EmbeddingTruncationPolicy::Truncate).unwrap();
        assert_eq!(result.texts, texts);
        assert!(!result.truncated);
    }

    #[test]
    fn test_guard_truncates_long_texts() {
        let long_text = "a".repeat(200);
        let result = guard_for_embedding(
            std::slice::from_ref(&long_text),
            50,
            EmbeddingTruncationPolicy::Truncate,
        )
        .unwrap();
        assert_eq!(result.texts.len(), 1);
        assert!(result.texts[0].len() <= 50);
        assert!(result.truncated);
    }

    #[test]
    fn test_guard_fail_policy_rejects_long_texts() {
        let long_text = "a".repeat(200);
        let err = guard_for_embedding(
            std::slice::from_ref(&long_text),
            50,
            EmbeddingTruncationPolicy::Fail,
        )
        .unwrap_err();
        assert!(err.to_string().contains("exceeds safe limit"));
    }

    #[test]
    fn test_parse_embedding_truncation_policy() {
        assert_eq!(
            parse_embedding_truncation_policy(""),
            EmbeddingTruncationPolicy::Truncate
        );
        assert_eq!(
            parse_embedding_truncation_policy("fail"),
            EmbeddingTruncationPolicy::Fail
        );
        assert_eq!(
            parse_embedding_truncation_policy("STRICT"),
            EmbeddingTruncationPolicy::Fail
        );
    }

    // ── embed_with_token_budget ────────────────────────────────────────────

    /// Counts how many times `embed()` is called by accumulating batch sizes.
    use std::sync::{Arc, Mutex};

    struct CountingEmbedProvider {
        /// Each element records the number of texts in that sub-batch call.
        call_sizes: Arc<Mutex<Vec<usize>>>,
        /// Simulated max_tokens limit.
        max_tokens: usize,
        /// Simulated max_batch_size (input count limit).
        max_batch: usize,
    }

    impl CountingEmbedProvider {
        fn new(max_tokens: usize) -> (Self, Arc<Mutex<Vec<usize>>>) {
            Self::new_with_batch(max_tokens, 2048)
        }

        fn new_with_batch(max_tokens: usize, max_batch: usize) -> (Self, Arc<Mutex<Vec<usize>>>) {
            let call_sizes = Arc::new(Mutex::new(Vec::new()));
            (
                Self {
                    call_sizes: call_sizes.clone(),
                    max_tokens,
                    max_batch,
                },
                call_sizes,
            )
        }
    }

    #[async_trait::async_trait]
    impl edgequake_llm::traits::EmbeddingProvider for CountingEmbedProvider {
        fn name(&self) -> &str {
            "counting"
        }
        fn model(&self) -> &str {
            "counting-embed"
        }
        fn dimension(&self) -> usize {
            4
        }
        fn max_tokens(&self) -> usize {
            self.max_tokens
        }
        fn max_batch_size(&self) -> usize {
            self.max_batch
        }

        async fn embed(&self, texts: &[String]) -> edgequake_llm::Result<Vec<Vec<f32>>> {
            self.call_sizes.lock().unwrap().push(texts.len());
            // Return a dummy 4-dim vector per text
            Ok(texts.iter().map(|_| vec![0.1, 0.2, 0.3, 0.4]).collect())
        }
    }

    struct FailingEmbedProvider {
        max_tokens: usize,
        max_batch: usize,
    }

    #[async_trait::async_trait]
    impl edgequake_llm::traits::EmbeddingProvider for FailingEmbedProvider {
        fn name(&self) -> &str {
            "failing"
        }
        fn model(&self) -> &str {
            "failing-embed"
        }
        fn dimension(&self) -> usize {
            4
        }
        fn max_tokens(&self) -> usize {
            self.max_tokens
        }
        fn max_batch_size(&self) -> usize {
            self.max_batch
        }

        async fn embed(&self, _texts: &[String]) -> edgequake_llm::Result<Vec<Vec<f32>>> {
            Err(edgequake_llm::error::LlmError::NetworkError(
                "error sending request for url (http://localhost:11434/api/embeddings)".into(),
            ))
        }
    }

    /// Parallel sub-batch path must surface the underlying provider error (not a generic shell).
    #[tokio::test]
    async fn test_embed_parallel_path_propagates_last_provider_error() {
        let texts: Vec<String> = (0..6).map(|_| "x".repeat(100)).collect();
        let provider: Arc<dyn edgequake_llm::traits::EmbeddingProvider> =
            Arc::new(FailingEmbedProvider {
                max_tokens: 80,
                max_batch: 2,
            });

        let err = embed_with_token_budget(&provider, &texts, None, None)
            .await
            .expect_err("all sub-batches should fail");
        let msg = err.to_string();
        assert!(
            msg.contains("All embedding sub-batches failed"),
            "expected aggregate prefix, got: {msg}"
        );
        assert!(
            msg.contains("error sending request"),
            "underlying provider error must be preserved, got: {msg}"
        );
    }

    /// When total estimated tokens fit within the budget, exactly ONE call is made.
    #[tokio::test]
    async fn test_embed_budget_single_batch_when_within_limit() {
        // 10 texts × 10 chars / 2.5 chars/token = 40 tokens, budget = 8192 * 0.85 ≈ 6963
        let texts: Vec<String> = (0..10).map(|i| format!("entity_{:04}", i)).collect(); // ~13 chars each
        let (provider, call_sizes) = CountingEmbedProvider::new(8192);
        let provider: Arc<dyn edgequake_llm::traits::EmbeddingProvider> = Arc::new(provider);

        let result = embed_with_token_budget(&provider, &texts, None, None)
            .await
            .unwrap();
        assert_eq!(result.len(), 10, "All 10 embeddings must be returned");
        let sizes = call_sizes.lock().unwrap();
        assert_eq!(sizes.len(), 1, "Should have made exactly 1 embed call");
    }

    /// When texts are large enough to exceed a tiny budget, they are split across
    /// multiple sub-batch calls and all embeddings are reassembled in order.
    #[tokio::test]
    async fn test_embed_budget_splits_batches_correctly() {
        // 20 texts of 100 chars each:
        //   100 / 2.5 = 40 tokens per text
        //   budget = 80 * 0.85 = 68 tokens ≈ 1 text per batch
        let texts: Vec<String> = (0..20).map(|_| "x".repeat(100)).collect();
        let (provider, call_sizes) = CountingEmbedProvider::new(80); // tiny budget forces splits
        let provider: Arc<dyn edgequake_llm::traits::EmbeddingProvider> = Arc::new(provider);

        let result = embed_with_token_budget(&provider, &texts, None, None)
            .await
            .unwrap();
        assert_eq!(result.len(), 20, "All 20 embeddings must be returned");
        // With max_tokens=80 and SAFETY_FACTOR=0.85, budget = 68 tokens.
        // Each text costs ceil(100/2.5)=40 tokens. Two texts = 80 > 68 → at least 2 calls.
        let sizes = call_sizes.lock().unwrap();
        assert!(
            sizes.len() >= 2,
            "Expected multiple batches, got {} call(s)",
            sizes.len()
        );
        // Total texts across all calls must equal 20 (no duplicates, no drops)
        let total: usize = sizes.iter().sum();
        assert_eq!(total, 20, "Total texts across batches must be 20");
    }

    /// Empty input returns empty output without calling the provider at all.
    #[tokio::test]
    async fn test_embed_budget_empty_input() {
        let (provider, call_sizes) = CountingEmbedProvider::new(8192);
        let provider: Arc<dyn edgequake_llm::traits::EmbeddingProvider> = Arc::new(provider);

        let result = embed_with_token_budget(&provider, &[], None, None)
            .await
            .unwrap();
        assert!(result.is_empty());
        assert!(
            call_sizes.lock().unwrap().is_empty(),
            "No calls for empty input"
        );
    }

    /// When `max_tokens == 0` (limit unknown), fall back to `embed_batched` — one call.
    #[tokio::test]
    async fn test_embed_budget_zero_max_tokens_fallback() {
        let texts: Vec<String> = (0..5).map(|i| format!("text_{}", i)).collect();
        let (provider, call_sizes) = CountingEmbedProvider::new(0); // 0 = unknown limit
        let provider: Arc<dyn edgequake_llm::traits::EmbeddingProvider> = Arc::new(provider);

        let result = embed_with_token_budget(&provider, &texts, None, None)
            .await
            .unwrap();
        assert_eq!(result.len(), 5);
        let sizes = call_sizes.lock().unwrap();
        assert_eq!(
            sizes.len(),
            1,
            "Should fall back to a single embed_batched call"
        );
    }

    // ── count-limit splitting (spec-011) ──────────────────────────────────

    /// When input count exceeds max_batch_size, splits into multiple calls.
    ///
    /// This is the regression test for the EU AI Act failure:
    /// many short entities fit within the token budget but exceed the
    /// Mistral input count limit (512).
    #[tokio::test]
    async fn test_embed_count_limit_splits_batches() {
        // 600 short texts (each 10 chars, ~4 tokens) with max_batch=512
        // Token budget = 8192 * 0.85 = 6963, 600 × 4 = 2400 tokens → fits budget
        // But 600 > 512 → must split
        let texts: Vec<String> = (0..600_usize).map(|i| format!("entity_{:04}", i)).collect();
        let (provider, call_sizes) = CountingEmbedProvider::new_with_batch(8192, 512);
        let provider: Arc<dyn edgequake_llm::traits::EmbeddingProvider> = Arc::new(provider);

        let result = embed_with_token_budget(&provider, &texts, None, None)
            .await
            .unwrap();
        assert_eq!(result.len(), 600, "All 600 embeddings returned");

        let sizes = call_sizes.lock().unwrap();
        assert!(
            sizes.len() >= 2,
            "600 texts with max_batch=512 must produce ≥ 2 calls, got {}",
            sizes.len()
        );
        // Each individual call must respect the count limit
        for (idx, &size) in sizes.iter().enumerate() {
            assert!(
                size <= 512,
                "Call {} sent {} items, exceeds max_batch_size 512",
                idx,
                size
            );
        }
        // No items lost or duplicated
        let total: usize = sizes.iter().sum();
        assert_eq!(total, 600, "Total items across all calls must be 600");
    }

    /// Exactly max_batch_size items → exactly ONE call (boundary: not exceeded).
    #[tokio::test]
    async fn test_embed_count_exactly_at_limit_is_one_call() {
        let texts: Vec<String> = (0..512_usize).map(|i| format!("e_{}", i)).collect();
        let (provider, call_sizes) = CountingEmbedProvider::new_with_batch(8192, 512);
        let provider: Arc<dyn edgequake_llm::traits::EmbeddingProvider> = Arc::new(provider);

        let result = embed_with_token_budget(&provider, &texts, None, None)
            .await
            .unwrap();
        assert_eq!(result.len(), 512);

        let sizes = call_sizes.lock().unwrap();
        assert_eq!(sizes.len(), 1, "Exactly 512 texts (== limit) → 1 call");
        assert_eq!(sizes[0], 512);
    }

    /// One more than max_batch_size → exactly TWO calls.
    #[tokio::test]
    async fn test_embed_count_one_over_limit_is_two_calls() {
        let texts: Vec<String> = (0..513_usize).map(|i| format!("e_{}", i)).collect();
        let (provider, call_sizes) = CountingEmbedProvider::new_with_batch(8192, 512);
        let provider: Arc<dyn edgequake_llm::traits::EmbeddingProvider> = Arc::new(provider);

        let result = embed_with_token_budget(&provider, &texts, None, None)
            .await
            .unwrap();
        assert_eq!(result.len(), 513);

        let sizes = call_sizes.lock().unwrap();
        assert_eq!(sizes.len(), 2, "513 texts with limit 512 → 2 calls");
        assert_eq!(sizes[0], 512, "First call: 512");
        assert_eq!(sizes[1], 1, "Second call: 1 remainder");
    }

    /// Both limits active simultaneously: token budget AND count limit both trigger.
    /// Verifies the flush uses the more restrictive limit.
    #[tokio::test]
    async fn test_embed_dual_limit_count_wins_over_token() {
        // 20 texts × 100 chars each = 40 tokens each
        // Token budget = 8192 * 0.85 = 6963 → 20×40=800 tokens fits budget
        // max_batch_count = 5 → must split on count, not tokens
        let texts: Vec<String> = (0..20).map(|_| "x".repeat(100)).collect();
        let (provider, call_sizes) = CountingEmbedProvider::new_with_batch(8192, 5);
        let provider: Arc<dyn edgequake_llm::traits::EmbeddingProvider> = Arc::new(provider);

        let result = embed_with_token_budget(&provider, &texts, None, None)
            .await
            .unwrap();
        assert_eq!(result.len(), 20);

        let sizes = call_sizes.lock().unwrap();
        assert!(
            sizes.len() >= 4,
            "20 texts with max_batch=5 → ≥ 4 calls, got {}",
            sizes.len()
        );
        for (idx, &size) in sizes.iter().enumerate() {
            assert!(
                size <= 5,
                "Call {} sent {} items, exceeds max_batch 5",
                idx,
                size
            );
        }
        let total: usize = sizes.iter().sum();
        assert_eq!(total, 20);
    }

    /// Token budget wins over count when texts are very large.
    #[tokio::test]
    async fn test_embed_dual_limit_token_wins_over_count() {
        // 20 texts × 1000 chars each = 400 tokens each
        // max_batch_count = 2048 (large) → count won't trigger
        // Token budget = 100 * 0.85 = 85 tokens → one 400-token text already exceeds budget
        // → splits on token budget (1 text per call since each > budget)
        let texts: Vec<String> = (0..5).map(|_| "y".repeat(1000)).collect();
        let (provider, call_sizes) = CountingEmbedProvider::new_with_batch(100, 2048);
        let provider: Arc<dyn edgequake_llm::traits::EmbeddingProvider> = Arc::new(provider);

        let result = embed_with_token_budget(&provider, &texts, None, None)
            .await
            .unwrap();
        assert_eq!(result.len(), 5);

        // Each text = ceil(1000/2.5) = 400 tokens; budget = 100*0.85 = 85 tokens
        // First text: 400 > 85 but i == batch_start (can't flush empty batch) → sent alone
        // Second text: 400 > 85 → flush first → each text in its own call
        let sizes = call_sizes.lock().unwrap();
        assert_eq!(
            sizes.len(),
            5,
            "Each text should be its own call due to tiny budget"
        );
        let total: usize = sizes.iter().sum();
        assert_eq!(total, 5);
    }

    #[test]
    fn spec047_p6_parse_embed_max_async_defaults_and_clamps() {
        assert_eq!(parse_embed_max_async(""), 8);
        assert_eq!(parse_embed_max_async("4"), 4);
        assert_eq!(parse_embed_max_async("0"), 8);
        assert_eq!(parse_embed_max_async("99"), 32);
        assert_eq!(parse_embed_max_async("nope"), 8);
    }

    #[test]
    fn local_embed_async_clamp_caps_ollama() {
        assert_eq!(
            apply_local_embed_async_clamp(8, "ollama"),
            LOCAL_EMBED_MAX_ASYNC
        );
        assert_eq!(apply_local_embed_async_clamp(8, "openai"), 8);
        assert_eq!(apply_local_embed_async_clamp(1, "lmstudio"), 1);
    }

    /// Provider with injected per-call latency; each vector encodes the input's
    /// numeric suffix so ordering can be verified after parallel fan-out.
    struct LatencyEmbedProvider {
        provider_name: &'static str,
        delay: std::time::Duration,
    }

    #[async_trait::async_trait]
    impl edgequake_llm::traits::EmbeddingProvider for LatencyEmbedProvider {
        fn name(&self) -> &str {
            self.provider_name
        }
        fn model(&self) -> &str {
            "latency-embed"
        }
        fn dimension(&self) -> usize {
            2
        }
        fn max_tokens(&self) -> usize {
            100_000
        }
        fn max_batch_size(&self) -> usize {
            1 // one text per call → N sub-batches
        }
        async fn embed(&self, texts: &[String]) -> edgequake_llm::Result<Vec<Vec<f32>>> {
            tokio::time::sleep(self.delay).await;
            Ok(texts
                .iter()
                .map(|t| {
                    let n: f32 = t.trim_start_matches('t').parse().unwrap_or(-1.0);
                    vec![n, 1.0]
                })
                .collect())
        }
    }

    fn latency_provider(
        name: &'static str,
        delay_ms: u64,
    ) -> Arc<dyn edgequake_llm::traits::EmbeddingProvider> {
        Arc::new(LatencyEmbedProvider {
            provider_name: name,
            delay: std::time::Duration::from_millis(delay_ms),
        })
    }

    fn numbered_texts(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("t{i}")).collect()
    }

    /// Fan-out mechanism: 8-way is markedly faster than serial and keeps order.
    #[tokio::test]
    async fn embed_fanout_is_faster_than_serial_and_preserves_order() {
        let provider = latency_provider("mistral", 40);
        let texts = numbered_texts(8);

        let t0 = std::time::Instant::now();
        let serial = embed_with_token_budget_at(&provider, &texts, 1, None, None, None)
            .await
            .unwrap();
        let serial_elapsed = t0.elapsed();

        let t1 = std::time::Instant::now();
        let parallel = embed_with_token_budget_at(&provider, &texts, 8, None, None, None)
            .await
            .unwrap();
        let parallel_elapsed = t1.elapsed();

        assert_eq!(serial, parallel, "parallel fan-out must preserve order");
        for (i, v) in parallel.iter().enumerate() {
            assert_eq!(v[0], i as f32);
        }
        assert!(
            parallel_elapsed * 3 < serial_elapsed,
            "expected >3x speedup: serial={serial_elapsed:?} parallel={parallel_elapsed:?}"
        );
    }

    /// SPEC-156: shared semaphore across concurrent embed stages caps peak in-flight.
    #[tokio::test]
    async fn shared_embed_slots_cap_peak_across_joined_stages() {
        use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

        struct PeakEmbedProvider {
            peak: Arc<AtomicUsize>,
            in_flight: Arc<AtomicUsize>,
            delay: std::time::Duration,
        }

        #[async_trait::async_trait]
        impl edgequake_llm::traits::EmbeddingProvider for PeakEmbedProvider {
            fn name(&self) -> &str {
                "mistral"
            }
            fn model(&self) -> &str {
                "peak-embed"
            }
            fn dimension(&self) -> usize {
                2
            }
            fn max_tokens(&self) -> usize {
                100_000
            }
            fn max_batch_size(&self) -> usize {
                1
            }
            async fn embed(&self, texts: &[String]) -> edgequake_llm::Result<Vec<Vec<f32>>> {
                let cur = self.in_flight.fetch_add(1, AtomicOrdering::SeqCst) + 1;
                self.peak.fetch_max(cur, AtomicOrdering::SeqCst);
                tokio::time::sleep(self.delay).await;
                self.in_flight.fetch_sub(1, AtomicOrdering::SeqCst);
                Ok(texts.iter().map(|_| vec![0.0, 1.0]).collect())
            }
        }

        let peak = Arc::new(AtomicUsize::new(0));
        let provider: Arc<dyn edgequake_llm::traits::EmbeddingProvider> =
            Arc::new(PeakEmbedProvider {
                peak: Arc::clone(&peak),
                in_flight: Arc::new(AtomicUsize::new(0)),
                delay: std::time::Duration::from_millis(30),
            });
        let slots = Arc::new(tokio::sync::Semaphore::new(3));
        let a = numbered_texts(4);
        let b = numbered_texts(4);
        let c = numbered_texts(4);
        let slots_a = Arc::clone(&slots);
        let slots_b = Arc::clone(&slots);
        let slots_c = Arc::clone(&slots);
        let p_a = Arc::clone(&provider);
        let p_b = Arc::clone(&provider);
        let p_c = Arc::clone(&provider);

        let t0 = std::time::Instant::now();
        let (ra, rb, rc) = tokio::try_join!(
            embed_with_token_budget_at(&p_a, &a, 8, Some(&slots_a), None, None),
            embed_with_token_budget_at(&p_b, &b, 8, Some(&slots_b), None, None),
            embed_with_token_budget_at(&p_c, &c, 8, Some(&slots_c), None, None),
        )
        .unwrap();
        let elapsed = t0.elapsed();

        assert_eq!(ra.len() + rb.len() + rc.len(), 12);
        let observed_peak = peak.load(AtomicOrdering::SeqCst);
        assert!(
            observed_peak > 1 && observed_peak <= 3,
            "peak in-flight must be in (1, 3], got {observed_peak}"
        );
        // 12 serial × 30ms = 360ms; with 3 slots ≈ 120ms (+slack).
        assert!(
            elapsed < std::time::Duration::from_millis(280),
            "joined stages under shared slots should overlap: {elapsed:?}"
        );
    }

    /// Policy: concurrency follows the embedding provider that is called,
    /// not the process-default LLM provider (the ingestion-slowness root cause).
    #[tokio::test]
    async fn embed_policy_cloud_provider_fans_out_local_provider_stays_serial() {
        let texts = numbered_texts(6);
        let delay = 40u64;
        let serial_floor = std::time::Duration::from_millis(delay * 6 * 9 / 10);

        let cloud = latency_provider("mistral", delay);
        let t0 = std::time::Instant::now();
        embed_with_token_budget(&cloud, &texts, None, None)
            .await
            .unwrap();
        let cloud_elapsed = t0.elapsed();
        assert!(
            cloud_elapsed * 2 < serial_floor,
            "cloud embedder must fan out: {cloud_elapsed:?} vs serial {serial_floor:?}"
        );

        let local = latency_provider("ollama", delay);
        let t1 = std::time::Instant::now();
        embed_with_token_budget(&local, &texts, None, None)
            .await
            .unwrap();
        assert!(
            t1.elapsed() >= serial_floor,
            "local embedder must stay serial, took {:?}",
            t1.elapsed()
        );
    }

    #[test]
    fn embed_max_async_for_is_provider_scoped() {
        // Pure clamp: same request, different upstream → different effective fan-out.
        assert_eq!(apply_local_embed_async_clamp(8, "mistral"), 8);
        assert_eq!(apply_local_embed_async_clamp(8, "openai"), 8);
        assert_eq!(apply_local_embed_async_clamp(8, "ollama"), 1);
        assert_eq!(apply_local_embed_async_clamp(8, "lm-studio"), 1);
    }

    #[test]
    fn spec047_p6_plan_embed_sub_batches_respects_count_limit() {
        let texts: Vec<String> = (0..20).map(|i| format!("t{i}")).collect();
        let ranges = plan_embed_sub_batches(&texts, 100_000, 5);
        assert!(ranges.len() >= 4);
        for (start, end) in &ranges {
            assert!(end - start <= 5);
        }
        assert_eq!(ranges.last().unwrap().1, 20);
    }

    /// Parallel path must preserve order across concurrent sub-batches.
    #[tokio::test]
    async fn spec047_p6_parallel_embed_preserves_order() {
        let texts: Vec<String> = (0..20).map(|i| format!("item-{i}")).collect();
        let (provider, call_sizes) = CountingEmbedProvider::new_with_batch(8_192, 5);
        let provider: Arc<dyn edgequake_llm::traits::EmbeddingProvider> = Arc::new(provider);
        // Force parallel path via env-independent concurrency (ranges > 1).
        let result = embed_with_token_budget(&provider, &texts, None, None)
            .await
            .unwrap();
        assert_eq!(result.len(), 20);
        // CountingEmbedProvider returns deterministic dims; order = input order.
        for (i, emb) in result.iter().enumerate() {
            assert!(!emb.is_empty(), "missing embedding at {i}");
        }
        let sizes = call_sizes.lock().unwrap();
        assert!(
            sizes.len() >= 4,
            "expected multiple sub-batches, got {}",
            sizes.len()
        );
    }

    #[test]
    fn x07_typed_retry_strategy_no_substring() {
        // X-07: classification is LlmError::retry_strategy(), not message contains.
        use edgequake_llm::LlmError;
        assert!(LlmError::RateLimited("too many requests".into())
            .retry_strategy()
            .should_retry());
        assert!(LlmError::Timeout.retry_strategy().should_retry());
        assert!(
            !LlmError::InvalidRequest("Too many inputs in request (400)".into())
                .retry_strategy()
                .should_retry()
        );
        assert!(!LlmError::AuthError("bad key".into())
            .retry_strategy()
            .should_retry());
    }

    /// Provider that permanently fails batches containing a poison marker.
    struct PartialFailEmbedProvider {
        max_batch: usize,
        poison: String,
    }

    #[async_trait::async_trait]
    impl edgequake_llm::traits::EmbeddingProvider for PartialFailEmbedProvider {
        fn name(&self) -> &str {
            "partial-fail"
        }
        fn model(&self) -> &str {
            "partial-fail-embed"
        }
        fn dimension(&self) -> usize {
            2
        }
        fn max_tokens(&self) -> usize {
            8_192
        }
        fn max_batch_size(&self) -> usize {
            self.max_batch
        }

        async fn embed(&self, texts: &[String]) -> edgequake_llm::Result<Vec<Vec<f32>>> {
            if texts.iter().any(|t| t.contains(&self.poison)) {
                return Err(edgequake_llm::LlmError::InvalidRequest(
                    "poison sub-batch".into(),
                ));
            }
            Ok(texts.iter().map(|_| vec![0.5, 0.5]).collect())
        }
    }

    /// SPEC-156: partial sub-batch failure after retry must hard-fail (count mismatch).
    /// Sub-batches still collect individually (no fail-fast Result gather).
    #[tokio::test]
    async fn unit_embed_partial_subbatch_hard_fails_on_count_mismatch() {
        // 12 texts, max_batch=5 → ≥3 sub-batches; poison only the middle range.
        let mut texts: Vec<String> = (0..12).map(|i| format!("ok-{i}")).collect();
        texts[5] = "POISON-item".into();
        texts[6] = "POISON-item-2".into();
        let provider: Arc<dyn edgequake_llm::traits::EmbeddingProvider> =
            Arc::new(PartialFailEmbedProvider {
                max_batch: 5,
                poison: "POISON".into(),
            });

        let err = embed_with_token_budget(&provider, &texts, None, None)
            .await
            .expect_err("SPEC-156: count mismatch after partial failure must Err");
        let msg = err.to_string();
        assert!(
            msg.contains("mismatch") || msg.contains("partial"),
            "unexpected error: {msg}"
        );

        // Contract: production path must not use fail-fast Result collect.
        let src = include_str!("embeddings.rs");
        let prod = src.split("#[cfg(test)]").next().expect("prod");
        assert!(
            !prod.contains("collect::<crate::error::Result<Vec<_>>>"),
            "must not fail-fast collect Result of all sub-batches"
        );
        assert!(
            prod.contains("Embedding count mismatch"),
            "SPEC-156 hard-fail path must be present"
        );
    }
}
