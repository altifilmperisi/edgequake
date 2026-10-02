# 06 — Data and database contract

Parent: [README](README.md) · Architecture: [05](05-frontend-architecture.md) ·
AI: [07](07-ai-engineer-spec.md)

## Scope

SPEC-157 is **primarily frontend**. Default path needs **no new tables or
migrations**. This document lists the fields consumed, persistence behaviour,
and an **optional** API contract hardening (W5) if non-stream / reload gaps
block Graph pane quality.

## Fields consumed (already exist)

### Retrieval / chat context (frontend `QueryContext`)

From [`types/query.ts`](../../edgequake_webui/src/types/query.ts):

| Field | Use in Source pane | Use in Graph pane |
|-------|--------------------|-------------------|
| `chunks[].document_id` | Load document | — |
| `chunks[].chunk_id` | Highlight / deeplink | — |
| `chunks[].page_start` / `page_end` | PDF page + badge | — |
| `chunks[].start_line` / `end_line` | Text scroll/highlight | — |
| `chunks[].content` | Passage preview / highlight | — |
| `chunks[].file_path` | Title fallback | — |
| `entities[].id` / `label` | — | Focus nodes |
| `entities[].relevance` | — | Salience (optional) |
| `relationships[]` | — | Edges in subgraph |

### Persisted message context (API)

[`message_context_mapper.rs`](../../edgequake/crates/edgequake-api/src/services/message_context_mapper.rs)
maps engine subgraph → `MessageContext` for DB persistence. Pages on sources
are already persisted (SPEC-033 comment in mapper). Entity **graph node ids**
may be weak after reload — client must fall back to normalised name matching
(LAW-157-10).

### Streaming vs non-streaming

| Path | Subgraph |
|------|----------|
| SSE streaming (`handlers/chat/streaming.rs`) | Builds and may emit `subgraph` |
| Non-stream completion (`handlers/chat/mod.rs`) | Observed `subgraph: None` (F-157-08) |

Graph pane must work from `message.context.entities/relationships` even when
`subgraph` is absent (map locally via `answer-graph.ts`).

## Companion URL contract (client-only)

```text
  pane=pdf|graph          required when open
  doc=<documentId>        required when pane=pdf
  page=<int>=1            optional
  chunk=<chunkId>         optional
  msg=<messageId>         required when pane=graph

  Sanitise:
    - unknown pane → closed
    - pdf without doc → closed
    - page < 1 → omit
    - graph without msg → closed (or lastAnswer if msg missing + focus=answer style — prefer closed for honesty)
```

No server persistence of companion layout in v1.

## Local persistence

| Key | Contents |
|-----|----------|
| `edgequake.query.companion.v1` | `{ version, widthPx, lastKind }` |
| Existing history width key | Unchanged; auto-rail is runtime |

Migrate unknown versions → defaults (EC-157-13).

## Optional backend change (W5, contract-only)

**Goal:** Always attach resolvable node ids and subgraph on chat responses
(stream and non-stream), so Graph pane after reload does not depend on fragile
name matching.

| Change | Migration? | Gate |
|--------|------------|------|
| Non-stream path sets `subgraph` like streaming | No schema change | `spec027_api_contract` / chat contract tests |
| Persist `graph_node_id` on `MessageContextEntity` if not already | Likely JSON shape only; confirm OpenAPI | API contract test |
| Ensure reload of conversation returns entities with ids | No DDL if already JSONB | Integration test |

**Do not** add tables for companion prefs in SPEC-157.

## Database expert notes

```text
  READ paths used:
    GET document by id
    GET pdf content / download URL (auth)
    GET conversation messages (existing context JSON)
    Optional: neighbourhood / entity lookup for expand

  WRITE paths:
    None new for companion open/close
    Chat submit unchanged (W4 may add document_ids scope — existing API)

  Isolation:
    Workspace-scoped document fetch must 404/403 cross-workspace
    → companion shows honest error (EC-157-07)
```

Indexes: none new. PDF auth and document fetch reuse existing paths.

## Failure modes (data)

| Mode | Client behaviour |
|------|------------------|
| Document 404 | Source error + Open full page disabled or explained |
| PDF binary 404 | Existing `PDFViewer` error copy + Retry |
| Auth expired | Existing authenticated PDF source failure path |
| Empty entities | Graph empty state |
| Unresolved names | Partial state listing names |

## Contract ASCII

```text
  Engine QueryContext
        |
        +-- map --> MessageContext (persist JSON)
        |              |
        |              +-- reload conversation --> Query UI messages
        |
        +-- stream event subgraph? --> useAnswerGraphStore
                                           |
                                           v
                                    answer-graph.ts
                                           |
                           +---------------+---------------+
                           |                               |
                     SourceLocation                  Focus ids / edges
                           |                               |
                     DocumentSourceView            EmbeddedAnswerGraph
```

Cross-ref: [08-edge-cases](08-edge-cases.md) ·
[lenses/LENS-database.md](lenses/LENS-database.md) · F-157-08, F-157-09 in
[02-surfaces](02-surfaces.md).
