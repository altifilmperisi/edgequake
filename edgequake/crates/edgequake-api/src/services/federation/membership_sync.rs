//! SSO-managed membership lifecycle (SPEC-158 LAW-158-8, EC-158-18/19/32/33).
//!
//! Only memberships created by SSO (`metadata.source == "sso"`) are re-synchronised from IdP
//! roles on later logins; manually granted memberships are never overwritten. The last owner of
//! a tenant is never demoted by an IdP claim change.

use edgequake_core::{Membership, MembershipRole, WorkspaceService};
use serde_json::json;
use uuid::Uuid;

use crate::error::ApiError;

use super::tenant_resolver::ResolvedTenant;

const SSO_SOURCE: &str = "sso";

fn map_ws(e: edgequake_core::Error) -> ApiError {
    ApiError::Internal(format!("workspace service: {e}"))
}

fn is_sso_managed(m: &Membership) -> bool {
    m.metadata.get("source").and_then(|v| v.as_str()) == Some(SSO_SOURCE)
}

/// Active members of the tenant, de-duplicated by user.
async fn distinct_member_count(
    ws: &dyn WorkspaceService,
    tenant_id: Uuid,
) -> Result<usize, ApiError> {
    let members = ws.get_tenant_memberships(tenant_id).await.map_err(map_ws)?;
    let mut users: Vec<Uuid> = members.iter().map(|m| m.user_id).collect();
    users.sort();
    users.dedup();
    Ok(users.len())
}

/// `max_users` guard — runs **before** any user row is created (no partial rows).
pub async fn ensure_capacity(
    ws: &dyn WorkspaceService,
    tenant: &ResolvedTenant,
    user_id: Option<Uuid>,
) -> Result<(), ApiError> {
    let Some(max) = tenant.max_users else {
        return Ok(());
    };
    if let Some(uid) = user_id {
        let members = ws
            .get_tenant_memberships(tenant.tenant_id)
            .await
            .map_err(map_ws)?;
        if members.iter().any(|m| m.user_id == uid) {
            return Ok(());
        }
    }
    if distinct_member_count(ws, tenant.tenant_id).await? >= max {
        return Err(ApiError::forbidden_reason("max_users"));
    }
    Ok(())
}

async fn is_last_owner(
    ws: &dyn WorkspaceService,
    tenant_id: Uuid,
    user_id: Uuid,
) -> Result<bool, ApiError> {
    let members = ws.get_tenant_memberships(tenant_id).await.map_err(map_ws)?;
    let owners: Vec<_> = members
        .iter()
        .filter(|m| m.role == MembershipRole::Owner)
        .collect();
    Ok(owners.len() == 1 && owners[0].user_id == user_id)
}

/// Create or re-sync the membership; returns the **effective** role to put in the session.
pub async fn sync_membership(
    ws: &dyn WorkspaceService,
    user_id: Uuid,
    tenant: &ResolvedTenant,
    desired: MembershipRole,
    provider_slug: &str,
) -> Result<MembershipRole, ApiError> {
    let existing = ws
        .get_user_memberships(user_id)
        .await
        .map_err(map_ws)?
        .into_iter()
        .find(|m| {
            m.tenant_id == tenant.tenant_id
                && (m.workspace_id.is_none() || m.workspace_id == tenant.workspace_id)
        });

    let Some(current) = existing else {
        ensure_capacity(ws, tenant, Some(user_id)).await?;
        return create(ws, user_id, tenant, desired, provider_slug).await;
    };

    if !is_sso_managed(&current) || current.role == desired {
        return Ok(current.role);
    }
    if current.role == MembershipRole::Owner
        && desired.level() < current.role.level()
        && is_last_owner(ws, tenant.tenant_id, user_id).await?
    {
        tracing::warn!(
            audit.event = "last_owner_demotion_skipped",
            tenant_id = %tenant.tenant_id,
            user_id = %user_id,
            "SPEC-158 EC-158-33: IdP claims would demote the last owner; keeping owner"
        );
        return Ok(current.role);
    }
    let updated = ws
        .update_membership_role(current.membership_id, desired)
        .await
        .map_err(map_ws)?;
    Ok(updated.role)
}

async fn create(
    ws: &dyn WorkspaceService,
    user_id: Uuid,
    tenant: &ResolvedTenant,
    role: MembershipRole,
    provider_slug: &str,
) -> Result<MembershipRole, ApiError> {
    let mut membership = Membership::new(user_id, tenant.tenant_id, role);
    if let Some(workspace_id) = tenant.workspace_id {
        membership = membership.for_workspace(workspace_id);
    }
    membership
        .metadata
        .insert("source".into(), json!(SSO_SOURCE));
    membership
        .metadata
        .insert("provider".into(), json!(provider_slug));
    match ws.add_membership(membership).await {
        Ok(m) => Ok(m.role),
        // Concurrent first login (EC-158-20): the winner's row stands.
        Err(e)
            if e.to_string()
                .to_ascii_lowercase()
                .contains("already exists") =>
        {
            Ok(role)
        }
        Err(e) => Err(map_ws(e)),
    }
}
