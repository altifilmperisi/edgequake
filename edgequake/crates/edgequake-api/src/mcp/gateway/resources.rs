//! MCP resources/list + resources/read for eq:// URIs (SPEC-152).

use serde_json::{json, Value};

use crate::error::ApiError;
use crate::middleware::TenantContext;

use super::dispatch::DispatchContext;
use super::json_rpc::GatewayError;

pub fn resources_list(tenant_ctx: &TenantContext) -> Value {
    let ws = tenant_ctx.workspace_id.as_deref().unwrap_or("default");
    json!({
        "resources": [
            {
                "uri": format!("eq://{ws}/documents"),
                "name": "documents",
                "description": "Workspace document catalog",
                "mimeType": "application/json"
            },
            {
                "uri": format!("eq://{ws}/workspaces"),
                "name": "workspaces",
                "description": "Visible workspaces",
                "mimeType": "application/json"
            }
        ],
        "resourceTemplates": [
            {
                "uriTemplate": "eq://{workspace}/documents/{doc_id}",
                "name": "document",
                "description": "Document metadata",
                "mimeType": "application/json"
            },
            {
                "uriTemplate": "eq://{workspace}/documents/{doc_id}/text",
                "name": "document-text",
                "description": "Document text resource",
                "mimeType": "text/plain"
            },
            {
                "uriTemplate": "eq://{workspace}/chunks/{chunk_id}",
                "name": "chunk",
                "mimeType": "application/json"
            },
            {
                "uriTemplate": "eq://{workspace}/entities/{entity_id}",
                "name": "entity",
                "mimeType": "application/json"
            },
            {
                "uriTemplate": "eq://{workspace}/retrievals/{ret_id}",
                "name": "retrieval",
                "mimeType": "application/json"
            }
        ]
    })
}

pub async fn resources_read(
    ctx: &DispatchContext<'_>,
    params: Option<Value>,
) -> Result<Value, GatewayError> {
    let uri = params
        .as_ref()
        .and_then(|p| p.get("uri"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| GatewayError::Api(ApiError::BadRequest("uri required".into())))?;

    if !uri.starts_with("eq://") {
        return Err(GatewayError::Api(ApiError::BadRequest(
            "uri must start with eq://".into(),
        )));
    }

    // Retrieval hydrate under standard budget
    if let Some(ret_id) = uri
        .rsplit('/')
        .next()
        .filter(|_| uri.contains("/retrievals/"))
    {
        if ret_id.starts_with("ret_") {
            let args = json!({
                "retrieval_id": ret_id,
                "view": "toc",
                "budget": "standard",
                "include_subgraph": false
            });
            let ws = ctx.tenant_ctx.workspace_id.as_deref().unwrap_or("default");
            let body = crate::mcp::project::fetch::eq_fetch(&args, ws)
                .await
                .map_err(GatewayError::Api)?;
            return Ok(json!({
                "contents": [{
                    "uri": uri,
                    "mimeType": "application/json",
                    "text": serde_json::to_string(&body).unwrap_or_else(|_| "{}".into())
                }]
            }));
        }
    }

    Ok(json!({
        "contents": [{
            "uri": uri,
            "mimeType": "application/json",
            "text": "{\"ok\":true,\"note\":\"hydrate via eq_document_get / eq_entity_get / eq_fetch\"}"
        }]
    }))
}
