//! Federated login orchestration (SPEC-158): verified claims → local user + tenant + membership.
//!
//! Order matters and is part of the security contract:
//! 1. provider gate (hd / tid)           — before any account work
//! 2. tenant resolution (org → tenant)   — unknown org denied, never defaulted
//! 3. user resolution `(iss, sub)` first — email linking only if `verified ∧ trust_email`
//! 4. capacity check, then JIT create    — no partial rows on denial
//! 5. membership create / role re-sync

use axum::extract::FromRef;
use chrono::Utc;
use edgequake_core::MembershipRole;
use uuid::Uuid;

use edgequake_auth::Role;

use crate::error::ApiError;
use crate::handlers::auth::{
    find_user_by_login, get_record_by_id, persist_user_record, UserRecord,
};
use crate::services::oidc_flow::OidcFlowService;
use crate::state::{AppState, PostgresRuntime};

use super::claims::FederatedClaims;
use super::link_policy::{check_provider_gate, may_link_by_email};
use super::membership_sync::{ensure_capacity, sync_membership};
use super::role_map::{global_role_for_membership, resolve_membership_role};
use super::store::{FederatedIdentityRecord, LinkOutcome};
use super::tenant_resolver::{resolve_tenant, ResolvedTenant};
use super::username::{first_free_username, synthetic_email};

/// How the local user was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UserOrigin {
    Federated,
    LinkedByEmail,
    Provisioned,
}

pub(crate) struct FederatedLogin {
    pub record: UserRecord,
    pub tenant: ResolvedTenant,
    pub membership_role: MembershipRole,
    /// Global role carried in the JWT (SSO never grants platform admin by itself).
    pub session_role: Role,
    pub origin: UserOrigin,
}

pub(crate) async fn resolve_federated_login(
    state: &AppState,
    provider: &OidcFlowService,
    claims: &FederatedClaims,
    requested_org: Option<&str>,
) -> Result<FederatedLogin, ApiError> {
    let policy = provider.policy();
    check_provider_gate(policy, claims).map_err(|d| ApiError::forbidden_reason(d.code()))?;

    let tenant = resolve_tenant(
        state.workspace_service.as_ref(),
        policy,
        claims,
        requested_org,
    )
    .await?
    .map_err(|d| d.into_api_error())?;

    let (mut record, origin) = resolve_user(state, provider, claims, &tenant).await?;
    crate::services::login_lockout::ensure_login_allowed(&record)?;
    if !record.is_active {
        return Err(ApiError::forbidden_reason("account_inactive"));
    }

    let user_uuid = parse_uuid(&record.user_id)?;
    let desired = resolve_membership_role(policy, claims);
    let membership_role = sync_membership(
        state.workspace_service.as_ref(),
        user_uuid,
        &tenant,
        desired,
        &policy.slug,
    )
    .await?;

    state
        .auth
        .federation
        .touch_identity(&claims.issuer, &claims.subject)
        .await?;
    record.updated_at = Utc::now();

    let session_role = if Role::parse(&record.role) == Role::Admin {
        // Explicit local platform admin who federated: keep what the operator granted.
        Role::Admin
    } else {
        global_role_for_membership(membership_role)
    };
    Ok(FederatedLogin {
        record,
        tenant,
        membership_role,
        session_role,
        origin,
    })
}

async fn load_record(state: &AppState, user_id: &str) -> Result<UserRecord, ApiError> {
    let pg = PostgresRuntime::from_ref(state);
    get_record_by_id(
        &state.storage,
        Some(&pg),
        &state.security,
        state.operational_stores.identity.as_deref(),
        user_id,
    )
    .await?
    .ok_or_else(|| ApiError::Internal("federated user record missing".into()))
}

async fn find_by_login(state: &AppState, login: &str) -> Result<Option<String>, ApiError> {
    let pg = PostgresRuntime::from_ref(state);
    Ok(
        find_user_by_login(&state.storage, Some(&pg), &state.security, login)
            .await?
            .map(|u| u.user_id),
    )
}

