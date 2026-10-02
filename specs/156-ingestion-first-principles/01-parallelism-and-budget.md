# 01 — Parallelism and budget

## Target dataflow

```text
  Chunk (CPU)
     |
     +---> Extract (buffer_unordered N) --------+
     |                                          |
     +---> Embed chunks (overlap, roadmap)      v
                                         Embed entities + relationships
                                         under ONE shared embed semaphore
                                                 |
                                                 v
                                         Persist: join! independent stores
                                                 |
                                                 v
                                         Merge: independent phases can join
```

Quick win ships the **embed join** only. Chunk-embed overlap with extraction is roadmap (needs streaming unique-before-embed or accept re-embed on late entities).

## Shared budget design

| Budget | Scope | Default | Local |
|--------|-------|---------|-------|
| Extract fan-out | Per document | `EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS` = 16 | 1 |
| Embed fan-out | Per document, **shared** across chunk/entity/rel | `EDGEQUAKE_EMBED_MAX_ASYNC` = 8 | 1 |
| Merge fan-out | Per phase | `EDGEQUAKE_MERGE_MAX_ASYNC` = 8 | 2 |
| PDF vision jobs | Process-wide | `EDGEQUAKE_PDF_VISION_JOBS` = 2 | same |
| PDF page concurrency | Per convert job | computed ≤ 2 cloud | 1–2 |
| Workspace ingest lane | Tenant × workspace | 1 | 1 |

**Rule (LAW-156-2):** do not invent a third semaphore that duplicates `buffer_unordered(N)`. One bound is enough.

## Quick win: parallel embeddings

Today ([embeddings.rs](../../edgequake/crates/edgequake-pipeline/src/pipeline/helpers/embeddings.rs)):

1. Await chunk embeds.
2. Await entity embeds.
3. Await relationship embeds.

Target:

1. Precompute unique entity / relationship text sets once.
2. `tokio::try_join!` the three stages.
3. Cap total in-flight embed HTTP calls with a single `Semaphore(embed_max_async)`.
4. When `embed_max_async == 1` (local), the join still runs but the semaphore serializes — same wall clock, simpler code path.

Risks:

- Provider TPM/RPM: cloud already sees up to `embed_max_async` in-flight within one stage; joining types does not increase the cap, only fills idle slots between stages.
- Memory: three text vectors live together briefly — acceptable vs LLM payload sizes.
- Progress callbacks: emit stage start/end as today; interleaved progress is OK for the ledger.

## Persist / merge (roadmap)

Independent after durable `commit_batch`:

- KV chunks, relational chunks, chunk vectors, typed embeddings → `tokio::try_join!`.
- Typed merge: entity graph vs relationship vector phases can overlap after placeholders are reserved.

Do **not** parallelize workspace ingest lane without a graph-write isolation story (AGE / merge races).

## Redundant semaphore (quick win)

`resilient_extract_parallel` and `extract_parallel_with_progress` both wrap `buffer_unordered(N)` with `Semaphore::new(N)`. Remove the inner Semaphore; keep `buffer_unordered` as the sole bound. Permit-held-across-retry semantics must be preserved via the stream concurrency limit (already true: one future per slot until complete).

## Tests

- Peak in-flight ≤ `embed_max_async` under three-way join (latency injects).
- Wall clock of join < ~⅓ of serial for equal-latency stages when async ≥ 3.
- Extract fan-out contract (`e2e_perf_extraction_fanout.rs`) still passes after Semaphore removal.
