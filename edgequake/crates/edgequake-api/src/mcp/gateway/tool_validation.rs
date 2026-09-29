//! MCP tool argument validation (SPEC-152).

use edgequake_auth::Role;
use serde_json::Value;

use crate::error::ApiError;
use crate::mcp::project::profile::{mcp_profile, McpProfile};

use super::json_rpc::GatewayError;

const SEARCH_MODES: &[&str] = &["naive", "local", "global", "hybrid", "mix"];
const GRANULARITIES: &[&str] = &["citation", "agent", "debug"];
const MAX_RESULTS_CAP: i64 = 50;
const FETCH_VIEWS: &[&str] = &["toc", "chunks", "entities", "citations", "full"];

/// Validate tool name + arguments before execution.
pub fn validate_tool_call(name: &str, arguments: &Value) -> Result<(), GatewayError> {
    validate_tool_call_with_role(name, arguments, None)
}

/// Validate tool args with optional caller role (debug granularity policy).
pub fn validate_tool_call_with_role(
    name: &str,
    arguments: &Value,
    role: Option<Role>,
) -> Result<(), GatewayError> {
    let canonical = match name {
        "edgequake_search" => "eq_search",
        "edgequake_fetch" => "eq_fetch",
        "edgequake_retrieve" => "eq_retrieve",
        other => other,
    };

    // Memory-only tools
    if matches!(
        canonical,
        "eq_ingest" | "eq_task_get" | "eq_document_delete" | "eq_workspace_delete"
    ) && mcp_profile() != McpProfile::Memory
    {
        return Err(GatewayError::Api(ApiError::BadRequest(
            "tool requires EDGEQUAKE_MCP_PROFILE=memory".into(),
        )));
    }

    match canonical {
        "eq_search" | "eq_retrieve" => {
            require_non_empty_query(arguments)?;
            validate_mode(arguments)?;
            validate_limit_or_max_results(arguments)?;
            if canonical == "eq_retrieve" {
                if let Some(g) = arguments
                    .get("content_granularity")
                    .and_then(|v| v.as_str())
                {
                    validate_granularity(g)?;
                }
                enforce_debug_granularity(arguments, role)?;
            }
            Ok(())
        }
        "eq_fetch" => {
            validate_fetch(arguments)?;
            enforce_debug_granularity(arguments, role)?;
            Ok(())
        }
        "eq_document_list"
        | "eq_workspace_list"
        | "eq_workspace_stats"
        | "eq_document_get"
        | "eq_entity_search"
        | "eq_entity_get"
        | "eq_neighborhood"
        | "eq_ingest"
        | "eq_task_get"
        | "eq_document_delete"
        | "eq_workspace_delete" => Ok(()),
        other => Err(GatewayError::Api(ApiError::BadRequest(format!(
            "Unknown tool: {other}"
        )))),
    }
}

/// EC-MCP-29: debug granularity requires admin when auth is enforced.
pub fn enforce_debug_granularity(
    arguments: &Value,
    role: Option<Role>,
) -> Result<(), GatewayError> {
    let wants_debug = arguments
        .get("content_granularity")
        .and_then(|v| v.as_str())
        .is_some_and(|g| g.eq_ignore_ascii_case("debug"));
    if !wants_debug {
        return Ok(());
    }
    match role {
        Some(Role::Admin) => Ok(()),
        Some(_) => Err(GatewayError::Api(ApiError::forbidden_reason(
            "content_granularity debug requires admin role",
        ))),
        None => Ok(()),
    }
}

fn validate_fetch(arguments: &Value) -> Result<(), GatewayError> {
    let id = arguments
        .get("retrieval_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if !id.starts_with("ret_") {
        return Err(GatewayError::Api(ApiError::BadRequest(
            "Invalid retrieval_id".into(),
        )));
    }
    if let Some(g) = arguments
        .get("content_granularity")
        .and_then(|v| v.as_str())
    {
        validate_granularity(g)?;
    }
    if let Some(view) = arguments.get("view").and_then(|v| v.as_str()) {
        if !FETCH_VIEWS.iter().any(|v| v.eq_ignore_ascii_case(view)) {
            return Err(GatewayError::Api(ApiError::BadRequest(format!(
                "Invalid view: {view}"
            ))));
        }
    }
    Ok(())
}

fn require_non_empty_query(arguments: &Value) -> Result<(), GatewayError> {
    let query = arguments
        .get("query")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if query.trim().is_empty() {
        return Err(GatewayError::Api(ApiError::BadRequest(
            "query is required".into(),
        )));
    }
    Ok(())
}

fn validate_mode(arguments: &Value) -> Result<(), GatewayError> {
    let Some(mode) = arguments.get("mode").and_then(|v| v.as_str()) else {
        return Ok(());
    };
    if mode.eq_ignore_ascii_case("bypass") {
        return Err(GatewayError::Api(ApiError::BadRequest(
            "bypass mode is not allowed on MCP tools".into(),
        )));
    }
    if !SEARCH_MODES.iter().any(|m| m.eq_ignore_ascii_case(mode)) {
        return Err(GatewayError::Api(ApiError::BadRequest(format!(
            "Invalid mode: {mode}"
        ))));
    }
    Ok(())
}

fn validate_granularity(value: &str) -> Result<(), GatewayError> {
    if GRANULARITIES.iter().any(|g| g.eq_ignore_ascii_case(value)) {
        Ok(())
    } else {
        Err(GatewayError::Api(ApiError::BadRequest(format!(
            "Invalid content_granularity: {value}"
        ))))
    }
}

fn validate_limit_or_max_results(arguments: &Value) -> Result<(), GatewayError> {
    for key in ["max_results", "limit"] {
        let Some(n) = arguments.get(key) else {
            continue;
        };
        let Some(v) = n.as_i64() else {
            return Err(GatewayError::Api(ApiError::BadRequest(format!(
                "{key} must be an integer"
            ))));
        };
        if !(1..=MAX_RESULTS_CAP).contains(&v) {
            return Err(GatewayError::Api(ApiError::BadRequest(format!(
                "{key} must be between 1 and {MAX_RESULTS_CAP}"
            ))));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn rejects_bypass_mode() {
        let err =
            validate_tool_call("eq_search", &json!({"query": "x", "mode": "bypass"})).unwrap_err();
        assert!(err.json_rpc_error().message.contains("bypass"));
    }

    #[test]
    fn accepts_eq_document_list() {
        validate_tool_call("eq_document_list", &json!({})).unwrap();
    }

    #[test]
    fn alias_search_still_validates() {
        validate_tool_call("edgequake_search", &json!({"query": "hello"})).unwrap();
    }
}
