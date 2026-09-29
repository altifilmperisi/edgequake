//! WWW-Authenticate builder for MCP OAuth (RFC 6750 + RFC 9728).

use crate::mcp::config::McpPublicConfig;

/// Build `WWW-Authenticate` header value for MCP 401 responses.
pub fn www_authenticate_bearer(headers: &axum::http::HeaderMap) -> String {
    let cfg = McpPublicConfig::resolve(headers);
    let prm_url = cfg.protected_resource_metadata_url();
    let scope = McpPublicConfig::challenge_scope();
    format!(r#"Bearer realm="edgequake-mcp", resource_metadata="{prm_url}", scope="{scope}""#)
}

/// Build `WWW-Authenticate` for insufficient_scope (403).
pub fn www_authenticate_insufficient_scope(
    headers: &axum::http::HeaderMap,
    required_scope: &str,
) -> String {
    let cfg = McpPublicConfig::resolve(headers);
    let prm_url = cfg.protected_resource_metadata_url();
    format!(
        r#"Bearer error="insufficient_scope", scope="{required_scope}", resource_metadata="{prm_url}", error_description="Additional scopes required for this MCP tool""#
    )
}
