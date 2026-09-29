//! MCP tool catalog SSOT (SPEC-152 / specs/152-new-mcp-contract/schemas).

use serde_json::{json, Value};

use crate::mcp::project::profile::{mcp_profile, McpProfile};

pub const TOOLS_LIST_TTL_MS: u64 = 3_600_000;

const ALIAS_NOTE: &str =
    " Alias of the EQ-MCP-1.0 tool; budget=standard enforced. Prefer eq_* names.";

/// Build tools/list result with caching metadata (SEP-2549).
pub fn tools_list_result() -> Value {
    let mut tools = vec![
        eq_document_list_tool(),
        eq_search_tool(),
        eq_fetch_tool(),
        eq_retrieve_tool(),
        eq_entity_search_tool(),
        eq_neighborhood_tool(),
        eq_workspace_list_tool(),
        eq_workspace_stats_tool(),
        eq_document_get_tool(),
        eq_entity_get_tool(),
        // Compatibility aliases (one minor version)
        edgequake_search_alias(),
        edgequake_fetch_alias(),
        edgequake_retrieve_alias(),
    ];

    if mcp_profile() == McpProfile::Memory {
        tools.extend([
            eq_ingest_tool(),
            eq_task_get_tool(),
            eq_document_delete_tool(),
            eq_workspace_delete_tool(),
        ]);
    }

    json!({
        "tools": tools,
        "ttlMs": TOOLS_LIST_TTL_MS,
        "cacheScope": "public"
    })
}

fn annotations_ro() -> Value {
    json!({
        "readOnlyHint": true,
        "destructiveHint": false,
        "idempotentHint": true,
        "openWorldHint": false
    })
}

fn envelope_output() -> Value {
    json!({
        "type": "object",
        "required": ["ok", "view", "budget_used", "truncation"],
        "properties": {
            "ok": { "type": "boolean" },
            "view": { "type": "string" },
            "budget_used": { "enum": ["cheap", "standard", "deep"] },
            "truncation": {
                "type": "object",
                "required": ["truncated"],
                "properties": {
                    "truncated": { "type": "boolean" },
                    "next_cursor": { "type": "string" },
                    "omitted_chunks": { "type": "integer" },
                    "omitted_entities": { "type": "integer" }
                }
            },
            "hits": { "type": "array" },
            "documents": { "type": "array" },
            "chunks": { "type": "array" },
            "entities": { "type": "array" },
            "stats": { "type": "object" }
        }
    })
}

fn eq_document_list_tool() -> Value {
    json!({
        "name": "eq_document_list",
        "description": "List documents in the workspace. Use this before answering what is in EdgeQuake — do not invent document lists via search. Empty workspace returns items: [].",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "workspace_id": { "type": "string", "x-mcp-header": "Workspace-Id" },
                "status": { "type": "string", "enum": ["pending", "processing", "completed", "failed"] },
                "query": { "type": "string", "description": "Title/file_name filter, not semantic RAG" },
                "date_from": { "type": "string", "format": "date-time" },
                "date_to": { "type": "string", "format": "date-time" },
                "limit": { "type": "integer", "minimum": 1, "maximum": 50, "default": 20 },
                "cursor": { "type": "string" },
                "budget": { "enum": ["cheap", "standard", "deep"], "default": "standard" }
            }
        },
        "outputSchema": envelope_output(),
        "annotations": annotations_ro()
    })
}

fn eq_search_tool() -> Value {
    json!({
        "name": "eq_search",
        "description": "Search Graph-RAG; returns hits[] (handles) + retrieval_id. Modes: naive=dense chunks; local=entity neighborhood; global=themes; hybrid=local∪global; mix=fused+rerank (default). Prefer document_ids when the user names a paper. Do not use for listing documents.",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "required": ["query"],
            "properties": {
                "query": { "type": "string" },
                "mode": { "type": "string", "enum": ["naive", "local", "global", "hybrid", "mix"], "default": "mix" },
                "scope": { "type": "string", "enum": ["workspace", "documents"], "default": "workspace" },
                "document_ids": { "type": "array", "items": { "type": "string" } },
                "document_pattern": { "type": "string" },
                "document_filter": {
                    "type": "object",
                    "properties": {
                        "date_from": { "type": "string", "format": "date-time" },
                        "date_to": { "type": "string", "format": "date-time" },
                        "document_pattern": { "type": "string" },
                        "document_ids": { "type": "array", "items": { "type": "string" } }
                    }
                },
                "budget": { "enum": ["cheap", "standard", "deep"], "default": "standard" },
                "limit": { "type": "integer", "minimum": 1, "maximum": 50, "default": 8 },
                "max_results": { "type": "integer", "minimum": 1, "maximum": 50 },
                "cursor": { "type": "string" },
                "workspace_id": { "type": "string", "x-mcp-header": "Workspace-Id" },
                "include_artifacts": { "type": "boolean", "default": false }
            }
        },
        "outputSchema": envelope_output(),
        "annotations": annotations_ro()
    })
}

