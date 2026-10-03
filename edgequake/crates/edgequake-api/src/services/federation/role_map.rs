//! IdP claim → membership role mapping (SPEC-158 LAW-158-8, EC-158-32/33).

use edgequake_core::MembershipRole;

use super::claims::FederatedClaims;
use super::policy::FederationPolicy;

/// Highest mapped role among the IdP's role claims, else `default_role`; capped at `max_role`.
pub fn resolve_membership_role(
    policy: &FederationPolicy,
    claims: &FederatedClaims,
) -> MembershipRole {
    let mapped = claims
        .roles
        .iter()
        .filter_map(|r| policy.role_map.get(r))
        .copied()
        .max_by_key(MembershipRole::level)
        .unwrap_or(policy.default_role);
    cap_role(mapped, policy.max_role)
}

pub fn cap_role(role: MembershipRole, max: MembershipRole) -> MembershipRole {
    if role.level() > max.level() {
        max
    } else {
        role
    }
}

/// Map a tenant membership role onto the *global* auth role carried in the JWT.
///
/// SSO never yields platform `admin` (tenant admins manage their tenant through membership
/// checks, not the global admin API — LAW-158-6).
pub fn global_role_for_membership(role: MembershipRole) -> edgequake_auth::Role {
    match role {
        MembershipRole::Readonly => edgequake_auth::Role::Readonly,
        _ => edgequake_auth::Role::User,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn policy() -> FederationPolicy {
        FederationPolicy {
            role_map: HashMap::from([
                ("eq-admin".to_string(), MembershipRole::Admin),
                ("eq-owner".to_string(), MembershipRole::Owner),
                ("eq-ro".to_string(), MembershipRole::Readonly),
            ]),
            ..FederationPolicy::default()
        }
    }

    fn with_roles(roles: &[&str]) -> FederatedClaims {
        FederatedClaims {
            issuer: "i".into(),
            subject: "s".into(),
            email: None,
            email_verified: false,
            preferred_username: None,
            name: None,
            organizations: vec![],
            roles: roles.iter().map(|r| r.to_string()).collect(),
            hosted_domain: None,
            idp_tenant_id: None,
            session_id: None,
        }
    }

    #[test]
    fn picks_highest_mapped_role() {
        assert_eq!(
            resolve_membership_role(&policy(), &with_roles(&["eq-ro", "eq-admin"])),
            MembershipRole::Admin
        );
    }

    #[test]
    fn unmapped_falls_back_to_default() {
        assert_eq!(
            resolve_membership_role(&policy(), &with_roles(&["x"])),
            MembershipRole::Member
        );
    }

    #[test]
    fn owner_is_capped_by_max_role() {
        assert_eq!(
            resolve_membership_role(&policy(), &with_roles(&["eq-owner"])),
            MembershipRole::Admin
        );
        let mut p = policy();
        p.max_role = MembershipRole::Owner;
        assert_eq!(
            resolve_membership_role(&p, &with_roles(&["eq-owner"])),
            MembershipRole::Owner
        );
    }

    #[test]
    fn sso_never_yields_global_admin() {
        assert_eq!(
            global_role_for_membership(MembershipRole::Owner),
            edgequake_auth::Role::User
        );
        assert_eq!(
            global_role_for_membership(MembershipRole::Readonly),
            edgequake_auth::Role::Readonly
        );
    }
}
