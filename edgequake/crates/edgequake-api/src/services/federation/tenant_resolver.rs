//! Organization → tenant resolution (SPEC-158 LAW-158-5).
//!
//! An unknown organization is **denied**, never defaulted. Only a provider that does not require
//! an organization may fall back to the legacy default tenant.

use edgequake_core::{Tenant, WorkspaceService};
use uuid::Uuid;

use crate::error::ApiError;
use crate::services::login_tokens::SessionScope;

use super::claims::FederatedClaims;
use super::policy::FederationPolicy;

#[derive(Debug, Clone)]
pub struct ResolvedTenant {
    pub tenant_id: Uuid,
    pub slug: String,
    pub workspace_id: Option<Uuid>,
    pub max_users: Option<usize>,
}

impl ResolvedTenant {
    pub fn scope(&self) -> SessionScope {
        SessionScope {
            tenant_id: self.tenant_id,
            workspace_id: self.workspace_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TenantDenial {
    /// Org claim present but no (active-capable) tenant maps to it, or requested org not asserted.
    OrgUnknown,
    /// Provider requires an org and the token carries none.
    OrgMissing,
    /// User belongs to several known orgs and none was selected (EC-158-13).
    OrgAmbiguous(Vec<String>),
    TenantSuspended,
}

impl TenantDenial {
    pub fn code(&self) -> &'static str {
        match self {
            Self::OrgUnknown => "org_unknown",
            Self::OrgMissing => "org_missing",
            Self::OrgAmbiguous(_) => "org_ambiguous",
            Self::TenantSuspended => "tenant_suspended",
        }
    }

    pub fn into_api_error(self) -> ApiError {
        match self {
            // Machine-readable list lets the SPA render the picker (EC-158-13).
            Self::OrgAmbiguous(orgs) => {
                ApiError::forbidden_reason(format!("org_ambiguous:{}", orgs.join(",")))
            }
            other => ApiError::forbidden_reason(other.code()),
        }
    }
}

/// Choose the tenant for this login.
pub async fn resolve_tenant(
    workspaces: &dyn WorkspaceService,
    policy: &FederationPolicy,
    claims: &FederatedClaims,
    requested_org: Option<&str>,
) -> Result<Result<ResolvedTenant, TenantDenial>, ApiError> {
    if let Some(slug) = policy.fixed_tenant_slug.as_deref() {
        return lookup_active(workspaces, slug).await;
    }

    if claims.organizations.is_empty() {
        if policy.require_org || requested_org.is_some() {
            return Ok(Err(TenantDenial::OrgMissing));
        }
        return default_tenant(workspaces).await.map(Ok);
    }

    let requested = requested_org
        .map(|o| o.trim().to_ascii_lowercase())
        .filter(|o| !o.is_empty());
    if let Some(req) = requested.as_deref() {
        if !claims.organizations.iter().any(|o| o.alias == req) {
            return Ok(Err(TenantDenial::OrgUnknown));
        }
        return lookup_active(workspaces, req).await;
    }

    // Keep only orgs EdgeQuake knows; ambiguity is an explicit choice, never a silent pick.
    let mut known = Vec::new();
    let mut suspended = false;
    for org in &claims.organizations {
        match workspaces
            .get_tenant_by_slug(&org.alias)
            .await
            .map_err(map_ws)?
        {
            Some(t) if t.is_active => known.push(t),
            Some(_) => suspended = true,
            None => {}
        }
    }
    match known.len() {
        0 if suspended => Ok(Err(TenantDenial::TenantSuspended)),
        0 => Ok(Err(TenantDenial::OrgUnknown)),
        1 => Ok(Ok(finish(workspaces, known.remove(0)).await?)),
        _ => Ok(Err(TenantDenial::OrgAmbiguous(
            known.into_iter().map(|t| t.slug).collect(),
        ))),
    }
}

async fn lookup_active(
    workspaces: &dyn WorkspaceService,
    slug: &str,
) -> Result<Result<ResolvedTenant, TenantDenial>, ApiError> {
    match workspaces.get_tenant_by_slug(slug).await.map_err(map_ws)? {
        None => Ok(Err(TenantDenial::OrgUnknown)),
        Some(t) if !t.is_active => Ok(Err(TenantDenial::TenantSuspended)),
        Some(t) => Ok(Ok(finish(workspaces, t).await?)),
    }
}

async fn default_tenant(workspaces: &dyn WorkspaceService) -> Result<ResolvedTenant, ApiError> {
    let (tenant_id, workspace_id) = crate::services::identity_storage::default_identity_scope();
    let tenant = workspaces.get_tenant(tenant_id).await.map_err(map_ws)?;
    Ok(ResolvedTenant {
        tenant_id,
        slug: tenant
            .as_ref()
            .map(|t| t.slug.clone())
            .unwrap_or_else(|| "default".into()),
        workspace_id: Some(workspace_id),
        max_users: tenant.map(|t| t.max_users),
    })
}

/// Attach the tenant's default workspace (slug `default`, else oldest).
async fn finish(
    workspaces: &dyn WorkspaceService,
    tenant: Tenant,
) -> Result<ResolvedTenant, ApiError> {
    let list = workspaces
        .list_workspaces(tenant.tenant_id)
        .await
        .map_err(map_ws)?;
    let workspace_id = list
        .iter()
        .find(|w| w.slug == "default")
        .or_else(|| list.iter().min_by_key(|w| w.created_at))
        .map(|w| w.workspace_id);
    Ok(ResolvedTenant {
        tenant_id: tenant.tenant_id,
        slug: tenant.slug,
        workspace_id,
        max_users: Some(tenant.max_users),
    })
}

fn map_ws(e: edgequake_core::Error) -> ApiError {
    ApiError::Internal(format!("workspace service: {e}"))
}
