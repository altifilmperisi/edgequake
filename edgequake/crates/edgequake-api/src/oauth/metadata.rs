//! RFC 8414 Authorization Server Metadata (+ OIDC discovery alias).

use axum::{extract::State, http::HeaderMap, Json};
use serde_json::{json, Value};

use crate::mcp::config::{mcp_resource_scopes_supported, McpPublicConfig};
use crate::state::AppState;

/// `GET /.well-known/oauth-authorization-server`
pub async fn authorization_server_metadata(
    State(_state): State<AppState>,
    headers: HeaderMap,
) -> Json<Value> {
    Json(metadata_document(&headers))
}

/// `GET /.well-known/openid-configuration` (same document).
pub async fn openid_configuration(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Json<Value> {
    authorization_server_metadata(State(state), headers).await
}

fn metadata_document(headers: &HeaderMap) -> Value {
    let cfg = McpPublicConfig::resolve(headers);
    let issuer = cfg.authorization_server.trim_end_matches('/').to_string();
    json!({
        "issuer": issuer,
        "authorization_endpoint": format!("{issuer}/oauth/authorize"),
        "token_endpoint": format!("{issuer}/oauth/token"),
        "registration_endpoint": format!("{issuer}/oauth/register"),
        "revocation_endpoint": format!("{issuer}/oauth/revoke"),
        "response_types_supported": ["code"],
        "grant_types_supported": ["authorization_code", "refresh_token"],
        "code_challenge_methods_supported": ["S256"],
        "token_endpoint_auth_methods_supported": ["none"],
        "revocation_endpoint_auth_methods_supported": ["none"],
        "scopes_supported": mcp_resource_scopes_supported(),
        "authorization_response_iss_parameter_supported": true,
        "resource_indicators_supported": true,
    })
}
