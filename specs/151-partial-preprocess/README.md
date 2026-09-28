# SPEC-151 — Partial Page Reprocess

> **Status:** On `main` after v0.27.0 (migration **160**). Not in the v0.27.0 tag.  
> **Scope:** Reprocess selected pages of an already-uploaded PDF without discarding
> healthy pages. Stages: **parsing**, **figures/charts**, **entity extraction**.  
> **Related:** [SPEC-051 reprocess](../051-reprocess/), [SPEC-134 PDF modality](../134-pdf-modality/),
> [SPEC-135 PDF pack](../135-pdf-pack/), [SPEC-049 figure filter](../049-improve-figure-extraction/),
> [SPEC-150 migrations](../150-reliable-migration-system/).

## What operators need to know

| Goal | Do this |
|------|---------|
| Inspect page health | Open document detail → Page Health strip (P/F/E bars) |
| Reprocess failed pages | Select pages → **Reprocess pages…** → choose stage → confirm |
| Preview impact | Dialog calls `dry_run=true` before enqueue |
| Reprocess via API | `POST /api/v1/documents/{id}/pages/reprocess` |
| Page health via API | `GET /api/v1/documents/{id}/pages/health` |

## One-screen architecture

```text
  UI (PageHealthStrip + ReprocessPagesDialog)
           |
           v
  POST …/pages/reprocess (dry_run | enqueue)
           |
           v
  PartialReprocessPlanner (pure) ──► PageReprocessExecutor
           |                              |
           |                    ┌─────────┼─────────┐
           |                    v         v         v
           |                 Parse    Figures   Entities
           |                 (OCR)    (assets)  (reuse+rebuild)
           |                    |         |         |
           +────────────────────┴─────────┴─────────┘
                                |
                                v
                   document_page_states (M160)
                   + spliced markdown (never downgrade)
```

## Success criteria

| ID | Criterion | Evidence |
|----|-----------|----------|
| S1 | Reprocessing N pages does not erase healthy pages' markdown | EC-151-never-downgrade + e2e |
| S2 | Stages are distinguishable: parse / figures / entities | UI + API contract |
| S3 | Partial OCR progress is durable (survives crash/failure) | `document_page_states.raw_markdown` |
| S4 | Entity rebuild reuses unchanged chunks by content hash | ChunkReuseIndex tests |
| S5 | Polished UX with inspected screenshots | `e2e/screenshots/` + ANALYSIS.md |

## Reading order

1. **Why / laws:** [00-why](00-why.md) · [01-first-principles](01-first-principles.md)
2. **As-is:** [02-current-pipeline](02-current-pipeline.md)
3. **Needs by lens:** [03-requirements](03-requirements.md) (PO) · [04-ux-journeys](04-ux-journeys.md) · [05-ui-spec](05-ui-spec.md) · [06-backend](06-backend-architecture.md) · [07-data-model](07-data-model.md) · [08-ai-pipeline](08-ai-pipeline.md)
4. **Build:** [09-edge-cases](09-edge-cases.md) · [10-implementation-plan](10-implementation-plan.md) · [11-test-plan](11-test-plan.md) · [12-risks](12-risks.md)
5. **Proof:** [e2e/screenshots/ANALYSIS.md](e2e/screenshots/ANALYSIS.md)

## Non-goals

- Replacing full-document reprocess (`POST /documents/reprocess` full/entities/merge).
- Surgical per-entity graph edits (we rebuild document indexes with chunk reuse).
- Per-page chunk ID scheme change (`{doc}-chunk-{N}` stays; reuse is by content hash).
- Object-store migration of PDF blobs.