async fn resolve_user(
    state: &AppState,
    provider: &OidcFlowService,
    claims: &FederatedClaims,
    tenant: &ResolvedTenant,
) -> Result<(UserRecord, UserOrigin), ApiError> {
    let store = &state.auth.federation;
    let policy = provider.policy();

    // 1. The federation key is (issuer, subject) — never the email (LAW-158-3).
    if let Some(identity) = store.find_identity(&claims.issuer, &claims.subject).await? {
        let record = load_record(state, &identity.user_id.to_string()).await?;
        return Ok((record, UserOrigin::Federated));
    }

    // 2. Intentional linking: verified email AND trusted provider only.
    if let Some(email) = claims
        .email
        .as_deref()
        .filter(|_| may_link_by_email(policy, claims))
    {
        if let Some(existing_id) = find_by_login(state, email).await? {
            let record = load_record(state, &existing_id).await?;
            let origin =
                link_identity(state, provider, claims, &record, UserOrigin::LinkedByEmail).await?;
            return Ok((record, origin));
        }
    }

    // 3. JIT provisioning.
    if !policy.jit_enabled {
        return Err(ApiError::forbidden_reason("jit_disabled"));
    }
    ensure_capacity(state.workspace_service.as_ref(), tenant, None).await?;
    let record = provision_user(state, provider, claims).await?;
    let origin = link_identity(state, provider, claims, &record, UserOrigin::Provisioned).await?;
    if origin == UserOrigin::Federated {
        // Lost the race to a concurrent first login: use the winner's user.
        let winner = store
            .find_identity(&claims.issuer, &claims.subject)
            .await?
            .ok_or_else(|| ApiError::Internal("federated identity vanished".into()))?;
        return Ok((
            load_record(state, &winner.user_id.to_string()).await?,
            origin,
        ));
    }
    Ok((record, origin))
}

async fn link_identity(
    state: &AppState,
    provider: &OidcFlowService,
    claims: &FederatedClaims,
    record: &UserRecord,
    origin: UserOrigin,
) -> Result<UserOrigin, ApiError> {
    let now = Utc::now();
    let outcome = state
        .auth
        .federation
        .link_identity(&FederatedIdentityRecord {
            federated_id: Uuid::new_v4(),
            user_id: parse_uuid(&record.user_id)?,
            provider_slug: provider.slug().to_string(),
            issuer: claims.issuer.clone(),
            subject: claims.subject.clone(),
            email_at_link: claims.email.clone(),
            email_verified_at_link: claims.email_verified,
            linked_at: now,
            last_login_at: now,
        })
        .await?;
    Ok(match outcome {
        LinkOutcome::Created => origin,
        LinkOutcome::AlreadyLinked(_) => UserOrigin::Federated,
    })
}

/// Next free username (`base`, `base-2`, …) checked against the identity store.
async fn free_username(state: &AppState, base: &str) -> Result<String, ApiError> {
    let mut taken: Vec<String> = Vec::new();
    for _ in 0..50 {
        let candidate = first_free_username(base, |c| taken.iter().any(|t| t == c));
        if find_by_login(state, &candidate).await?.is_none() {
            return Ok(candidate);
        }
        taken.push(candidate);
    }
    Ok(format!(
        "{}-{}",
        first_free_username(base, |_| false),
        &Uuid::new_v4().to_string()[..8]
    ))
}

async fn provision_user(
    state: &AppState,
    provider: &OidcFlowService,
    claims: &FederatedClaims,
) -> Result<UserRecord, ApiError> {
    let policy = provider.policy();
    let email = match claims.email.as_deref() {
        Some(real) => {
            // Global UNIQUE(email): an unlinked existing account is a hard conflict, never a link.
            if find_by_login(state, real).await?.is_some() {
                return Err(ApiError::Conflict("account_exists_unlinked".into()));
            }
            real.to_string()
        }
        None => synthetic_email(&policy.slug, &claims.subject),
    };
    let base = claims
        .preferred_username
        .clone()
        .or_else(|| {
            claims
                .email
                .as_deref()
                .and_then(|e| e.split('@').next().map(str::to_string))
        })
        .unwrap_or_else(|| {
            format!(
                "sso-{}",
                &claims.subject.chars().take(12).collect::<String>()
            )
        });
    let username = free_username(state, &base).await?;

    let password_hash = state
        .auth
        .password
        .hash_unvalidated_secret(&Uuid::new_v4().to_string())
        .map_err(|e| ApiError::Internal(format!("sso password hash: {e}")))?;
    let role = global_role_for_membership(resolve_membership_role(policy, claims));
    let now = Utc::now();
    let record = UserRecord {
        user_id: Uuid::new_v4().to_string(),
        username,
        email,
        password_hash,
        role: role.to_string(),
        is_active: true,
        created_at: now,
        updated_at: now,
        last_login_at: Some(now),
        failed_login_attempts: 0,
        locked_until: None,
        metadata: std::collections::HashMap::from([
            ("auth_provider".to_string(), serde_json::json!(policy.slug)),
            ("oidc_issuer".to_string(), serde_json::json!(claims.issuer)),
            (
                "oidc_subject".to_string(),
                serde_json::json!(claims.subject),
            ),
        ]),
    };
    let pg = PostgresRuntime::from_ref(state);
    persist_user_record(
        &state.storage,
        Some(&pg),
        &state.security,
        state.operational_stores.identity.as_deref(),
        &record,
    )
    .await?;
    Ok(record)
}

fn parse_uuid(raw: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(raw).map_err(|_| ApiError::Internal("invalid user id".into()))
}
