# 01 — Current surfaces (as-is)

Parent: [README](README.md) · Why: [00](00-why.md) · Next: [02-architecture](02-architecture.md)

EdgeQuake ships **two** MCP surfaces today. Neither conforms to EQ-MCP-1.0. This document names the exact code so the replacement plan is not folklore.

---

## Surface A — Remote Streamable HTTP gateway (L0)

| Item | Value |
|------|-------|
| Mount | `POST /mcp`, alias `POST /api/v1/mcp` |
| Catalog | [`mcp/gateway/tools.rs`](../../edgequake/crates/edgequake-api/src/mcp/gateway/tools.rs) |
| Dispatch | [`mcp/gateway/dispatch.rs`](../../edgequake/crates/edgequake-api/src/mcp/gateway/dispatch.rs) |
| Services | [`services/query_context.rs`](../../edgequake/crates/edgequake-api/src/services/query_context.rs) |
| Auth | [`mcp/auth/gateway_auth.rs`](../../edgequake/crates/edgequake-api/src/mcp/auth/gateway_auth.rs) |
| Scopes | [`oauth/scopes.rs`](../../edgequake/crates/edgequake-api/src/oauth/scopes.rs) |

### Tools advertised

`edgequake_search`, `edgequake_fetch`, `edgequake_retrieve` only.

### Search cardinality defect

`search_context` always runs a full `retrieve_context` at Agent granularity with `include_subgraph: true`, caches the bundle, then returns **a single** `ContextSearchResult`:

```281:349:edgequake/crates/edgequake-api/src/services/query_context.rs
pub async fn search_context(...) -> ApiResult<ContextSearchResponse> {
    // ... builds ContextRetrievalRequest with include_subgraph: true ...
    let response = retrieve_context(...).await?;
    Ok(ContextSearchResponse {
        results: vec![ContextSearchResult {
            retrieval_id: response.retrieval_id.clone(),
            title,   // first 80 chars of first chunk or first entity
            snippet, // first 200 chars of first chunk
            score: response.retrieval_quality.coverage_score,
            // ...
        }],
    })
}
```

`max_results: 50` therefore still yields **one** opaque retrieval session. That is non-conformant under EQ-MCP-1.0 §5.2 / acceptance test 5.

### Double JSON in CallToolResult

```168:175:edgequake/crates/edgequake-api/src/mcp/gateway/dispatch.rs
fn call_tool_result(structured: Value) -> Value {
    let text = serde_json::to_string(&structured).unwrap_or_else(|_| "{}".into());
    json!({
        "content": [{ "type": "text", "text": text }],
        "structuredContent": structured
    })
}
```

MCP 2026-07-28 says servers that return `structuredContent` SHOULD also put serialized JSON in a text block. Agents that read both pay twice; agents that only read text still get megabytes. EQ-MCP overrides this SHOULD ([05](05-envelope-budget-errors.md)).

### Missing capabilities

| Capability | Status |
|------------|--------|
| `initialize.instructions` | Absent — `legacy_initialize` returns tools capability only |
| `resources/list`, `resources/read` | Absent (Method not found) |
| `eq_document_list` / workspace tools | Absent |
| `document_ids` in MCP `inputSchema` | Omitted in `tools.rs`; present on REST `DocumentFilter` |
| Fetch default `include_subgraph` | `true` |
| KiB budget / agent `truncation` cursor | Absent; engine `TruncationInfo.dropped` stays zeros |
| Retrieval TTL | 15 minutes (`retrieval_id_cache.rs`); no `expires_at` on search output |

### What already works (reuse)

| Asset | Location | Reuse for |
|-------|----------|-----------|
| `DocumentFilter.document_ids` | `handlers/query_types.rs` + resolver | Scope without new engine work |
| `ModeSelection` | `context_types.rs` | `mode_used` / `mode_reason` |
| `TruncationInfo` / `DroppedCounts` | `context_types.rs` | Shape; fill counts in projection |
| `ContentGranularity` | citation / agent / debug | citation ≈ snippet; agent views replace debug for MCP |
| Document list REST | `GET /api/v1/documents` | `eq_document_list` |
| Entity neighborhood | `GET /api/v1/graph/entities/{name}/neighborhood` | `eq_neighborhood` |
| Artifact fetch | `GET /api/v1/query/context/artifacts/...` | `eq://` hydration |
| Task poll | `GET /api/v1/tasks/{track_id}` | `eq_task_get` |

