//! MCP resource scope helpers (SPEC-152).

pub const MCP_SCOPE_READ: &str = "edgequake:read";
pub const MCP_SCOPE_QUERY: &str = "edgequake:query";
pub const MCP_SCOPE_WRITE: &str = "edgequake:write";

/// Scope required for a tool (tools/list uses read).
pub fn required_scope_for_tool(tool_name: &str) -> Option<&'static str> {
    let canonical = match tool_name {
        "edgequake_search" => "eq_search",
        "edgequake_fetch" => "eq_fetch",
        "edgequake_retrieve" => "eq_retrieve",
        other => other,
    };
    match canonical {
        "eq_fetch" | "eq_document_list" | "eq_document_get" | "eq_workspace_list"
        | "eq_workspace_stats" | "eq_entity_get" | "eq_neighborhood" | "eq_task_get" => {
            Some(MCP_SCOPE_READ)
        }
        "eq_search" | "eq_retrieve" | "eq_entity_search" => Some(MCP_SCOPE_QUERY),
        "eq_ingest" | "eq_document_delete" | "eq_workspace_delete" => Some(MCP_SCOPE_WRITE),
        _ => Some(MCP_SCOPE_READ),
    }
}

/// Whether granted scopes cover the required scope.
pub fn scopes_cover(granted: &[String], required: &str) -> bool {
    if granted.is_empty() {
        return true;
    }
    granted.iter().any(|s| s == required || s == "*")
}

/// Intersect requested scopes with supported resource scopes.
pub fn normalize_requested_scopes(requested: Option<&str>) -> String {
    let supported = [MCP_SCOPE_READ, MCP_SCOPE_QUERY, MCP_SCOPE_WRITE];
    let requested: Vec<&str> = requested
        .unwrap_or("")
        .split_whitespace()
        .filter(|s| !s.is_empty())
        .collect();
    if requested.is_empty() {
        return format!("{MCP_SCOPE_READ} {MCP_SCOPE_QUERY}");
    }
    let kept: Vec<&str> = requested
        .into_iter()
        .filter(|s| supported.contains(s))
        .collect();
    if kept.is_empty() {
        format!("{MCP_SCOPE_READ} {MCP_SCOPE_QUERY}")
    } else {
        kept.join(" ")
    }
}
