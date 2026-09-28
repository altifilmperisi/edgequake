# 01 — First principles (LAW-151)

Parent: [README](README.md) · Why: [00](00-why.md) · Data: [07](07-data-model.md) · AI: [08](08-ai-pipeline.md)

These laws are derived from the pipeline physics — not taste. Code that violates a law is a defect.

## LAW-151-1 — The page is the unit of truth for PDF repair

A PDF page is the natural boundary for OCR, asset crops, and (via markers) chunk spans.

```text
  PDF page N
    ├── parse: raw markdown section <!-- edgequake-page:N -->
    ├── figures: mm-assets with page_num = N
    └── entities: chunks where page_start..page_end overlaps N
```

**Implication:** health, selection, and reprocess are expressed in page numbers (1-indexed), not chunk indexes or byte offsets.

## LAW-151-2 — Never downgrade a page

If a reprocess attempt for page N fails, **keep** the previous markdown section and assets.

```text
  before: section_N = GOOD
  attempt fails → after: section_N = GOOD (unchanged)
  attempt succeeds → after: section_N = NEW
```

Empty / placeholder sections may be replaced. Non-placeholder sections may only be replaced by a successful new parse.

## LAW-151-3 — Stage order is Parse → Figures → Entities

```text
  Parse  ──includes──► Figures ──includes──► Entities
  Entities alone is allowed (markdown unchanged).
  Figures alone implies Parse if raw OCR for those pages is missing.
```

Closure is computed by the planner (pure function). Downstream stages are locked and explained in the UI, not silently skipped.

## LAW-151-4 — Reuse extractions by content hash, not by positional chunk ID

Chunk IDs are `{doc}-chunk-{N}` and shift when page text length changes. Therefore:

```text
  snapshot.chunks[i].content ──SHA-256──► reuse_key
  new_chunks[j].content     ──SHA-256──► lookup
  hit + not overlapping dirty pages → rebind ExtractionResult to new chunk id
```

Dirty = any chunk whose `page_start..page_end` overlaps the selected page set (cross-page packs count).

## LAW-151-5 — Document graph rebuild, not surgical edge surgery

Removing a page's contributions via prefix matching and a 200-source-id cap is lossy ([document_graph_cascade](../../edgequake/crates/edgequake-api/src/services/document_graph_cascade.rs)).

**Chosen strategy:** retract this document's indexes → merge from a hybrid extraction set (dirty fresh + clean reused). The document may be briefly unqueryable; correctness > surgical precision.

## LAW-151-6 — Durable page state outlives the convert job

```text
  on_page_complete → UPSERT document_page_states
                     (raw_markdown, parse_status, …)
  on_convert_fail  → salvage leftover pdf2md checkpoints into states
  on_success       → states remain (checkpoints on disk may be deleted)
```

## LAW-151-7 — One active task per document for page reprocess

Admission returns **409** if Pending/Processing tasks exist for the document. Cancel-then-requeue is explicit, never silent.

## LAW-151-8 — DRY single sources of truth

| Concern | SSOT module |
|---------|-------------|
| Page section split/replace | `edgequake-pdf::page_sections` |
| Page selection parse | `edgequake-pdf::page_selection` |
| Stage closure | `ReprocessStage::closure` |
| Chunk reuse | `ChunkReuseIndex` |
| Page health row | `document_page_states` (+ derived fallback) |

Cross-refs: requirements [03](03-requirements.md) · edge cases [09](09-edge-cases.md).
