# 03 — Requirements (Product Owner)

Parent: [README](README.md) · Why: [00](00-why.md) · UX: [04](04-ux-journeys.md) · Tests: [11](11-test-plan.md)

## Personas

| Persona | Need |
|---------|------|
| Analyst | Fix 2 bad OCR pages in a 40-page report without re-paying vision for the rest |
| Ops | See page-level failure counts; requeue safely while other docs process |
| Developer | API + dry_run plan for automation / scripts |

## User stories

1. As an analyst, I can open a PDF document and see which pages failed parse / figures / entities.
2. As an analyst, I can select pages (click, shift-range, or type `1-3,7`) and reprocess them.
3. As an analyst, I can choose a stage and understand which downstream stages will also run.
4. As an analyst, I see a preview of impact (pages, estimated vision calls, reusable chunks) before confirm.
5. As an ops user, I never lose already-good page markdown when a reprocess attempt fails.
6. As a developer, I can call health + reprocess APIs with dry_run.

## REQ catalogue

| ID | Requirement | Priority | AC |
|----|-------------|----------|-----|
| REQ-151-01 | Persist per-page parse/figures/entities status | P0 | M160 + health API |
| REQ-151-02 | Salvage OCR on convert failure | P0 | checkpoint → states |
| REQ-151-03 | Reprocess selected pages: parse | P0 | PageSelection::Set + splice |
| REQ-151-04 | Reprocess selected pages: figures | P0 | asset bundle scoped |
| REQ-151-05 | Reprocess selected pages: entities with chunk reuse | P0 | ChunkReuseIndex |
| REQ-151-06 | Never downgrade page markdown | P0 | EC-151-01 |
| REQ-151-07 | Stage closure Parse→Figures→Entities | P0 | planner + UI lock |
| REQ-151-08 | Dry-run impact preview | P0 | dialog preview |
| REQ-151-09 | 409 when document busy | P0 | admission |
| REQ-151-10 | Page Health strip + dialog UX | P0 | e2e screenshots |
| REQ-151-11 | Legacy health derivation (no M160 rows yet) | P1 | GET health fallback |
| REQ-151-12 | i18n en/fr/zh | P1 | locale keys |

## Acceptance criteria (product)

| AC | Statement |
|----|-----------|
| AC-151-01 | Selecting pages 3,5 and stage=parse re-OCRs only those pages; other sections byte-stable when OCR fails for a page |
| AC-151-02 | Stage=entities alone does not call vision |
| AC-151-03 | Health strip shows failed count matching backend health |
| AC-151-04 | Dry-run returns reusable_chunk_count ≥ 0 and dirty_chunk_count |
| AC-151-05 | Screenshots inspected in `e2e/screenshots/ANALYSIS.md` |

## Non-goals

- Workspace-wide partial reprocess.
- Changing global chunk ID scheme.
- Editing markdown manually then re-extracting (future).

Cross-refs: edge cases [09](09-edge-cases.md) · implementation [10](10-implementation-plan.md).
