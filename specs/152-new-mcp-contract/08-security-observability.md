# 08 — Security and observability

Parent: [README](README.md) · Architecture: [02](02-architecture.md) · Errors: [05](05-envelope-budget-errors.md)

## Tenancy and workspace

- `workspace_id` travels as logical header `Workspace-Id` (wire: `X-Workspace-Id` / `Mcp-Param-Workspace-Id`). Keep current injection in dispatch.
- Tools MUST NOT search across workspaces unless `workspace_id` is explicit (arg or injected claim).
- `enforce_workspace_claim` remains: authenticated claim and arg MUST match when both set.
- Cross-tenant access → `eq/forbidden`.

The workspace is both a **security** boundary and a **relevance** boundary (LAW-152-6). Unscoped multi-PDF retrieval without `cross_document` is a defect.

---

## Auth and scopes

| Scope | Methods / tools |
|-------|-----------------|
| `edgequake:read` | `tools/list`, `initialize`, `ping`, fetch, catalog, entity get/neighborhood, `resources/*` |
| `edgequake:query` | `eq_search`, `eq_retrieve`, `eq_entity_search` (+ aliases) |
| `edgequake:write` | `eq_ingest`, `eq_task_get`, deletes |

Query profile deployments SHOULD omit write scope grants even if tools are compiled out.

OAuth PRM / CIMD / Streamable HTTP hardening from SPEC-028 MCP suite remain in force.

Never pass third-party LLM API keys through the model. URL-mode elicitation for auth to external keys only.

---

## Untrusted content

Ingest of user text is untrusted content. Results that echo document bytes SHOULD set an `untrustedContentHint` (or equivalent content annotation) when the host/protocol supports it (WebMCP / MCP annotation direction).

Projection MUST NOT treat chunk text as instructions for the server.

---

## Destructive operations

| Tool | Requirement |
|------|-------------|
| `eq_document_delete` | `confirm: true` or `eq/confirm_required` |
| `eq_workspace_delete` | `confirm: true` or `eq/confirm_required` |

Hosts that support elicitation SHOULD prompt the user before confirming.

Do **not** ship write tools on a read-only Grok connector.

---

## Retrieval ids as capability tokens

- `retrieval_id` is scoped to the workspace that created it.
- MUST expire; echo `expires_at`.
- Recommendation: 1 hour; **reported `expires_at` wins** ([02](02-architecture.md)).
- Expired → client re-runs `eq_search` (message MUST say so).

---

## Observability

### Per-result `stats`

See [05](05-envelope-budget-errors.md): timings, item counts, `cached`, `fingerprint`.

### Host / server logs (SHOULD)

| Field | Example |
|-------|---------|
| tool | `eq_fetch` |
| budget | `standard` |
| truncated | `true` |
| workspace | UUID |
| retrieval_id | `ret_…` |
| latency_ms | 210 |

### Audit (from SPEC-028 backlog)

Prefer logging tool name + `retrieval_id` + workspace **without** full chunk content (PII / corpus leakage).

---

## Rate limits and body size

Existing gateway body cap (`MCP_MAX_BODY_BYTES` = 1 MiB) remains for requests. Response KiB budgets are stricter and agent-facing ([05](05-envelope-budget-errors.md)).
