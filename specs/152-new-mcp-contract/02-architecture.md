# 02 — Architecture

Parent: [README](README.md) · As-is: [01](01-current-surfaces.md) · Tools: [04](04-tool-contract.md) · Plan: [09](09-implementation-plan.md)

## Decision: one projection, two transports

REST stays wide. MCP stays a **thin AgentView projection** over existing services. Both Streamable HTTP and stdio MUST share that projection so budgets and schemas cannot drift.

```mermaid
flowchart TB
  hosts[MCP_Hosts]
  http["POST_/mcp"]
  stdio["stdio_bridge"]
  dispatch[gateway_dispatch]
  project[mcp_project_AgentView]
  ctx[QueryContextService]
  docs[Documents_Workspaces_Tasks]
  graph[Entities_Neighborhood]

  hosts --> http
  hosts --> stdio
  stdio --> http
  http --> dispatch
  dispatch --> project
  project --> ctx
  project --> docs
  project --> graph
```

### Module placement (target)

```text
edgequake-api/src/mcp/
  gateway/          # transport, auth, JSON-RPC (keep)
  project/          # NEW — AgentView
    mod.rs
    envelope.rs     # ok, view, budget_used, truncation, stats
    summary.rs      # ≤2 KiB content[0].text
    budget.rs       # cheap|standard|deep KiB + item caps
    search.rs       # hits[] from ContextBundle
    fetch.rs        # views + id subset + cursor
    catalog.rs      # document/workspace list/get
    graph.rs        # entity search/get/neighborhood
    ids.rs          # ent:{ws}:{slug}, chunk id normalize
    errors.rs       # eq/* structured errors
  auth/
  registry.rs
```

`@edgequake/mcp-server` becomes a **stdio bridge** that forwards `tools/list` and `tools/call` to `POST /mcp` (same schemas). Local file ingest may upload via REST first, then poll via `eq_task_get`. It MUST NOT reimplement ranking, budgets, or entity compaction.

---

## What REST MUST NOT change (Phases A–B)

| Endpoint | Constraint |
|----------|------------|
| `POST /api/v1/query/context/search` | May keep returning one `ContextSearchResult` for REST clients |
| `POST /api/v1/query/context` | `ContextRetrievalResponse` / `ContextBundle` shape stable |
| `GET /api/v1/query/context/{retrieval_id}` | Same cache; fetch options may gain callers |

MCP `eq_search` calls the **same retrieve path** `search_context` already uses, then **projects** the cached bundle into `hits[]`. REST cardinality is independent of MCP hit cardinality.

---

## Profiles

| Profile | Default for | Tools |
|---------|-------------|-------|
| **query** | Remote `/mcp` (Grok, Cursor remote) | Catalog + retrieval + graph reads. No ingest/delete. `instructions` MUST say the connector is query-only. |
| **memory** | stdio server; optional remote flag / scope | query + `eq_ingest`, `eq_task_get`, `eq_document_delete`, `eq_workspace_delete` |

No `eq_answer` / LLM essay tool in either profile. Evidence only.

---

## Tool surface (normative names)

### Catalog (read-only, idempotent)

| Tool | Job |
|------|-----|
| `eq_workspace_list` | Workspaces the caller can see |
| `eq_workspace_stats` | Counts + `last_ingest_at` |
| `eq_document_list` | List / filter documents (answers “what’s in the workspace”) |
| `eq_document_get` | Metadata (default); outline/text via include / resource link |

### Retrieval

| Tool | Job |
|------|-----|
| `eq_search` | Handles: `hits[]` + `retrieval_id` |
| `eq_fetch` | Body for handles; views; default `toc`, subgraph off |
| `eq_retrieve` | Sugar: search then fetch `view=chunks`; MUST still return `hits` |

### Graph

| Tool | Job |
|------|-----|
| `eq_entity_search` | Compact entity hits |
| `eq_entity_get` | Full description + lineage links |
| `eq_neighborhood` | Nodes + typed edges + path; hop-capped |

### Write (memory profile)

| Tool | Job |
|------|-----|
| `eq_ingest` | Text or upload reference → `{document_id, task_id, status}` |
| `eq_task_get` | Poll ingest / task |
| `eq_document_delete` | Destructive; `confirm: true` required |
| `eq_workspace_delete` | Destructive; `confirm: true` required |

### L1 ship set (query profile)

