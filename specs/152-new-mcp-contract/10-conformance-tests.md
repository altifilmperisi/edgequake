# 10 — Conformance tests

Parent: [README](README.md) · Plan: [09](09-implementation-plan.md) · Fixture: [fixtures/l0-retrieve-vs-l1.md](fixtures/l0-retrieve-vs-l1.md)

## Runner

| Suite | Command / location |
|-------|-------------------|
| Gateway conformance | `edgequake/crates/edgequake-api/tests/spec152_mcp_conformance.rs` |
| Existing transport/oauth | Keep `spec028_mcp_*` green via aliases |
| Stdio package | `cd mcp && bun test` / vitest after rename |
| Schema SSOT | Unit test: `tools/list` JSON matches `specs/152-new-mcp-contract/schemas/` |

Tests 2 and 7 MUST use fixture documents or query-context test doubles — **not** a live four-PDF workspace.

---

## Acceptance tests (normative)

### T1 — List

**Given** a workspace with exactly four completed PDF documents.  
**When** `eq_document_list` with default limit.  
**Then** `ok=true`, `documents`/`items` length 4, each has `id` and `file_name` or `title`, no semantic `query` required.  
**Phase:** A

### T2 — Scope

**Given** two docs `sol_pi_….pdf` and `ten_….pdf` with distinct chunk texts.  
**When** `eq_search` / `eq_retrieve` with `document_ids=[sol_pi_id]` and query about a mechanism only in sol_pi.  
**Then** zero returned chunks have `file_name == ten_….pdf`.  
**Phase:** B

### T3 — Budget

**When** `eq_retrieve(..., budget=standard)` on a large fixture bundle.  
**Then** UTF-8 byte length of `structuredContent` ≤ 24 KiB, and if items were omitted `truncation.truncated=true` with omitted counts consistent (≥ omitted, scores of included ≥ excluded).  
**Phase:** B

### T4 — No double encode

**When** any successful tool call.  
**Then** `content[0].text` is not equal to `serde_json::to_string(structuredContent)`, length ≤ 2048, and does not start with `{` containing the full hits array dump. Prefer: text is summary template; structured parses as envelope.  
**Phase:** A

### T5 — Search cardinality

**When** `eq_search(limit=8)` against a bundle with ≥8 chunks.  
**Then** `hits.len() <= 8` and `hits.len() >= 1` (when corpus non-empty), and `hits` are first-class objects with `kind`+`id` — not a single opaque session masquerading as the only result.  
**Phase:** B  
**Note:** One `retrieval_id` for the session is required; many hits under it.

### T6 — Fetch incrementality

**Given** a `retrieval_id` from search with multiple hits.  
**When** `eq_fetch(view=chunks, ids=[top_hit_id], include_subgraph=false)`.  
**Then** response does not include the full subgraph entity set from the cached bundle (entities empty or only those required for the single chunk’s lineage).  
**Phase:** B

### T7 — Entity compactness

**When** default entity list from search metadata / `eq_entity_search` / fetch `view=entities` at `budget=standard`.  
**Then** every `one_liner` / compact description ≤ 280 chars; no page-long vision dump.  
**Phase:** C

### T8 — Mode echo

**When** request `mode=local`.  
**Then** `mode_used == "local"` OR `mode_reason` explicitly describes override.  
**Phase:** B

### T9 — Cursor resume

**Given** a truncated fetch with `next_cursor`.  
**When** second fetch with that cursor.  
**Then** union of chunk ids has no duplicates; omitted counts decrease or cursor null when exhausted.  
**Phase:** B

### T10 — Empty workspace

**Given** workspace with zero documents.  
**When** `eq_document_list`.  
**Then** `items`/`documents` is `[]`, `ok=true`, no fabricated entities/chunks.  
**Phase:** A

---

## Schema conformance

| Check | Assert |
|-------|--------|
| Input schemas | `additionalProperties === false` |
| Output | `structuredContent` validates tool `outputSchema` |
| Annotations | All four hints present on every tool |
| Alias budget | `edgequake_retrieve` response respects standard KiB cap |

---

## Mapping from L0 failures

| Session failure | Test |
|-----------------|------|
| No document list | T1, T10 |
| Cross-doc bleed | T2 |
| 280k / uncapped | T3 |
| Dual JSON | T4 |
| One search row | T5 |
| Fetch resends graph | T6 |
| Figure essays | T7 |
| Silent mode swap | T8 |
| Bad pagination | T9 |
