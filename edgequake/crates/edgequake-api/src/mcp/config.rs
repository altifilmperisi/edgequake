//! MCP public URL and resource identity (RFC 9728).

use axum::http::HeaderMap;

/// Resource scopes advertised on Protected Resource Metadata and 401 challenges.
pub const MCP_RESOURCE_SCOPES: &[&str] = &["edgequake:read", "edgequake:query"];

/// Public MCP resource configuration derived from env or request Host.
#[derive(Debug, Clone)]
pub struct McpPublicConfig {
    pub resource_url: String,
    pub authorization_server: String,
    pub public_base: String,
}

impl McpPublicConfig {
    /// Resolve MCP resource URL and authorization server base.
    pub fn resolve(headers: &HeaderMap) -> Self {
        let public_base = std::env::var("EDGEQUAKE_PUBLIC_URL")
            .ok()
            .map(|s| s.trim().trim_end_matches('/').to_string())
            .filter(|s| s.starts_with("http"))
            .or_else(|| forwarded_base(headers))
            .or_else(|| host_header_base(headers))
            .unwrap_or_else(|| "http://127.0.0.1:8080".to_string());

        let auth_base = std::env::var("EDGEQUAKE_OAUTH_ISSUER_URL")
            .ok()
            .map(|s| s.trim().trim_end_matches('/').to_string())
            .filter(|s| s.starts_with("http"))
            .unwrap_or_else(|| public_base.clone());

        Self {
            resource_url: format!("{public_base}/mcp"),
            authorization_server: auth_base,
            public_base,
        }
    }

    /// RFC 9728 path-inserted metadata URL for resource `…/mcp`.
    pub fn protected_resource_metadata_url(&self) -> String {
        format!(
            "{}/.well-known/oauth-protected-resource/mcp",
            self.public_base_url()
        )
    }

    /// Root-level PRM fallback URL.
    pub fn protected_resource_metadata_root_url(&self) -> String {
        format!(
            "{}/.well-known/oauth-protected-resource",
            self.public_base_url()
        )
    }

    /// Public base URL without `/mcp` suffix.
    pub fn public_base_url(&self) -> String {
        self.public_base.trim_end_matches('/').to_string()
    }

    /// Space-separated scopes for `WWW-Authenticate` challenge.
    pub fn challenge_scope() -> &'static str {
        "edgequake:read edgequake:query"
    }
}

fn forwarded_base(headers: &HeaderMap) -> Option<String> {
    let host = headers
        .get("x-forwarded-host")
        .or_else(|| headers.get("host"))?
        .to_str()
        .ok()?;
    let host = host.split(',').next()?.trim();
    if host.is_empty() {
        return None;
    }
    let proto = headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim())
        .filter(|s| *s == "http" || *s == "https")
        .unwrap_or("https");
    // Prefer https when behind a TLS-terminating proxy that forgot X-Forwarded-Proto.
    let scheme = if proto == "http" && headers.get("x-forwarded-proto").is_none() {
        "http"
    } else {
        proto
    };
    Some(format!("{scheme}://{host}"))
}

fn host_header_base(headers: &HeaderMap) -> Option<String> {
    let host = headers.get("host")?.to_str().ok()?;
    if host.starts_with("http") {
        return Some(host.trim_end_matches('/').to_string());
    }
    // Local / test defaults: http. Production should set EDGEQUAKE_PUBLIC_URL or X-Forwarded-*.
    let scheme = if host.starts_with("127.0.0.1") || host.starts_with("localhost") {
        "http"
    } else {
        "https"
    };
    Some(format!("{scheme}://{host}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn env_public_url_wins() {
        std::env::set_var("EDGEQUAKE_PUBLIC_URL", "https://api.example.com");
        let cfg = McpPublicConfig::resolve(&HeaderMap::new());
        assert_eq!(cfg.resource_url, "https://api.example.com/mcp");
        assert_eq!(
            cfg.protected_resource_metadata_url(),
            "https://api.example.com/.well-known/oauth-protected-resource/mcp"
        );
        std::env::remove_var("EDGEQUAKE_PUBLIC_URL");
    }

    #[test]
    fn forwarded_proto_https() {
        std::env::remove_var("EDGEQUAKE_PUBLIC_URL");
        let mut headers = HeaderMap::new();
        headers.insert("host", HeaderValue::from_static("demo.edgequake.com"));
        headers.insert("x-forwarded-proto", HeaderValue::from_static("https"));
        let cfg = McpPublicConfig::resolve(&headers);
        assert_eq!(cfg.resource_url, "https://demo.edgequake.com/mcp");
    }
}
