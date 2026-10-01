# 08 — Data contract (graph & query)

Parent: [README](README.md) · Findings B-* [04](04-findings.md) · Laws: 6,10,11 · Lens: [Database](lenses/LENS-database.md) · Next: [09-edge-cases](09-edge-cases.md)

## WHY

UI honesty and Graph Studio features fail when totals, degrees, communities,
and answer subgraphs are wrong or missing. Server owns graph truth
(LAW-155-10).

## Current SSOT (AGE)

```text
  One AGE graph · Node / EDGE labels
  Isolation: tenant_id + workspace_id properties
  Node id: {workspace_uuid}::NAME
  Denorm: eq_node_id, eq_source_id, eq_target_id, eq_rel_type
  Communities: node props community_id (+ community_report) via community_index_service
  Degree: computed on the fly (definition inconsistent)
  Counts: node_count_fast / edge_count_fast = pg_class.reltuples (GLOBAL)  ← bug
```

## Cheap wave (W3) — must ship

| Change | Detail | Closes |
|--------|--------|--------|
| Workspace totals | Use `pg_node_count_by_workspace` / edge equivalent for `total_*` and stream metadata; `is_truncated = returned < total` OR `returned >= max_nodes && total > returned` | B01, G03 |
| Stream `start_node` | Honour param via existing BFS; document in OpenAPI | B02 |
| Degree SSOT | Always expose `{in, out, total}`; popular/search/get_graph/neighbourhood agree; fix start_node path degree 0 | B04, B05 |
| Tenant degrees batch | `get_degrees_batch` requires TenantContext; filter by workspace | B03 |
| Edge DTO | `id` = stable key incl. `relation_type`; `description`, `keywords`, `weight`; stop `{src}_{tgt}` alone | B06 |
| Community field | `GraphNodeResponse.community_id: Option<String>` (+ optional report snippet) | B07 |
| `GET /graph/communities` | `{ id, size, label?, modularity? }[]` workspace-scoped | B07, G12 |
| Type facets | `GET /graph/facets` → entity_type counts, relationship_type counts | UI legend |
| Chat/query linkage | Include graph `node_id` on context entities; include `subgraph` (+ retrieval stats) on chat `done` | B09, Q05 |
| UI types align | Fix `types/graph.ts` to match wire DTOs (or generate from OpenAPI) | B10 |

### Degree contract

```text
  degree: {
    in: u64,
    out: u64,
    total: u64   // in+out; self-loops counted once per product decision — document
  }
  Deprecated: bare number — accept for one release as total for compat
```

### Truncation contract

```text
  KnowledgeGraphResponse {
    nodes, edges,
    is_truncated: bool,
    total_nodes: u64,   // workspace exact (or exact-enough under lock)
    total_edges: u64,
    max_nodes: u64,     // echo request clamp
  }
```

### OpenAPI / codegen

After DTO changes: `make codegen-openapi-refresh` ·
`cargo test -p edgequake-api --test spec027_api_contract` · WebUI
`bun run codegen:api:offline`.

### Cargo e2e gates (W3)

```text
  e2e_spec155_graph_totals_workspace
  e2e_spec155_stream_start_node
  e2e_spec155_degrees_tenant
  e2e_spec155_degree_shape
  e2e_spec155_edge_id_multigraph
  e2e_spec155_communities_endpoint
  e2e_spec155_chat_done_subgraph_ids
```

## Medium wave (post-W3 / W3.5)

| Change | Notes | Finding |
|--------|-------|---------|
| ETag / `graph_version` | Per-workspace watermark; `If-None-Match` on GET /graph | — |
| `document_ids` on /graph & stream | Reuse lineage finders; return truncation if hit 5000 | B08 |
| Per-hop ego caps | Bound BFS fan-out | G17 |
| `created_at` / `updated_at` stamping | Pipeline merger write; no full backfill required | B10 |
| Materialised degree / PageRank | Refresh with community job | — |
| Lineage truncation flag | Surface LIMIT 5000 | B08 |

Indexes: prefer expression indexes if filtering `entity_type` heavily; FTS
already on labels search — keep LIKE leading-wildcard off hot paths.

## Deferred (explicit)

| Item | Trigger |
|------|---------|
| Merge suggestions | Knowledge-ops staffing |
| Server-side layout coordinates | Multi-client shared canvas |
| Graph diff / temporal versioning | Audit product |
| Edge bundling server | Scale &gt;10k edges interactive |

## Query UI consumption notes

- WebUI uses **chat** stream, not `/query/stream` — enrich chat events.
- `ContextEntity.id = "ent:{name}"` → add `graph_node_id`.
- Preserve `source_document_ids[]` (stop collapsing to one).
- Optional: expose `context_tokens` / cost later (Costs page already separate).

## Security notes

- B03 is a **tenant isolation** fix — treat as P0; align with SPEC-154 membership bind mindset.
- Totals leak (B01) is cross-tenant scale disclosure — fix before marketing multi-tenant graph.

## Migration / SQL

Prefer **no** new tables for cheap wave (properties + queries). Communities
list = aggregate distinct `community_id` with counts under workspace filter.
If exact counts are hot, optional materialised `workspace_graph_stats`
updated by merger (medium).

```text
  BEFORE: total_nodes = reltuples(shared)
  AFTER:  total_nodes = count(*) filter workspace   (or stats row)
```

Cross-ref: [06-graph-studio-spec](06-graph-studio-spec.md) · [11-e2e-test-matrix](11-e2e-test-matrix.md).
