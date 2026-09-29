# 03 — Object model (agent view)

Parent: [README](README.md) · Architecture: [02](02-architecture.md) · Tools: [04](04-tool-contract.md)

Agent-visible objects are a **projection** of storage and REST DTOs. Storage keys MAY differ; the agent surface MUST use the forms below.

---

## Stable IDs

| Object | Agent ID form | Display | Storage / REST today |
|--------|---------------|---------|----------------------|
| Workspace | UUID | `name` | UUID |
| Document | ULID/UUID as today | `title` **and** `file_name` | Document UUID |
| Chunk | `{document_id}-chunk-{n}` | file + line range | Same (not `chk_`) |
| Entity | `ent:{workspace}:{stable_slug}` | Title Case label | Graph bare `NAME` or `{ws}::NAME`; ContextEntity `ent:{NAME}` |
| Relationship | `rel:{source}:{type}:{target}` | type + endpoints | Same pattern in mapper |
| Retrieval | `ret_` + UUID | query fingerprint | Cache key |
| Resource | `eq://{workspace}/{kind}/{id}` | — | New |

During the compatibility window, `eq_entity_get` / `eq_neighborhood` MUST accept:

- `ent:{workspace}:{slug}`
- `ent:{NAME}` (SPEC-028 ContextEntity)
- bare slug / graph node id

Projection ALWAYS emits the `ent:{workspace}:{slug}` form when workspace is known.

`RELATED_TO` is **forbidden** as a default edge type in agent views ([06](06-retrieval-and-graph.md)).

---

## Document

```json
{
  "id": "01a0b824-0000-0000-0000-000000000001",
  "title": "SoL-Pi: Recursively Scaling Auto-Research Loops",
  "file_name": "sol_pi_2609.20519v1.pdf",
  "status": "completed",
  "created_at": "2026-09-19T05:29:24Z",
  "chunk_count": 47,
  "entity_count": 86,
  "bytes": 2481132
}
```

| Field | Required | Notes |
|-------|----------|-------|
| `id` | yes | Document UUID |
| `title` | yes | May equal `file_name` if unset |
| `file_name` | yes when known | Scope and citation display |
| `status` | yes | Align with document pipeline statuses |
| `created_at` | SHOULD | RFC 3339 |
| `chunk_count`, `entity_count`, `bytes` | SHOULD | Catalog density without fetch |

---

## Chunk

```json
{
  "id": "01a0b824-0000-0000-0000-000000000001-chunk-7",
  "document_id": "01a0b824-0000-0000-0000-000000000001",
  "file_name": "sol_pi_2609.20519v1.pdf",
  "start_line": 510,
  "end_line": 581,
  "page": 5,
  "score": 0.81,
  "text": "…"
}
```

| Field | Required | Notes |
|-------|----------|-------|
| `id` | yes | `{document_id}-chunk-{n}` |
| `document_id` | yes | Lineage |
| `file_name` | SHOULD | Cross-doc bleed detection |
| `start_line`, `end_line`, `page` | SHOULD | From lineage when present |
| `score` | yes when ranked | See `score_type` on parent envelope |
| `text` | view-dependent | Omitted or truncated per budget; full body MAY live behind `eq://` |

---

## Entity

```json
{
  "id": "ent:ws3:action_fusion",
  "name": "Action Fusion",
  "slug": "ACTION_FUSION",
  "type": "Method",
  "one_liner": "Merges edit/write + run into one provider request.",
  "degree": 29,
  "document_ids": ["01a0b824-0000-0000-0000-000000000001"]
}
```

### Allowed `entity.type` in agent views

`Person | Organization | Concept | Method | System | Metric | Event | Artifact | Location`

Map storage labels (often ALL_CAPS: `PERSON`, `METHOD`, …) to Title Case enum values in the projection. Unknown storage types → `Concept` (or `Artifact` if clearly figure/drawing).

### Artifacts / figures

`DRAWING` / figure-OCR nodes are `Artifact`. They MUST NOT appear in the default entity list unless `include_artifacts: true`.

### Compactness

| Budget | Max `one_liner` chars |
|--------|----------------------|
| `cheap` | 160 |
| `standard` | 280 |
| `deep` | 800 |

Full description lives on `eq_entity_get` or `eq://…/entities/{id}`, not in default search/neighborhood lists.

Deduplicate: `ACTION_FUSION` and “Action Fusion” are one slug.

Every agent-visible entity MUST carry `document_ids` when known (empty array only if truly corpus-global, which EdgeQuake entities are not).

---

## Relationship

```json
{
  "id": "rel:ent:ws3:action_fusion:IMPLEMENTS:ent:ws3:sol_pi",
  "type": "IMPLEMENTS",
  "source": "ent:ws3:action_fusion",
  "target": "ent:ws3:sol_pi",
  "confidence": 0.72
}
```

Preferred types: `PART_OF`, `IMPLEMENTS`, `EVALUATED_ON`, `PROPOSED_BY`, `CITED_IN`, `CONTRASTS_WITH`, `MEASURES`.

`RELATED_TO` only with `confidence < 0.4` and **hidden** unless `include_weak_edges: true`.

---

## Hit (search)

```json
{
  "kind": "chunk",
  "id": "01a0b824-…-chunk-7",
  "document_id": "01a0b824-…",
  "title": "Action Fusion mechanism",
  "snippet": "≤ 240 chars",
  "score": 0.81
}
```

`kind`: `chunk | document | entity`.

---

## Citation

```json
{
  "quote": "…",
  "document_id": "…",
  "page": 5,
  "chunk_id": "…-chunk-7",
  "score": 0.81
}
```

---

## Retrieval session

| Field | Notes |
|-------|-------|
| `retrieval_id` | `ret_{uuid}` — capability token for the workspace |
| `expires_at` | RFC 3339; MUST match cache |
| `fingerprint` | Same query + corpus version ⇒ same fingerprint (evals) |

---

## Resource URIs

```text
eq://{ws}/workspaces
eq://{ws}/documents
eq://{ws}/documents/{doc_id}
eq://{ws}/documents/{doc_id}/outline
eq://{ws}/documents/{doc_id}/text
eq://{ws}/chunks/{chunk_id}
eq://{ws}/entities/{entity_id}
eq://{ws}/retrievals/{ret_id}
```

Large text lives behind resources. Tool results carry IDs and snippets; `resources/read` hydrates under the same budget rules.
