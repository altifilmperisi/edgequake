//! MCP gateway authentication — OAuth-aware 401 with PRM pointer + audience binding.

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};

use crate::mcp::config::McpPublicConfig;
use crate::oauth::types::McpAuthScopes;

use super::www_authenticate::www_authenticate_bearer;

pub async fn mcp_gateway_auth(
    State(state): State<crate::state::AppState>,
    mut request: Request<Body>,
    next: Next,
) -> Response<Body> {
    if !state.auth.config.auth_enabled {
        request
            .extensions_mut()
            .insert(McpAuthScopes::api_key_full());
        return next.run(request).await;
    }

    if let Some(token) = crate::middleware::extract_api_key(&request) {
        // Master / stored API keys — full MCP access, no audience requirement.
        if let Ok(Some(auth)) =
            crate::services::auth_validation::validate_master_or_stored_api_key(&state, &token)
                .await
        {
            if let Some(response) =
                crate::middleware::apply_authenticated_context(&state, &mut request, auth)
            {
                return response;
            }
            request
                .extensions_mut()
                .insert(McpAuthScopes::api_key_full());
            return next.run(request).await;
        }

        // JWT path — require aud = MCP resource URL (RFC 8707).
        if let Ok(claims) = state.auth.jwt.verify_token(&token) {
            let cfg = McpPublicConfig::resolve(request.headers());
            let aud_ok = claims
                .aud
                .as_ref()
                .is_some_and(|aud| aud.iter().any(|a| a == &cfg.resource_url));
            if !aud_ok {
                return oauth_unauthorized_response(request.headers());
            }

            let scopes = McpAuthScopes::from_scope_claim(claims.scope.as_deref());
            match crate::services::auth_validation::authenticated_from_claims(&claims) {
                Ok(authenticated) => {
                    if let Some(response) = crate::middleware::apply_authenticated_context(
                        &state,
                        &mut request,
                        authenticated,
                    ) {
                        return response;
                    }
                    request.extensions_mut().insert(scopes);
                    return next.run(request).await;
                }
                Err(e) => return e.into_response(),
            }
        }
    }

    oauth_unauthorized_response(request.headers())
}

fn oauth_unauthorized_response(headers: &axum::http::HeaderMap) -> Response<Body> {
    let www = www_authenticate_bearer(headers);
    let mut response = (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({
            "error": "unauthorized",
            "message": "Authentication required — use OAuth 2.1 Bearer token or API key"
        })),
    )
        .into_response();
    if let Ok(val) = www.parse() {
        response.headers_mut().insert("WWW-Authenticate", val);
    }
    response
}
