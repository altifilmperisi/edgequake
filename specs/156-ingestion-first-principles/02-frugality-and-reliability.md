# 02 — Frugality and reliability

## Gleaning policy (quick win)

| Behavior | Before | After |
|----------|--------|-------|
| Glean LLM error | `.await?` fails whole chunk; outer retry re-pays base | Fail-open: keep base, warn, stop glean |
| Empty glean iteration | Continues to `max_gleaning` | Early stop when 0 new entities and 0 new relationships |
| `always_glean` | Dead field | Removed |

Cloud default remains `max_gleaning=1` (Acc / LightRAG parity). Early stop only helps when `max_gleaning≥2`.

## Partial checkpoint (quick win)

**Bug:** each completed chunk `tokio::spawn`s `save_partial_chunk_extraction`, which:

1. Clones full document text and SHA-256s it.
2. Reads + deserializes the whole blob.
3. Inserts one chunk, double-serializes, upserts.

That is O(N²) bytes and lost updates under concurrent RMW.

**Fix:** single-writer mpsc consumer (same pattern as [run_progress_writer.rs](../../edgequake/crates/edgequake-api/src/services/run_progress_writer.rs)):

- Hash content once at spawn.
- Coalesce (~250 ms) and flush on drop / `flush_now`.
- Append chunk results in one ordered consumer.

New module: `processor/partial_chunk_checkpoint_writer.rs` (keep `pipeline_checkpoint.rs` from growing).

## Retry stacking (roadmap)

| Layer | Attempts |
|-------|----------|
| sota / base extract | 3 |
| Per-chunk resilient | 3 |
| Glean effort lift | 2 |
| Task | 3 |

Target: one `RetryPolicy` (full jitter, Retry-After, non-retryable short-circuit) and a **per-chunk attempt budget** so layers do not multiply.

Also: real task backoff via `available_at` (revive orphaned `claim_eligibility.rs`).

## Heartbeat vs lease (quick win)

- Heartbeat was fixed 60s; lease TTL min 30s → double processing window.
- Fix: interval = `max(ttl/3, 5s)` from one helper; warn (not debug) on refresh failure.

## Extraction / embed cache (roadmap)

Key: `hash(chunk_text, model, prompt_version, caps, glean_config)`.  
Store: reuse SPEC-103 `public.llm_cache` (extract out of v1 today).  
Embed: `hash(model, text)` across documents.

## Cost accounting gaps (roadmap)

- Failed / retry attempts not counted.
- `summarization_cost_usd` never written.
- Prompt-cache hit/write invisible on document cost.
- Embedding cost estimated `chars/2.5`, not provider usage.
- No vision line item.
- Pricing tables triplicated (`pipeline/extraction.rs`, `progress/cost.rs`, `edgequake-llm`).

## Stall detection (roadmap)

Client-only 15 min silence. Server needs a run-progress ledger reaper (SPEC-155 ledger already exists).

## Tests

- Glean fail-open + early stop with mock LLM.
- Checkpoint writer: N concurrent completes → N chunks persisted; no lost ids.
- Heartbeat derivation for TTL 30 / 120 / 600.
