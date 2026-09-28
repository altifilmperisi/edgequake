---
title: "JSONB envelope acceptance (GAP-091-05)"
---

# JSONB envelope acceptance (GAP-091-05)

> **Status:** Accepted by design — not a migration gap
> **Spec:** SPEC-091 IW3 / [19-improvement-plan.md](../../specs/091-simplify-data-layer/19-improvement-plan.md)

## Summary

Several typed relational tables intentionally keep **JSONB payload columns**
rather than fully normalized scalar schemas. This is an explicit product/engine
decision: envelope typing at the application layer, not a deferred schema debt.

## Accepted JSONB surfaces

| Table | Column | Purpose | Why JSONB stays |
| --- | --- | --- | --- |
| `pipeline_checkpoints` | checkpoint payload | Resume tokens, extraction snapshots | Evolving pipeline stages; low query surface |
| `document_artifacts` | artifact body | Lineage, multimodal manifests/chunks | Variable-shape artifacts; read by document id |
| `llm_cache` | `value` | LLM/keyword/multimodal cache entries | Keyed by hash; opaque provider payloads |
| `compensation_quarantine` | `payload` | Saga DLQ records | Same shape as legacy KV DLQ for operator parity |

## Non-goals (already typed elsewhere)

- Document **metadata shells** → `documents.metadata` JSONB with relational CAS
  (`document_shell.rs`) — authoritative for list/detail; not part of this gap.
- **Chunk text** → `chunks.content` text (SPEC-091 Wave D).
- **Chunk embeddings** → `chunk_embeddings.embedding` typed vector (SPEC-091 W3).

## Operator implications

- Console/advisor residue checks **exclude** transient JSONB families (checkpoints,
  cache, quarantine) from migration-125 durable guards by design.
- Drain worker (`compensation_drain.rs` + applier) interprets quarantine
  `payload.kind` — do not expect SQL-level retract without the applier.

## Verification

- Typed sidecar stores write these tables directly (`relational_sidecar_store.rs`,
  `llm_cache.rs`, `PgQuarantineSink`).
- Contract tests: `contract_spec091_llm_cache_scope.rs`, compensation DLQ tests
  in `compensation.rs`.