### Tests locked to L0 names

- `tests/spec028_mcp_e2e.rs`
- `tests/spec028_mcp_oauth_e2e.rs`
- `tests/spec028_mcp_transport.rs`
- `tests/spec028_api_contract.rs` (source string asserts)

Aliases MUST keep these green during the compatibility window.

---

## Surface B — `@edgequake/mcp-server` (stdio)

| Item | Value |
|------|-------|
| Package | `mcp/` — `@edgequake/mcp-server` `0.2.0`, bin `edgequake-mcp` |
| Registration | [`mcp/src/server.ts`](../../mcp/src/server.ts) |
| Spec doc | [`mcp/docs/SPEC.md`](../../mcp/docs/SPEC.md) |
| SDK pin | `edgequake-sdk@^0.1.0` (repo SDK is ahead) |

### Tool inventory (17)

`health`, `workspace_{list,create,get,delete,stats}`, `document_{upload,upload_file,list,get,delete,status}`, `query`, `graph_search_entities`, `graph_get_entity`, `graph_entity_neighborhood`, `graph_search_relationships`.

### Defects vs EQ-MCP-1.0

| Defect | Evidence |
|--------|----------|
| No `outputSchema` / `structuredContent` / annotations | Every handler returns `content: [{ type: "text", text: JSON.stringify(...) }]` |
| No `eq_search` / `eq_fetch` / `eq_retrieve` | Uses LLM `query` → `POST /api/v1/query` instead |
| No budget / truncation / hits | Absent |
| Deletes without `confirm` | `workspace_delete`, `document_delete` |
| Page pagination, not cursors | `document_list` uses `page` / `page_size` |
| No server `instructions` | `new McpServer({ name, version })` only |
| `document_list.search` likely noop | SDK `ListDocumentsQuery` has no `search` field |

### What already maps

| TS tool | EQ-MCP target |
|---------|---------------|
| `workspace_list` | `eq_workspace_list` |
| `workspace_stats` | `eq_workspace_stats` |
| `document_list` | `eq_document_list` |
| `document_get` | `eq_document_get` |
| `document_upload` (+ file) | `eq_ingest` |
| `graph_search_entities` | `eq_entity_search` |
| `graph_get_entity` | `eq_entity_get` |
| `graph_entity_neighborhood` | `eq_neighborhood` |
| `document_status` | partial → prefer `eq_task_get` |
| `query` | **out of contract** (LLM essay) |

Resources today use `edgequake://workspace/{id}/…` (markdown). EQ-MCP uses `eq://{ws}/…` with structured reads.

Tests that lock names: [`mcp/tests/server.test.ts`](../../mcp/tests/server.test.ts) expected array of 17 names.

---

## SPEC-028 relationship

SPEC-028 delivered transport SOTA (Streamable HTTP, OAuth PRM, `outputSchema` stubs, `_meta`). It did **not** deliver agent-safe grain:

- Search = one session handle, not hits.
- Fetch = full `ContextBundle` by default.
- No catalog tools on the remote connector.
- Text channel duplicates structured JSON.

EQ-MCP-1.0 keeps SPEC-028 transport and OAuth; it replaces the **tool semantics and projection**.

---

## Gap summary (must close)

```text
L0 remote                          EQ-MCP L1+
──────────                         ──────────
3 mega-tools                       6+ typed tools (query profile)
1 search "hit"                     hits[] up to limit
subgraph on by default             subgraph off; views
JSON text == structured            ≤2 KiB summary text
no document list                   eq_document_list
document_pattern only in schema    document_ids primary
no instructions                    initialize.instructions
no resources                       eq:// resources (L2)
15m TTL silent                     expires_at echoed
```
