//! Workspace / tenant claim enforcement for MCP tools (FP-MCP-04, EC-MCP-30).

use edgequake_auth::Role;
use serde_json::Value;

use crate::error::ApiError;
use crate::middleware::TenantContext;

use super::json_rpc::GatewayError;

/// Auth claims beat tool-supplied workspace (EC-MCP-30 / EC-154-30).
pub fn enforce_workspace_claim(
    tenant_ctx: &TenantContext,
    arguments: &Value,
    auth_role: Option<Role>,
) -> Result<(), GatewayError> {
    if auth_role.is_none() {
        return Ok(());
    }

    let arg_ws = arguments
        .get("workspace_id")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let Some(arg_ws) = arg_ws else {
        return Ok(());
    };

    let ctx_ws = tenant_ctx
        .workspace_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    match ctx_ws {
        Some(ctx) if ctx == arg_ws => Ok(()),
        Some(ctx) => Err(GatewayError::Api(ApiError::forbidden_reason(format!(
            "workspace_id '{arg_ws}' does not match authenticated workspace claim '{ctx}'"
        )))),
        // EC-154-30: authenticated but unbound — any tool workspace_id is foreign.
        None => Err(GatewayError::Api(ApiError::forbidden_reason(format!(
            "workspace_id '{arg_ws}' is forbidden without an authenticated workspace claim"
        )))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_auth::Role;
    use serde_json::json;

    #[test]
    fn allows_matching_workspace() {
        let ctx = TenantContext {
            workspace_id: Some("ws-a".into()),
            ..Default::default()
        };
        enforce_workspace_claim(&ctx, &json!({ "workspace_id": "ws-a" }), Some(Role::User))
            .expect("match");
    }

    #[test]
    fn rejects_mismatch() {
        let ctx = TenantContext {
            workspace_id: Some("ws-a".into()),
            ..Default::default()
        };
        assert!(enforce_workspace_claim(
            &ctx,
            &json!({ "workspace_id": "ws-b" }),
            Some(Role::User),
        )
        .is_err());
    }

    #[test]
    fn ec_154_30_rejects_tool_workspace_without_claim() {
        let ctx = TenantContext {
            workspace_id: None,
            ..Default::default()
        };
        assert!(enforce_workspace_claim(
            &ctx,
            &json!({ "workspace_id": "ws-foreign" }),
            Some(Role::User),
        )
        .is_err());
    }
}
