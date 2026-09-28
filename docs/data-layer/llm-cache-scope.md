---
title: "LLM cache scope decision (GAP-091-14)"
---

# LLM Cache Scope Decision (SPEC-091, GAP-091-14)

**Status:** ACCEPTED (IW0, 2026-07-30) — pinned by
`edgequake-storage/tests/contract_spec091_llm_cache_scope.rs`.

## Decision

`public.llm_cache` entries are keyed by **content hash** within a storage
**namespace** — composite PK `(cache_key, namespace)` (migration 124). They
intentionally carry **no tenant/workspace column**:

- Two workspaces sharing a namespace **share** cache entries. A hit returns
  the LLM output previously computed for an identical prompt + model.
- Distinct namespaces are fully isolated (the namespace predicate is on every
  read/write path in `adapters/postgres/llm_cache.rs`).

## Rationale

The LLM cache is a **content-addressed recomputation guard**, not document
data: same input ⇒ same output. Sharing across workspaces is therefore
semantically safe and avoids duplicated LLM spend when workspaces ingest
overlapping corpora or issue identical keyword-extraction prompts. A lost or
isolated entry only costs one recomputation — never correctness.

## Accepted residual

- **Timing/usage side channel:** within a namespace, workspace B can observe
  (via latency) that workspace A's identical prompt was already cached. No
  content crosses — the output is deterministic for the input — but the access
  pattern leaks. Accepted: namespaces map to deployment trust boundaries.
- **Provider drift:** a cache entry written under provider/model X is served
  to a workspace configured for provider Y **only when the cache key matches**;
  cache keys already incorporate the prompt hash, and multimodal keys embed
  `{mode}-{type}` — model identity is part of the hashed prompt envelope for
  extraction caches. Operators requiring hard per-tenant cache isolation must
  deploy per-tenant namespaces (storage namespace is already a config knob).

## Consequences

- Do NOT add a workspace/tenant column to `llm_cache` without updating this
  record and the contract test.
- Cache invalidation on document delete stays namespace-scoped and
  key-targeted (no per-workspace sweep exists or is needed).
