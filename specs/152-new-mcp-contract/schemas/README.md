# EQ-MCP-1.0 JSON Schemas (SSOT)

Normative JSON Schema 2020-12 for tool `inputSchema` roots and the shared read envelope.

| File | Role |
|------|------|
| [envelope.schema.json](envelope.schema.json) | Shared `structuredContent` skeleton + `$defs` |
| [eq_document_list.input.json](eq_document_list.input.json) | L1 |
| [eq_search.input.json](eq_search.input.json) | L1 |
| [eq_fetch.input.json](eq_fetch.input.json) | L1 |
| [eq_retrieve.input.json](eq_retrieve.input.json) | L1 |
| [eq_entity_search.input.json](eq_entity_search.input.json) | L1 |
| [eq_neighborhood.input.json](eq_neighborhood.input.json) | L1 |
| [eq_workspace_list.input.json](eq_workspace_list.input.json) | L2/catalog |
| [eq_workspace_stats.input.json](eq_workspace_stats.input.json) | L2/catalog |
| [eq_document_get.input.json](eq_document_get.input.json) | L2/catalog |
| [eq_entity_get.input.json](eq_entity_get.input.json) | L2 |
| [eq_ingest.input.json](eq_ingest.input.json) | L3 |
| [eq_task_get.input.json](eq_task_get.input.json) | L3 |
| [eq_document_delete.input.json](eq_document_delete.input.json) | L3 |
| [eq_workspace_delete.input.json](eq_workspace_delete.input.json) | L3 |
| [tools.catalog.json](tools.catalog.json) | Name → schema + annotations index |

**Rules:**

1. Every input root MUST have `"additionalProperties": false`.
2. Rust `mcp/gateway/tools.rs` MUST match these files (conformance test).
3. Tool-specific `outputSchema` = envelope + tool fields from [04-tool-contract](../04-tool-contract.md); full per-tool output schemas MAY be added as `eq_*.output.json` in a follow-up without changing semantics.
