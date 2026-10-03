//! Re-validate an SSO session's tenant scope at every credential renewal
//! (SPEC-158 LAW-158-5, EC-158-34): suspended tenant or revoked membership ⇒ 403, immediately.

use edgequake_auth::Role;
use edgequake_core::WorkspaceService;
use uuid::Uuid;

use crate::error::ApiError;
use crate::services::login_tokens::SessionScope;

use super::role_map::global_role_for_membership;

/// Return the *current* global role for this user inside `scope`, or a 403 with a stable code.
///
/// `local_role` is the user record's own role: an explicit local platform admin stays admin; an
/// SSO user's JWT role always derives from the live membership (role downgrades take effect on
/// the next refresh, not only on the next login).
pub async fn revalidate_scope(
    workspaces: &dyn WorkspaceService,
    user_id: Uuid,
    scope: &SessionScope,
    local_role: &str,
) -> Result<Role, ApiError> {
    let internal = |e: edgequake_core::Error| ApiError::Internal(format!("workspace service: {e}"));
    if let Some(tenant) = workspaces
        .get_tenant(scope.tenant_id)
        .await
        .map_err(internal)?
    {
        if !tenant.is_active {
            return Err(ApiError::forbidden_reason("tenant_suspended"));
        }
    }
    let membership = workspaces
        .get_user_role(user_id, scope.tenant_id)
        .await
        .map_err(internal)?
        .ok_or_else(|| ApiError::forbidden_reason("membership_revoked"))?;
    if Role::parse(local_role) == Role::Admin {
        return Ok(Role::Admin);
    }
    Ok(global_role_for_membership(membership))
}
