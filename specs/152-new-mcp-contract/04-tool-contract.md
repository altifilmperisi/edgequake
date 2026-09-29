# 04 — Tool contract

Parent: [README](README.md) · Objects: [03](03-object-model.md) · Envelope: [05](05-envelope-budget-errors.md) · Schemas: [schemas/](schemas/)

RFC 2119. Every tool MUST declare `inputSchema` with `additionalProperties: false`, MUST declare `outputSchema`, MUST return validating `structuredContent`, MUST set annotations (`readOnlyHint`, `idempotentHint`, `destructiveHint`, `openWorldHint`), and MUST be self-describing in `description` (mode semantics, defaults, when not to use).

`content[0].text` MUST be a ≤ 2 KiB summary ([02](02-architecture.md)); MUST NOT dump JSON again.

Wire schemas: [`schemas/`](schemas/).

---

## 4.1 Catalog

### `eq_workspace_list`

| | |
|--|--|
| **Job** | Workspaces the caller can see. No default workspace invention. |
| **Annotations** | `readOnlyHint: true`, `idempotentHint: true`, `destructiveHint: false`, `openWorldHint: false` |
| **Inputs** | `limit?` (default 20, max 100), `cursor?` |
| **Output** | Envelope + `items: [{id, name, slug?, document_count?, entity_count?}]`, `next_cursor?` |
| **When not to use** | Do not invent a workspace if `items` is empty. |
| **REST** | `GET /api/v1/tenants/{tenant}/workspaces` (offset/limit → cursor) |

### `eq_workspace_stats`

| | |
|--|--|
| **Job** | `{document_count, entity_count, relationship_count, chunk_count, last_ingest_at?}` |
| **Annotations** | read-only, idempotent |
| **Inputs** | `workspace_id?` (header injection allowed) |
| **REST** | `GET /api/v1/workspaces/{id}/stats` (+ get for timestamps if needed) |

### `eq_document_list`

| | |
|--|--|
| **Job** | Answer “list documents in EdgeQuake / this workspace.” MUST exist on the Grok (query) connector. |
| **Annotations** | read-only, idempotent |
| **Inputs** | `workspace_id?`, `status?`, `query?` (title/file search), `date_from?`, `date_to?`, `limit` (default 20, max 50), `cursor?` |
| **Output** | Envelope + `documents: Document[]` (or `items`), `next_cursor?` |
| **Empty** | `documents: []` / `items: []` — never a hallucinated graph |
| **REST** | `GET /api/v1/documents` and/or `GET /api/v1/documents/search` |

### `eq_document_get`

| | |
|--|--|
| **Job** | Metadata by default. |
| **Inputs** | `document_id` (required), `include?: ["metadata"|"outline"|"text"]` (default `["metadata"]`) |
| **`text`** | Returns resource link `eq://…/documents/{id}/text`, not full PDF inline |
| **Annotations** | read-only, idempotent |
| **REST** | `GET /api/v1/documents/{id}` |

---

## 4.2 Retrieval

### `eq_search`

Handles, not bodies.

**Inputs:**

```json
{
  "query": "string",
  "mode": "naive|local|global|hybrid|mix",
  "scope": "workspace|documents",
  "document_ids": ["…"],
  "document_pattern": "sol_pi*",
  "budget": "cheap|standard|deep",
  "limit": 8,
  "cursor": null,
  "workspace_id": "…"
}
```

| Rule | Normative |
|------|-----------|
| `limit` default | 8; max 50 means **hits**, not sessions |
| `budget` default | `standard` |
| `mode` default | `mix` |
| `document_ids` XOR `document_pattern` when both would conflict | Prefer `document_ids`; error or ignore pattern with `mode_reason` if both set — prefer reject with `eq/invalid_id` style validation message; implementers SHOULD reject ambiguous both-set |
| Scope | If neither ids nor pattern: compute and return `cross_document` |

**Output (minimum):**

```json
{
  "ok": true,
  "view": "search",
  "budget_used": "standard",
  "retrieval_id": "ret_…",
  "expires_at": "2026-09-29T14:00:00Z",
  "mode_used": "mix",
  "mode_reason": "default mix",
  "cross_document": false,
  "score_type": "unit_interval",
  "hits": [
    {
      "kind": "chunk",
      "id": "…",
      "document_id": "…",
      "title": "…",
      "snippet": "≤ 240 chars",
      "score": 0.81
    }
  ],
  "documents_considered": ["…"],
  "next_cursor": null,
  "truncation": { "truncated": false },
  "stats": { "total_ms": 210, "cached": false }
}
```

