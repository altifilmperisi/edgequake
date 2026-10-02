# SPEC-156 — Ingestion First Principles

> **Status:** Study complete; quick wins shipping with this pack  
> **Product pin:** EdgeQuake v0.28.5  
> **Scope:** Parallelism, frugality (quality / performance / reliability ratio),
> adaptive rasterization honesty, DRY/SOLID cleanup of the ingest path.  
> **Inherits:** [SPEC-010](../010-ingestion-reliability/) ·
> [SPEC-038](../038-ingestion-large-pdf/) · [SPEC-047](../047-*/) ·
> [SPEC-090](../090-performance/) · [SPEC-103](../103-llm-cache/) ·
> [SPEC-126](../126-provider-kv-cache/) · [SPEC-134](../134-manuscrit/) ·
> [SPEC-135](../135-*/) · [SPEC-151](../151-*/) · [SPEC-153](../153-workload-benchmark/)  
> **Peers:** [SPEC-155](../155-improve-ux-ui/) (run-progress writer pattern)

## Start here

1. [01-parallelism-and-budget.md](01-parallelism-and-budget.md)
2. [02-frugality-and-reliability.md](02-frugality-and-reliability.md)
3. [03-rasterization.md](03-rasterization.md)
4. [04-roadmap.md](04-roadmap.md)

## First principles (LAW-156)

| ID | Law |
|----|-----|
| LAW-156-1 | Do only work with **marginal information**. |
| LAW-156-2 | Parallelize what is independent; bound with **one shared budget**. |
| LAW-156-3 | Side effects are **idempotent and single-writer**. |
| LAW-156-4 | Retry **once**, at the cheapest layer, with the provider's signal. |
| LAW-156-5 | Spend quality where **entropy** is (dense / handwritten pages). |
| LAW-156-6 | Knobs that do not change the render are **documented as cosmetic** or removed. |
| LAW-156-7 | Files ≤ 500 lines; one responsibility per module (SRP). |

Cost of a document = LLM tokens + embedding tokens + vision tokens.  
Latency = critical path through dependent stages.

## Stage-by-stage (as-is)

| Stage | Parallelism today | Default / knob | LLM / vision | Bottleneck |
|-------|-------------------|----------------|--------------|------------|
| Worker claim | `WORKER_THREADS` | cpus×4; local 4 | 0 | One ingest per workspace lane |
| PDF → MD | `PdfVisionSemaphore` + page `buffer_unordered` | `EDGEQUAKE_PDF_VISION_JOBS`, concurrency ≤2 cloud | 1 VLM / page | Semaphore |
| Chunk | Sync CPU | adaptive / env | 0 (semantic: 1 embed) | Semantic one-shot |
| Extract | `buffer_unordered(N)` + redundant Semaphore | 16 cloud / 1 local | 1 / chunk | Barrier before embed |
| Glean | Serial inside chunk permit | `max_gleaning`=1 cloud | +1 / chunk | Fail discards base |
| Embed | Chunk → entity → rel **serial** | `EDGEQUAKE_EMBED_MAX_ASYNC`=8 | 0 | No overlap |
| Persist | Serial awaits | — | 0 | Independent stores chained |
| Merge | 4 phases serial | `EDGEQUAKE_MERGE_MAX_ASYNC`=8 | rare | Phases independent |

## Key findings (audit 2026-10-02)

**Parallelism:** Only extraction fans out. Embed types and persist writes are chained.

**Frugality:** ~2 LLM calls/chunk on cloud (base + glean). No chunk-hash extraction cache. Unique-before-embed sets computed twice.

**Reliability:** Per-chunk `tokio::spawn` checkpoint is O(N²) and racy. Heartbeat fixed at 60s while lease TTL can be 30s. Task backoff defeated (Pending + 2s poll). Stacked retries (3×3×2×3).

**Rasterization:** `edgequake-pdf2md` 0.9.11 ignores `dpi`; only `max_rendered_pixels` (2000 print / 3600 manuscript) changes the image. Adaptive DPI is cosmetic.

**DRY/SOLID:** `worker.rs` ~2583, `pdf_processing.rs` ~2580, `merger/mod.rs` ~2229; duplicated factories, checkpoint savers, six retry loops, triplicated pricing.

## Quick wins (shipped with SPEC-156)

1. Single-writer coalescing partial-chunk checkpoint (`run_progress_writer` pattern).
2. Gleaning fail-open + early stop; remove dead `always_glean`.
3. Parallel chunk/entity/relationship embeds under one shared semaphore.
4. Heartbeat interval = lease TTL / 3; warn on failure.
5. `extraction_time_ms` in resilient loop; drop redundant Semaphore; honest PDF env docs.

## Decision log

| Decision | Choice | Why |
|----------|--------|-----|
| Raster posture | Quality-first default; cost-first as eval-gated option | Acc / OmniDocBench quality bar; don't lower ceiling without harness |
| Extraction cache | Roadmap, not quick win | Needs prompt-version keying + SPEC-103 store wiring |
| Unified RetryPolicy | Roadmap | Touches sota + worker + claim; risk of Acc regression |
| Default concurrency | Unchanged | Acc pins local/fifo; cloud 16 stays |
| Spec number | 156 | After SPEC-155 |

## Non-goals (this pack)

- Changing default `max_rendered_pixels` or PDF concurrency.
- Shipping adaptive per-page EdgeParse skip (roadmap + harness).
- Splitting mega-files beyond the new small modules this pack adds.