fn eq_fetch_tool() -> Value {
    json!({
        "name": "eq_fetch",
        "description": "Fetch bodies for a retrieval_id. Default view=toc, include_subgraph=false. Views: toc|chunks|entities|citations|full. Pass ids to fetch one hit without the whole subgraph.",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "required": ["retrieval_id"],
            "properties": {
                "retrieval_id": { "type": "string", "pattern": "^ret_" },
                "ids": { "type": "array", "items": { "type": "string" } },
                "view": { "type": "string", "enum": ["toc", "chunks", "entities", "citations", "full"], "default": "toc" },
                "budget": { "enum": ["cheap", "standard", "deep"], "default": "standard" },
                "include_subgraph": { "type": "boolean", "default": false },
                "include_artifacts": { "type": "boolean", "default": false },
                "include_weak_edges": { "type": "boolean", "default": false },
                "cursor": { "type": "string" },
                "content_granularity": { "type": "string", "enum": ["citation", "agent", "debug"], "default": "agent" }
            }
        },
        "outputSchema": envelope_output(),
        "annotations": annotations_ro()
    })
}

fn eq_retrieve_tool() -> Value {
    json!({
        "name": "eq_retrieve",
        "description": "Sugar: eq_search then eq_fetch(view=chunks). Still returns hits so you can fetch more without re-searching. Prefer search→fetch for incremental work.",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "required": ["query"],
            "properties": {
                "query": { "type": "string" },
                "mode": { "type": "string", "enum": ["naive", "local", "global", "hybrid", "mix"], "default": "mix" },
                "document_ids": { "type": "array", "items": { "type": "string" } },
                "document_pattern": { "type": "string" },
                "document_filter": { "type": "object" },
                "budget": { "enum": ["cheap", "standard", "deep"], "default": "standard" },
                "limit": { "type": "integer", "minimum": 1, "maximum": 50, "default": 8 },
                "max_results": { "type": "integer", "minimum": 1, "maximum": 50 },
                "workspace_id": { "type": "string", "x-mcp-header": "Workspace-Id" },
                "include_subgraph": { "type": "boolean", "default": false },
                "enable_rerank": { "type": "boolean", "default": true },
                "content_granularity": { "type": "string", "enum": ["citation", "agent", "debug"] }
            }
        },
        "outputSchema": envelope_output(),
        "annotations": annotations_ro()
    })
}

fn eq_entity_search_tool() -> Value {
    json!({
        "name": "eq_entity_search",
        "description": "Search entities (compact one_liner). Artifacts/DRAWING hidden unless include_artifacts=true.",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "required": ["q"],
            "properties": {
                "q": { "type": "string" },
                "type": { "type": "string" },
                "document_ids": { "type": "array", "items": { "type": "string" } },
                "include_artifacts": { "type": "boolean", "default": false },
                "limit": { "type": "integer", "minimum": 1, "maximum": 50, "default": 10 },
                "budget": { "enum": ["cheap", "standard", "deep"], "default": "standard" },
                "workspace_id": { "type": "string", "x-mcp-header": "Workspace-Id" }
            }
        },
        "outputSchema": envelope_output(),
        "annotations": annotations_ro()
    })
}

fn eq_neighborhood_tool() -> Value {
    json!({
        "name": "eq_neighborhood",
        "description": "Typed neighborhood around an entity. max_hops 1–2 (3 only on budget=deep). RELATED_TO hidden unless include_weak_edges.",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "required": ["entity_id"],
            "properties": {
                "entity_id": { "type": "string" },
                "max_hops": { "type": "integer", "minimum": 1, "maximum": 3, "default": 1 },
                "edge_types": { "type": "array", "items": { "type": "string" } },
                "document_ids": { "type": "array", "items": { "type": "string" } },
                "include_artifacts": { "type": "boolean", "default": false },
                "include_weak_edges": { "type": "boolean", "default": false },
                "budget": { "enum": ["cheap", "standard", "deep"], "default": "standard" },
                "workspace_id": { "type": "string", "x-mcp-header": "Workspace-Id" }
            }
        },
        "outputSchema": envelope_output(),
        "annotations": annotations_ro()
    })
}

fn eq_workspace_list_tool() -> Value {
    json!({
        "name": "eq_workspace_list",
        "description": "List workspaces the caller can see. No default workspace invention.",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "limit": { "type": "integer", "minimum": 1, "maximum": 100, "default": 20 },
                "cursor": { "type": "string" },
                "budget": { "enum": ["cheap", "standard", "deep"], "default": "standard" }
            }
        },
        "outputSchema": envelope_output(),
        "annotations": annotations_ro()
    })
}

