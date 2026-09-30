//! MCP resource scope helpers (SPEC-152).

pub const MCP_SCOPE_READ: &str = "edgequake:read";
pub const MCP_SCOPE_QUERY: &str = "edgequake:query";
pub const MCP_SCOPE_WRITE: &str = "edgequake:write";

/// Default scopes for newly minted user API keys (SPEC-154 LAW-154-5).
pub fn default_api_key_scopes() -> Vec<String> {
    vec![MCP_SCOPE_READ.to_string(), MCP_SCOPE_QUERY.to_string()]
}

/// Normalize legacy (`read`/`write`/`query`) and OAuth scope names.
///
/// Empty input → read+query (write fail-closed).
pub fn normalize_api_key_scopes(raw: &[String]) -> Vec<String> {
    if raw.is_empty() {
        return default_api_key_scopes();
    }
    let mut out = Vec::new();
    for s in raw {
        let t = s.trim();
        if t.is_empty() {
            continue;
        }
        let mapped = match t {
            "read" | "edgequake:read" => MCP_SCOPE_READ,
            "query" | "edgequake:query" => MCP_SCOPE_QUERY,
            "write" | "edgequake:write" => MCP_SCOPE_WRITE,
            "admin" | "*" => "*",
            other if other.starts_with("edgequake:") => other,
            _ => continue,
        };
        let owned = mapped.to_string();
        if !out.contains(&owned) {
            out.push(owned);
        }
    }
    if out.is_empty() {
        default_api_key_scopes()
    } else {
        out
    }
}

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
///
/// SPEC-154 LAW-154-4: empty grant never means allow-all (DRY with
/// [`McpAuthScopes::allows`] for OAuth JWTs).
pub fn scopes_cover(granted: &[String], required: &str) -> bool {
    crate::oauth::types::McpAuthScopes {
        scopes: granted.to_vec(),
        is_api_key: false,
        break_glass: false,
    }
    .allows(required)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scopes_cover_empty_never_allows() {
        // SPEC-154 LAW-154-4 / EC-154-04
        assert!(!scopes_cover(&[], MCP_SCOPE_READ));
        assert!(!scopes_cover(&[], MCP_SCOPE_QUERY));
    }

    #[test]
    fn scopes_cover_explicit_and_star() {
        let granted = vec![MCP_SCOPE_READ.to_string()];
        assert!(scopes_cover(&granted, MCP_SCOPE_READ));
        assert!(!scopes_cover(&granted, MCP_SCOPE_QUERY));
        let star = vec!["*".to_string()];
        assert!(scopes_cover(&star, MCP_SCOPE_WRITE));
    }
}