`max_results: 50` returning **one** retrieval session is non-conformant.

**Annotations:** read-only, idempotent, `openWorldHint: false`.

**When not to use:** Do not use search to list documents — use `eq_document_list`.

**Alias:** `edgequake_search` → same with `budget=standard`.

### `eq_fetch`

Body for handles.

**Inputs:**

```json
{
  "retrieval_id": "ret_…",
  "ids": ["chunk-…", "ent:…"],
  "view": "toc|chunks|entities|citations|full",
  "budget": "standard",
  "include_subgraph": false,
  "include_artifacts": false,
  "include_weak_edges": false,
  "cursor": null
}
```

| View | Contains |
|------|----------|
| `toc` (default) | documents + top 8 entities + 3 snippets |
| `chunks` | chunk text + lineage |
| `entities` | compact entities + typed edges among them |
| `citations` | `{quote, document_id, page, chunk_id, score}` |
| `full` | chunks + compact subgraph, still budget-capped |

Default `include_subgraph` is **false**. Fetching `ids=[top hit]` MUST NOT resend the whole subgraph.

**Alias:** `edgequake_fetch` → `view=toc`, subgraph off, `budget=standard`.

### `eq_retrieve`

Optional sugar: `search` then `fetch(view=chunks)`.

MUST accept the same `scope` / `document_ids` / `budget` / `mode` as search.  
MUST still return `hits` so the agent can fetch more without re-searching.

**Alias:** `edgequake_retrieve` → `budget=standard`.

---

## 4.3 Graph

### `eq_entity_search`

| Inputs | `q`, `type?`, `document_ids?`, `include_artifacts?` (default false), `limit` default 10, `budget?`, `workspace_id?` |
| Output | Envelope + compact `entities[]` |
| Annotations | read-only, idempotent |
| REST | `GET /api/v1/graph/entities` or nodes/search |

### `eq_entity_get`

| Inputs | `entity_id`, `budget?` |
| Output | Full description (budget-capped), aliases, `document_ids`, degree, top chunks as `eq://` links |
| Annotations | read-only, idempotent |
| L2 | Required for Graph-RAG conformance |

### `eq_neighborhood`

| Inputs | `entity_id`, `max_hops` 1–2 (3 only on `budget=deep`), `edge_types?`, `document_ids?`, `include_artifacts?`, `include_weak_edges?`, `budget?` |
| Output | `entities` + typed `relationships` + `path[]` |
| Cap | Top-k neighbors by edge weight — not the raw 111-degree drawing hub |
| Annotations | read-only, idempotent |
| REST | `build_entity_neighborhood` / `GET …/neighborhood` |

---

## 4.4 Write (memory profile)

### `eq_ingest`

| Inputs | Text body and/or upload reference (`file_uri` / prior upload id), `title?`, `workspace_id?` |
| Output | `{document_id, task_id, status}` inside envelope |
| Annotations | `readOnlyHint: false`, `destructiveHint: false`, `idempotentHint: false`, `openWorldHint: true` |
| REST | `POST /api/v1/documents` / upload / pdf |

### `eq_task_get`

| Inputs | `task_id` (track id) |
| Output | Status, progress, error if any |
| Align | MCP Tasks extension if host supports it; otherwise poll |
| REST | `GET /api/v1/tasks/{track_id}` |

### `eq_document_delete` / `eq_workspace_delete`

| Annotations | `destructiveHint: true`, `readOnlyHint: false` |
| Inputs | id + **`confirm: true`** required |
| Else | `ok: false`, code `eq/confirm_required`, `isError: true` |
| Hosts | SHOULD elicit confirmation when elicitation is supported |

Query profile MUST omit write tools and MUST say so in `instructions`.

---

## 4.5 Annotation matrix (summary)

| Tool | readOnly | idempotent | destructive | openWorld |
|------|----------|------------|-------------|-----------|
| `eq_workspace_list` | T | T | F | F |
| `eq_workspace_stats` | T | T | F | F |
| `eq_document_list` | T | T | F | F |
| `eq_document_get` | T | T | F | F |
| `eq_search` | T | T | F | F |
| `eq_fetch` | T | T | F | F |
| `eq_retrieve` | T | T | F | F |
| `eq_entity_search` | T | T | F | F |
| `eq_entity_get` | T | T | F | F |
| `eq_neighborhood` | T | T | F | F |
| `eq_ingest` | F | F | F | T |
| `eq_task_get` | T | T | F | F |
| `eq_document_delete` | F | F | T | F |
| `eq_workspace_delete` | F | F | T | F |
