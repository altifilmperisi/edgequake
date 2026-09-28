# 06 — Backend architecture (Full Stack)

Parent: [README](README.md) · Pipeline: [02](02-current-pipeline.md) · Data: [07](07-data-model.md) · Plan: [10](10-implementation-plan.md)

## WHY

Public APIs must expose page health and partial reprocess without growing the already-large `pdf_processing.rs`. Planner is pure; executor is I/O.

## API contract

### `GET /api/v1/documents/{document_id}/pages/health`

Response:

```json
{
  "document_id": "…",
  "page_count": 12,
  "pages": [
    {
      "page_number": 1,
      "parse": { "status": "ok", "error": null },
      "figures": { "status": "ok", "count": 2, "error": null },
      "entities": { "status": "failed", "chunk_count": 3, "failed_chunk_count": 1, "error": "…" }
    }
  ],
  "summary": { "parse_failed": 0, "figures_failed": 0, "entities_failed": 1 },
  "source": "stored" 
}
```

`source`: `stored` | `derived` (legacy docs without M160 rows).

### `POST /api/v1/documents/{document_id}/pages/reprocess`

Request:

```json
{
  "pages": [3, 5],
  "stages": ["parse"],
  "dry_run": false
}
```

`pages` may also be a string `"1-3,7"` (server parses via `page_selection`).

Stages: `parse` | `figures` | `entities`.

Response (dry_run):

```json
{
  "dry_run": true,
  "pages": [3, 5],
  "requested_stages": ["parse"],
  "effective_stages": ["parse", "figures", "entities"],
  "dirty_chunk_count": 4,
  "reusable_chunk_count": 118,
  "estimated_vision_calls": 2,
  "warnings": [],
  "suggest_full_reprocess": false
}
```

Response (enqueue): `202` with `{ track_id, task_id, plan: {…} }`.

Errors: `400` bad pages · `409` busy · `422` not PDF / no markers / no vision when needed.

## Module layout (SOLID)

```text
  edgequake-api/src/
    handlers/documents/pages/
      health.rs          # GET health
      reprocess.rs       # POST reprocess
    processor/page_reprocess/
      mod.rs
      plan.rs            # PartialReprocessPlanner (pure) — S
      parse.rs           # subset OCR — S
      figures.rs         # PageAssetBundle — S
      splice.rs          # never-downgrade splice — S
      finalize.rs        # persist + enqueue Insert
    services/page_health_derive.rs  # legacy fallback
```

| SOLID | Application |
|-------|-------------|
| S | One module per stage; planner has no I/O |
| O | `page_result_sink` hook on VisionConfig |
| L | PageStateStorage impls same trait as other stores |
| I | Narrow traits: upsert_parse / upsert_figures / upsert_entities |
| D | Executor depends on traits (KV, PDF, PageState), not Postgres structs |

## Task payload extension

`PdfProcessingData` gains:

```rust
#[serde(default)]
pub page_scope: Option<PageScope>,
```

`PageScope { pages: Vec<u32>, stages: Vec<ReprocessStage> }`

No new `TaskType` (avoids partitioned CHECK migration).

## Progress phase

New phase `page_reprocess` with `counts.current/total` = selected pages completed/total. Frontend monotonic guard must allow phase change reset.

Cross-refs: AI details [08](08-ai-pipeline.md) · edge cases [09](09-edge-cases.md).