`eq_document_list`, `eq_search`, `eq_fetch`, `eq_retrieve`, `eq_entity_search`, `eq_neighborhood` + envelope + budget + truncation + summary text + `instructions`.

---

## Compatibility aliases

For **one minor version**, the gateway MUST keep:

| Alias | Dispatches as |
|-------|----------------|
| `edgequake_search` | `eq_search` with `budget=standard` |
| `edgequake_fetch` | `eq_fetch` with `budget=standard`, default view `toc`, subgraph off |
| `edgequake_retrieve` | `eq_retrieve` with `budget=standard` |

Aliases MUST enforce the new budget so old clients stop exploding context. After the window, aliases MAY return `eq/invalid_id`-style deprecation or be removed in a major bump.

OAuth scope map ([`oauth/scopes.rs`](../../edgequake/crates/edgequake-api/src/oauth/scopes.rs)):

| Scope | Tools |
|-------|-------|
| `edgequake:read` | `tools/list`, initialize, fetch, catalog, entity_get, neighborhood, resources |
| `edgequake:query` | search, retrieve, entity_search |
| `edgequake:write` (new) | ingest, task_get, deletes |

---

## Graph quality: two layers

| Layer | When | Behavior |
|-------|------|----------|
| **Projection (L1/L2)** | Every agent response | Hide `DRAWING`/`ARTIFACT` unless `include_artifacts`; hide `RELATED_TO` unless `include_weak_edges`; cap neighbor degree; truncate to `one_liner` |
| **Extraction (follow-on)** | Pipeline prompts | Prefer typed edges: `PART_OF`, `IMPLEMENTS`, `EVALUATED_ON`, `PROPOSED_BY`, `CITED_IN`, `CONTRASTS_WITH`, `MEASURES` |

L1 MUST NOT wait on corpus re-ingestion. Perfect MCP envelopes with raw drawing hubs still drown agents — projection filters are mandatory; typed-edge extraction is tracked separately.

---

## Text vs structuredContent (protocol deviation)

MCP tools spec: servers that return `structuredContent` SHOULD also put serialized JSON in a TextContent block.

**EQ-MCP override:** `content[0].text` MUST be a human summary ≤ 2 KiB:

1. Status line (`ok`, `view`, `budget_used`, `mode_used` if any).
2. Up to 8 hit / item lines (`kind id score title`).
3. Truncation line if `truncation.truncated`.

It MUST NOT be a second copy of the JSON. `structuredContent` is the validated payload hosts MUST prefer.

Rationale: LAW-152-2. Hosts that only read text still see handles; hosts that read both do not pay twice.

---

## Cursors

| Surface | Cursor meaning |
|---------|----------------|
| `eq_document_list`, `eq_workspace_list` | Opaque encoding of page/offset over existing REST list APIs |
| `eq_search` / `eq_fetch` | Index into the cached retrieval bundle under the same budget |
| MCP `resources/list` | Protocol `nextCursor` |

No numeric `page` on the MCP agent surface.

---

## Headers / workspace

Logical header name in schemas: `Workspace-Id` (`x-mcp-header`).

Runtime (already implemented): inject from `X-Workspace-Id` or `Mcp-Param-Workspace-Id` into tool args when missing ([`dispatch.rs`](../../edgequake/crates/edgequake-api/src/mcp/gateway/dispatch.rs), [`validate.rs`](../../edgequake/crates/edgequake-api/src/mcp/gateway/validate.rs)). Do not invent a third required header wire name without updating both sites.

Tools MUST NOT search across workspaces unless `workspace_id` is explicit on the call (or injected from the authenticated claim).

---

## TTL

| Handle | Recommendation | Source of truth |
|--------|----------------|-----------------|
| `retrieval_id` | ~1 hour | Echoed `expires_at` from cache |
| `tools/list` | 1 hour `ttlMs` | Already `TOOLS_LIST_TTL_MS` |

Raising shared cache from 15 minutes to 1 hour affects REST `GET /query/context/{id}`. Implementers MUST either raise TTL and document the REST behavior change, or keep 15 minutes and report honest `expires_at`. **Echoed `expires_at` wins over the recommendation.**

---

## Schema SSOT

JSON Schema files under [`schemas/`](schemas/) are normative. Rust `tools/list` and a conformance test MUST match them. Every `inputSchema` root object MUST set `additionalProperties: false`.
