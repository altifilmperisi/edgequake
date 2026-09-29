# 09 — Implementation plan

Parent: [README](README.md) · Architecture: [02](02-architecture.md) · Tests: [10](10-conformance-tests.md) · Risks: [11](11-migration-risks.md)

Normative sequence. Each phase is shippable. Keep `cargo test -p edgequake-api --test spec028_mcp_e2e` green via aliases or explicit test updates.

---

## Phase A — Stop the bleeding (L1 start, query profile)

**Goal:** Agents can list documents; fetch stops dumping subgraph + dual JSON; instructions exist.

| # | Change | Files |
|---|--------|-------|
| A1 | Summary text in `call_tool_result` (≤2 KiB; not `to_string(json)`) | `mcp/gateway/dispatch.rs`, new `mcp/project/summary.rs` |
| A2 | `initialize` / `server_discover` include `instructions` (query-only text) | `mcp/gateway/dispatch.rs` |
| A3 | Add `eq_document_list` tool; wire to `list_documents` + opaque cursor over page | `mcp/gateway/tools.rs`, `mcp/gateway/dispatch.rs`, `mcp/gateway/tool_validation.rs`, `mcp/project/catalog.rs` |
| A4 | Default fetch/retrieve `include_subgraph: false` for MCP path | `mcp/gateway/tools.rs`, `dispatch.rs` execute_tool, aliases |
| A5 | Register aliases `edgequake_*` → projection with `budget=standard` | `dispatch.rs`, `tools.rs` (list both names or alias-only dispatch) |
| A6 | Envelope fields on document list + existing tools (`ok`, `view`, `budget_used`, `truncation`) | `mcp/project/envelope.rs` |
| A7 | Update OAuth scope map for `eq_document_list` → `edgequake:read` | `oauth/scopes.rs` |
| A8 | Snapshot / unit tests for tools/list includes `eq_document_list` | `mcp/gateway/tools.rs` tests, `spec028_mcp_e2e` adjust |

**Exit:** Acceptance tests 1, 4 (partial — summary not clone), 10. Grok can list four PDFs without RAG.

**Do not** change REST search cardinality yet.

---

## Phase B — Hits, views, budgets

**Goal:** Search returns hits; fetch is incremental; KiB budgets enforced.

| # | Change | Files |
|---|--------|-------|
| B1 | `eq_search` projects `hits[]` from cached bundle; advertise `document_ids` | `mcp/project/search.rs`, `tools.rs`, `tool-schemas` sync from `specs/152/schemas` |
| B2 | Keep REST `search_context` one-row behavior **or** add internal `retrieve_for_mcp` shared helper without breaking OpenAPI | `services/query_context.rs` (prefer thin wrapper used only by MCP) |
| B3 | `eq_fetch` views: `toc\|chunks\|entities\|citations\|full`; id subset | `mcp/project/fetch.rs` |
| B4 | `budget.rs` item + KiB caps; cursor resume; `eq/truncated_invalid` | `mcp/project/budget.rs` |
| B5 | `mode_used` / `mode_reason` from `ModeSelection`; `cross_document`; `expires_at` | search/fetch projectors |
| B6 | `eq_retrieve` = search + fetch chunks; keep `hits` | `dispatch.rs` |
| B7 | `score_type` policy | `mcp/project/scores.rs` |
| B8 | Conformance tests 2, 3, 5, 6, 8, 9 (fixtures) | `tests/spec152_mcp_conformance.rs` |

**Exit:** L1 retrieval complete on remote query profile.

**TTL decision:** Either raise cache to 1h + document REST impact, or keep 15m and echo honest `expires_at` ([11](11-migration-risks.md)).

---

## Phase C — Compact graph + catalog completeness

**Goal:** L2-ready graph tools; entity compactness; workspace catalog.

| # | Change | Files |
|---|--------|-------|
| C1 | `eq_entity_search`, `eq_neighborhood`, `eq_entity_get` | `mcp/project/graph.rs`, dispatch, tools |
| C2 | Artifact / weak-edge filters; top-k neighbors; `one_liner` caps | graph projector |
| C3 | `eq_workspace_list`, `eq_workspace_stats`, `eq_document_get` | `catalog.rs` |
| C4 | Entity id normalize `ent:{ws}:{slug}` | `mcp/project/ids.rs` |
| C5 | Acceptance test 7 | conformance suite |
| C6 | Optional: start typed-edge extraction follow-on ticket (pipeline) | out of MCP crate |

**Exit:** L2 minus resources.

---

## Phase D — Resources, stdio alignment, L3

**Goal:** Full contract surfaces; one semantic SSOT.

| # | Change | Files |
|---|--------|-------|
| D1 | `resources/list` + `resources/read` for `eq://` | new `mcp/gateway/resources.rs`, capabilities on initialize |
| D2 | Rewrite `@edgequake/mcp-server` as stdio → `/mcp` bridge | `mcp/src/server.ts`, tools rewrite, drop LLM `query` from default |
| D3 | Update `mcp/tests/server.test.ts` expected names to `eq_*` | tests |
| D4 | `eq_ingest`, `eq_task_get`, deletes with `confirm` | memory profile flag / scope |
| D5 | Optional Skills `explain-paper` | `mcp/skills/…` |
| D6 | Full conformance 1–10 in CI; sync `specs/028/.../mcp/tool-schemas.json` or replace pointer to SPEC-152 schemas | CI + docs |
| D7 | Alias deprecation notice in tool descriptions | tools.rs |

**Exit:** L3 available on memory profile; remote default stays query (L1/L2).

---

## Suggested calendar (90 days)

| Weeks | Phase | Impact |
|-------|-------|--------|
| 1–2 | A | List + no dual JSON + subgraph off |
| 3–5 | B | Hits, views, budget, truncation |
| 6–8 | C | Entity compact, neighborhood, scope polish |
| 9–12 | D | Resources, stdio, Skills, CI conformance |

Matches impact order from the motivating session: payload shape and missing list/scope tools first.

---

## Code ownership checklist

| Concern | Owner module |
|---------|--------------|
| JSON-RPC / headers / auth | `mcp/gateway/*` |
| Agent envelope / budget / hits | `mcp/project/*` |
| Ranking / modes / cache | `services/query_context.rs`, `edgequake-query` |
| Document list pagination | existing document handlers |
| Schema SSOT | `specs/152-new-mcp-contract/schemas/` |
| Stdio UX | `mcp/` bridge only |

---

## Explicit non-work

- Do not add `eq_answer`.
- Do not change Web UI for this spec.
- Do not block Phase A/B on extraction retyping.
- Do not change OpenAPI `ContextSearchResponse` cardinality in Phase A (MCP-only projection).
