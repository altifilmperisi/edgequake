---
title: "Serving fence decision"
---

# Serving fence default decision (GAP-091-21b / SPEC-091 IP2)

> **Status:** Accepted — default **on** (2026-07-31, SPEC-091 IP2)
> **Spec:** [21-ingestion-pipeline-data-model-improvement.md](../../specs/091-simplify-data-layer/21-ingestion-pipeline-data-model-improvement.md) LAW-IP1 / IP-AC-05

## Decision

`EDGEQUAKE_SERVING_FENCE` defaults **on** when unset. Explicit `off` / `false` / `0` disables for soak rollback.

Query and list paths filter ANN/FTS to `chunk_serving_state = ready` unless operators opt out.

## Rationale

LAW-IP1 / LD-09: a chunk is query-visible only when text ∧ embedding ∧ graph ∧ ready agree. Leaving the fence off after typed cutover left partial tuples queryable — the opposite of fail-closed integrity.

IP2 productizes `outbox_events` writers and marks `ready` after successful persist+merge; operators who need the old behavior set `EDGEQUAKE_SERVING_FENCE=off`.

## Evidence on disk

| Artifact | What it shows |
| --- | --- |
| `edgequake-storage/src/serving_fence.rs` | unset → fence **on**; only explicit off disables |
| `migrations/109_spec091_serving_fence.sql` + `133_spec091_outbox_harden.sql` | fence + outbox schema |
| `ingestion_persister.rs` | ready mark + outbox `chunk_ready` / `merge_done` / `compensate` |

## Operator guidance

1. Fresh installs / HEAD: fence on by default — ensure ingest completes (ready mark) before expecting query hits.
2. Soak rollback: `export EDGEQUAKE_SERVING_FENCE=off`.
3. List UI: `query_ready` enrichment when fence on ([20](../../specs/091-simplify-data-layer/20-ingestion-surface-assessment.md)).

## Revisit trigger

If a measured recall regression appears on partial-ingest demos, document the delta under `specs/091-simplify-data-layer/measurements/` and consider a staged default for one release — do not silently revert without LAW-I2 evidence.
