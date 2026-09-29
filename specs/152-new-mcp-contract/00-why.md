# 00 — Why EQ-MCP-1.0

Parent: [README](README.md) · Next: [01-current-surfaces](01-current-surfaces.md)

## The job to be done

An agent must list, scope, retrieve, and traverse a Graph-RAG workspace **without drowning**. Every tool result exists so the model can choose the **next decision**. If the result cannot be scanned in one glance, the grain is wrong.

This is a **contract**, not a feature list. It is derived from first principles, from using the current three-tool connector on a four-document workspace, and from how agents burn context in 2026.

---

## First principles (LAW-152)

### LAW-152-1 — The unit of intelligence is the next decision

A tool result exists so the model can choose the next action. Scannable grain beats completeness.

### LAW-152-2 — Tokens are the scarce resource, not endpoints

Tool descriptions, schemas, and results all sit in the context window. A 280 KiB retrieve that a gateway then cuts mid-object is a failed call.

### LAW-152-3 — Names are capabilities; payloads are evidence; graphs are hypotheses

Catalog tools name what exists. Retrieval returns evidence with lineage. Graph tools return structure the model must not infer from prose.

### LAW-152-4 — Search returns handles; fetch returns bodies

Combining them is allowed (`eq_retrieve`). Defaulting to the combined megapayload is not.

### LAW-152-5 — Silence is a bug

Truncation, cross-document bleed, mode fallback, and cache hits MUST be explicit fields. Agents cannot compensate for omitted metadata.

### LAW-152-6 — The workspace is a security and relevance boundary

Isolation is tenancy. Unrelated PDFs in one workspace are a retrieval defect unless scope is declared.

### LAW-152-7 — Write and read are different trust classes

Ingest, delete, and workspace destroy are consequential. Query is not. Annotations and confirmations MUST follow that split.

### LAW-152-8 — The MCP layer is not the REST API

Tool grain follows the *job* (“list docs”, “explain this entity”), not `/api/v1/*` 1:1. REST stays wide; MCP stays small and typed.

---

## 5-WHY — Why the SoL-Pi explanation failed in-session

| # | Question | Answer |
|---|----------|--------|
| 1 | Why did the agent drown explaining one mechanism? | Tool results were hundreds of KiB of chunks + subgraph with figure/DRAWING hubs. |
| 2 | Why were payloads so large? | `include_subgraph` defaults true; `content_granularity=agent` returns full text; no MCP KiB budget. |
| 3 | Why couldn't the agent list the four PDFs? | Remote gateway has no `eq_document_list`; “list docs” went through semantic RAG. |
| 4 | Why did another paper bleed into the answer? | Search schema omits `document_ids`; pattern-only filter is easy to miss; `cross_document` is silent. |
| 5 | **Root cause** | **Agent surface = REST megabundle + missing catalog/scope tools.** Modes exist; payload shape and list/scope do not. |

```text
  User: "explain Action Fusion in sol_pi PDF"
         │
         v
  edgequake_retrieve (L0) ──► one ContextBundle
         │                      chunks + full subgraph + figure essays
         v
  Host / model context ──► truncated mid-JSON or 200k tokens
         │
         v
  Agent invents document list / mixes ten_*.pdf chunks
```

---

## Ten failures that MUST become tests

These failed in the motivating session and are normative acceptance criteria ([10-conformance-tests](10-conformance-tests.md)):

1. **List** — No way to return exactly the workspace PDFs without a semantic query.
2. **Scope** — Chunks from an unrelated `file_name` appear when the user named one paper.
3. **Budget** — Structured JSON exceeds 24 KiB on `budget=standard` without honest truncation.
4. **Double encode** — `content[0].text` is a clone of `structuredContent`.
5. **Search cardinality** — `limit=8` returns one opaque session, not up to 8 hits.
6. **Fetch incrementality** — Fetch of one hit id resends the whole subgraph.
7. **Entity compactness** — Default entity text is a page-long vision dump.
8. **Mode silence** — Requested `local` is swapped without `mode_used` / `mode_reason`.
9. **Cursor resume** — Second page duplicates chunk ids.
10. **Empty workspace** — List invents a graph instead of `items: []`.

---

## Goals and non-goals

### Goals

- Let an agent list, scope, retrieve, and traverse without drowning.
- Make every result validatable against an `outputSchema`.
- Keep default responses under a hard token / KiB budget.
- Preserve lineage (document + chunk + page).
- Align with MCP 2026-07-28: structured content, resource URIs, cursors, annotations.

### Non-goals

- Replacing REST or the Web UI.
- Generating the final user-facing answer inside the MCP server.
- Hiding LightRAG modes — they stay as *declared strategy*.
- Blocking L1 on corpus re-extraction for typed edges.
