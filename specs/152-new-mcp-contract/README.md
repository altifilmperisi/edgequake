# SPEC-152 — EQ-MCP-1.0 New MCP Contract

> **Status:** Implemented (Phases A–D in tree; query profile default)  
> **Date:** 2026-09-29  
> **Document control:** EQ-MCP-1.0  
> **Protocol:** MCP `2026-07-28` (stateless core, `outputSchema`, `structuredContent`, cursors, annotations, optional Skills/Tasks)  
> **Applies to:** Remote Streamable HTTP gateway (`POST /mcp`) **and** `@edgequake/mcp-server` (stdio bridge)  
> **Supersedes (agent surface):** SPEC-028 three-tool MCP exposure as the *agent* contract. REST query-context DTOs remain; MCP projects them.  
> **Related:** [SPEC-028 MCP suite](../028-edgequake-query-service/mcp/000-index.md), [007 exposure lens](../028-edgequake-query-service/007-mcp-exposure-lens.md), [031 document filter MCP](../031-filter-document/005-mcp-integration.md)

---

## What operators and implementers need to know

| Goal | Do this |
|------|---------|
| Understand why L0 fails agents | Read [00-why](00-why.md) |
| See what ships today | Read [01-current-surfaces](01-current-surfaces.md) |
| Know the target architecture | Read [02-architecture](02-architecture.md) |
| Implement a tool | Read [04-tool-contract](04-tool-contract.md) + [schemas/](schemas/) |
| Enforce budgets / errors | Read [05-envelope-budget-errors](05-envelope-budget-errors.md) |
| Ship in phases | Follow [09-implementation-plan](09-implementation-plan.md) |
| Prove conformance | Run tests in [10-conformance-tests](10-conformance-tests.md) |

---

## Conformance levels

| Level | Label | Minimum surface |
|-------|-------|-----------------|
| **L0** | Query stub (current remote) | `edgequake_search` / `_fetch` / `_retrieve` — **non-conformant**; must be labeled “EdgeQuake query preview” |
| **L1** | Agent-safe read | `eq_document_list`, `eq_search`, `eq_fetch`, `eq_retrieve`, `eq_entity_search`, `eq_neighborhood` + envelope + budget + truncation + summary text + `instructions` |
| **L2** | Graph-RAG | L1 + `eq_entity_get` + typed-edge filters + `eq://` resources + `document_ids` scope + `score_type` |
| **L3** | Memory | L2 + ingest + task poll + deletes with `confirm` + optional Skills |

A connector that only implements L0 MUST NOT be marketed as “EdgeQuake MCP”.

---

## Document map

| # | Document | Read when |
|---|----------|-----------|
| [00-why.md](00-why.md) | First principles and the ten session failures |
| [01-current-surfaces.md](01-current-surfaces.md) | Gateway vs stdio, exact code paths |
| [02-architecture.md](02-architecture.md) | Projection layer, profiles, aliases |
| [03-object-model.md](03-object-model.md) | Document / chunk / entity / URI forms |
| [04-tool-contract.md](04-tool-contract.md) | Twelve tools + deletes |
| [05-envelope-budget-errors.md](05-envelope-budget-errors.md) | Envelope, budgets, `eq/*` errors |
| [06-retrieval-and-graph.md](06-retrieval-and-graph.md) | Modes, scope, scores, graph filters |
| [07-resources-instructions-skills.md](07-resources-instructions-skills.md) | Resources, instructions, Skills |
| [08-security-observability.md](08-security-observability.md) | Tenancy, confirm, stats |
| [09-implementation-plan.md](09-implementation-plan.md) | Phases A–D, file-level |
| [10-conformance-tests.md](10-conformance-tests.md) | Acceptance tests 1–10 |
| [11-migration-risks.md](11-migration-risks.md) | Alias window, TTL, text deviation |

### Artifacts

| Path | Purpose |
|------|---------|
| [schemas/](schemas/) | JSON Schema 2020-12 SSOT (`additionalProperties: false` on inputs) |
| [fixtures/l0-retrieve-vs-l1.md](fixtures/l0-retrieve-vs-l1.md) | Side-by-side L0 vs L1 shape |

---

## One-screen architecture

```text
  Hosts (Grok / Cursor / Claude / Codex)
           │
           ├─ Streamable HTTP ──► POST /mcp  (query profile default)
           └─ stdio ───────────► @edgequake/mcp-server ──► same /mcp tools
                                      │
                                      v
                           mcp/project/  (AgentView projection)
                                      │
              ┌───────────────────────┼───────────────────────┐
              v                       v                       v
     QueryContextService      list_documents /        entity search /
     (search/fetch/retrieve)  workspaces / tasks      neighborhood
```

---

## Normative conventions

- RFC 2119 keywords.
- JSON Schema 2020-12 for all `inputSchema` / `outputSchema`.
- Tool names: `eq_<verb>_<object>` on the official server. Grok MAY prefix `edgequake___`; the semantic name after the host prefix MUST match.
- Compatibility aliases `edgequake_search|fetch|retrieve` for one minor version MUST enforce the new budget.

---

## Out of scope

- Replacing the REST API or Web UI.
- An MCP tool that returns an LLM essay (`query` / `eq_answer`).
- Blocking L1 on extraction retyping of every `RELATED_TO` edge.
- Changing OpenAPI query-context response shapes in Phases A–B.