fn eq_workspace_stats_tool() -> Value {
    json!({
        "name": "eq_workspace_stats",
        "description": "Workspace counts: documents, entities, relationships, chunks.",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "workspace_id": { "type": "string", "x-mcp-header": "Workspace-Id" },
                "budget": { "enum": ["cheap", "standard", "deep"], "default": "standard" }
            }
        },
        "outputSchema": envelope_output(),
        "annotations": annotations_ro()
    })
}

fn eq_document_get_tool() -> Value {
    json!({
        "name": "eq_document_get",
        "description": "Document metadata by default. include text returns eq:// resource link, not PDF bytes.",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "required": ["document_id"],
            "properties": {
                "document_id": { "type": "string" },
                "include": {
                    "type": "array",
                    "items": { "enum": ["metadata", "outline", "text"] },
                    "default": ["metadata"]
                },
                "workspace_id": { "type": "string", "x-mcp-header": "Workspace-Id" },
                "budget": { "enum": ["cheap", "standard", "deep"], "default": "standard" }
            }
        },
        "outputSchema": envelope_output(),
        "annotations": annotations_ro()
    })
}

fn eq_entity_get_tool() -> Value {
    json!({
        "name": "eq_entity_get",
        "description": "Full entity description (budget-capped) plus lineage resource URI.",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "required": ["entity_id"],
            "properties": {
                "entity_id": { "type": "string" },
                "budget": { "enum": ["cheap", "standard", "deep"], "default": "standard" },
                "workspace_id": { "type": "string", "x-mcp-header": "Workspace-Id" }
            }
        },
        "outputSchema": envelope_output(),
        "annotations": annotations_ro()
    })
}

fn eq_ingest_tool() -> Value {
    json!({
        "name": "eq_ingest",
        "description": "Ingest text or upload_ref. Returns document_id + task_id. Poll with eq_task_get.",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "content": { "type": "string" },
                "title": { "type": "string" },
                "upload_ref": { "type": "string" },
                "workspace_id": { "type": "string", "x-mcp-header": "Workspace-Id" }
            }
        },
        "outputSchema": envelope_output(),
        "annotations": {
            "readOnlyHint": false,
            "destructiveHint": false,
            "idempotentHint": false,
            "openWorldHint": true
        }
    })
}

fn eq_task_get_tool() -> Value {
    json!({
        "name": "eq_task_get",
        "description": "Poll ingest/task status by task_id (track id).",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "required": ["task_id"],
            "properties": { "task_id": { "type": "string" } }
        },
        "outputSchema": envelope_output(),
        "annotations": annotations_ro()
    })
}

fn eq_document_delete_tool() -> Value {
    json!({
        "name": "eq_document_delete",
        "description": "Delete a document. MUST pass confirm: true.",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "required": ["document_id", "confirm"],
            "properties": {
                "document_id": { "type": "string" },
                "confirm": { "type": "boolean" },
                "workspace_id": { "type": "string", "x-mcp-header": "Workspace-Id" }
            }
        },
        "outputSchema": envelope_output(),
        "annotations": {
            "readOnlyHint": false,
            "destructiveHint": true,
            "idempotentHint": false,
            "openWorldHint": false
        }
    })
}

fn eq_workspace_delete_tool() -> Value {
    json!({
        "name": "eq_workspace_delete",
        "description": "Delete a workspace. MUST pass confirm: true.",
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "required": ["workspace_id", "confirm"],
            "properties": {
                "workspace_id": { "type": "string" },
                "confirm": { "type": "boolean" }
            }
        },
        "outputSchema": envelope_output(),
        "annotations": {
            "readOnlyHint": false,
            "destructiveHint": true,
            "idempotentHint": false,
            "openWorldHint": false
        }
    })
}

fn edgequake_search_alias() -> Value {
    let mut t = eq_search_tool();
    t["name"] = json!("edgequake_search");
    let desc = t["description"].as_str().unwrap_or("").to_string() + ALIAS_NOTE;
    t["description"] = json!(desc);
    t
}

fn edgequake_fetch_alias() -> Value {
    let mut t = eq_fetch_tool();
    t["name"] = json!("edgequake_fetch");
    let desc = t["description"].as_str().unwrap_or("").to_string() + ALIAS_NOTE;
    t["description"] = json!(desc);
    t
}

fn edgequake_retrieve_alias() -> Value {
    let mut t = eq_retrieve_tool();
    t["name"] = json!("edgequake_retrieve");
    let desc = t["description"].as_str().unwrap_or("").to_string() + ALIAS_NOTE;
    t["description"] = json!(desc);
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tools_list_has_eq_document_list_and_aliases() {
        let list = tools_list_result();
        assert_eq!(list["ttlMs"], TOOLS_LIST_TTL_MS);
        let names: Vec<&str> = list["tools"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|t| t["name"].as_str())
            .collect();
        assert!(names.contains(&"eq_document_list"));
        assert!(names.contains(&"eq_search"));
        assert!(names.contains(&"edgequake_search"));
        assert!(names.contains(&"edgequake_fetch"));
        assert!(names.contains(&"edgequake_retrieve"));
    }
}
