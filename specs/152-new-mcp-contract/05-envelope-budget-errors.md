# 05 — Envelope, budget, errors

Parent: [README](README.md) · Tools: [04](04-tool-contract.md) · Schema: [schemas/envelope.schema.json](schemas/envelope.schema.json)

## Shared read envelope

Every read tool shares this skeleton. Tool-specific fields (`hits`, `documents`, …) are additional properties validated by that tool’s `outputSchema`.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "required": ["ok", "view", "budget_used", "truncation"],
  "properties": {
    "ok": { "type": "boolean" },
    "view": { "type": "string" },
    "budget_used": { "enum": ["cheap", "standard", "deep"] },
    "hits": { "type": "array" },
    "documents": { "type": "array" },
    "chunks": { "type": "array" },
    "entities": { "type": "array" },
    "relationships": { "type": "array" },
    "citations": { "type": "array" },
    "truncation": {
      "type": "object",
      "required": ["truncated"],
      "properties": {
        "truncated": { "type": "boolean" },
        "next_cursor": { "type": "string" },
        "omitted_chunks": { "type": "integer" },
        "omitted_entities": { "type": "integer" },
        "omitted_relationships": { "type": "integer" }
      }
    },
    "lineage_resources": {
      "type": "array",
      "items": { "type": "string", "pattern": "^eq://" }
    },
    "stats": { "type": "object" },
    "score_type": { "enum": ["unit_interval", "raw"] },
    "error": {
      "type": "object",
      "properties": {
        "code": { "type": "string" },
        "message": { "type": "string" },
        "task_id": { "type": "string" }
      }
    }
  }
}
```

When `ok` is false, MCP CallToolResult MUST set `isError: true`. Prefer typed `error` in `structuredContent` over HTML or unstructured walls of text.

---

## Context budget (hard contract)

Server-enforced in the **projection**, not suggestions. Byte check is on the serialized `structuredContent` JSON (UTF-8), not on engine token estimates alone.

| Budget class | Max chunks | Max entities | Max entity `one_liner` | Target structured JSON |
|--------------|------------|--------------|------------------------|------------------------|
| `cheap` | 5 | 8 | 160 chars | ≤ 8 KiB |
| `standard` (default) | 12 | 16 | 280 chars | ≤ 24 KiB |
| `deep` | 30 | 40 | 800 chars | ≤ 80 KiB |

### Overflow algorithm

1. Include highest-score items first (stable tie-break: id ascending).
2. Set `truncation.truncated = true`.
3. Set `truncation.next_cursor` and `truncation.omitted_{chunks,entities,relationships}` as applicable.
4. MUST NOT rely on an outer gateway to slice JSON mid-object.

If after honest truncation the payload still exceeds the KiB cap → `ok: false`, code `eq/truncated_invalid` (server bug).

A client that receives a truncated body without `truncation` is entitled to treat the result as **invalid**.

`ttlMs` and `cacheScope` SHOULD be set on list/search *tools/list* and list results per MCP 2026-07-28 where applicable.

---

## Summary text channel

| Rule | Normative |
|------|-----------|
| Max length | 2 KiB characters |
| Content | Status + up to 8 item lines + truncation line |
| Forbidden | `serde_json::to_string` of the full structured object |

Replace [`call_tool_result`](../../edgequake/crates/edgequake-api/src/mcp/gateway/dispatch.rs) accordingly.

---

## Typed errors

| Code | When |
|------|------|
| `eq/invalid_id` | Bad `ret_` / entity / doc id / ambiguous filter |
| `eq/not_found` | Missing object |
| `eq/scope_required` | Ambiguous multi-doc query where policy requires scope |
| `eq/not_ready` | Document still ingesting (`task_id` included) |
| `eq/budget_exceeded` | Client asked `deep` on a host that forbids it |
| `eq/truncated_invalid` | Internal generator blew the KiB cap after truncation |
| `eq/forbidden` | Workspace / tenant mismatch |
| `eq/confirm_required` | Destructive call without `confirm: true` |

Map from existing `ApiError` variants where possible (`Gone` → re-search messaging with `eq/not_found` or a dedicated expired retrieval message in `error.message`).

---

## Stats (observability)

Every result SHOULD include:

```json
{
  "embedding_ms": 0,
  "retrieve_ms": 0,
  "rerank_ms": 0,
  "total_ms": 210,
  "items": {
    "chunks": 12,
    "entities": 8,
    "relationships": 4,
    "documents": 1
  },
  "cached": false,
  "fingerprint": "…"
}
```

Host-level logs: tool name, budget, truncated flag, workspace. Fingerprints enable evals: same query + same corpus version ⇒ same fingerprint.
