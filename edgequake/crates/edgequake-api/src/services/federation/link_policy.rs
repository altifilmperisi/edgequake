//! Account-linking and provider gate decisions (SPEC-158 LAW-158-3, LAW-158-12).
//!
//! Pure functions — no I/O — so every branch is unit-testable.

use super::claims::FederatedClaims;
use super::policy::{FederationPolicy, LinkPolicy};
use super::username::is_synthetic_email;

/// Why a federated assertion was refused before any account work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderGateDenial {
    HostedDomainMismatch,
    IdpTenantNotAllowed,
}

impl ProviderGateDenial {
    pub fn code(&self) -> &'static str {
        match self {
            Self::HostedDomainMismatch => "hd_mismatch",
            Self::IdpTenantNotAllowed => "idp_tenant_not_allowed",
        }
    }
}

/// May an email match attach this IdP identity to an *existing* local user?
///
/// True only when policy is `VerifiedEmail`, the provider is `trust_email`, the token says
/// `email_verified`, and the email is a real (non-synthetic) address.
pub fn may_link_by_email(policy: &FederationPolicy, claims: &FederatedClaims) -> bool {
    policy.link_policy == LinkPolicy::VerifiedEmail
        && policy.trust_email
        && claims.email_verified
        && claims
            .email
            .as_deref()
            .map(|e| !e.is_empty() && !is_synthetic_email(e))
            .unwrap_or(false)
}

/// Provider-specific allow-lists (Google `hd`, Entra `tid`).
pub fn check_provider_gate(
    policy: &FederationPolicy,
    claims: &FederatedClaims,
) -> Result<(), ProviderGateDenial> {
    if !policy.allowed_hosted_domains.is_empty() {
        let ok = claims
            .hosted_domain
            .as_deref()
            .map(|hd| {
                policy
                    .allowed_hosted_domains
                    .iter()
                    .any(|a| a.eq_ignore_ascii_case(hd))
            })
            .unwrap_or(false);
        if !ok {
            return Err(ProviderGateDenial::HostedDomainMismatch);
        }
    }
    if !policy.allowed_idp_tenant_ids.is_empty() {
        let ok = claims
            .idp_tenant_id
            .as_deref()
            .map(|tid| {
                policy
                    .allowed_idp_tenant_ids
                    .iter()
                    .any(|a| a.eq_ignore_ascii_case(tid))
            })
            .unwrap_or(false);
        if !ok {
            return Err(ProviderGateDenial::IdpTenantNotAllowed);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claims(email: Option<&str>, verified: bool) -> FederatedClaims {
        FederatedClaims {
            issuer: "i".into(),
            subject: "s".into(),
            email: email.map(Into::into),
            email_verified: verified,
            preferred_username: None,
            name: None,
            organizations: vec![],
            roles: vec![],
            hosted_domain: None,
            idp_tenant_id: None,
            session_id: None,
        }
    }

    fn trusting() -> FederationPolicy {
        FederationPolicy {
            trust_email: true,
            link_policy: LinkPolicy::VerifiedEmail,
            ..FederationPolicy::default()
        }
    }

    #[test]
    fn default_policy_never_links() {
        assert!(!may_link_by_email(
            &FederationPolicy::default(),
            &claims(Some("a@b.c"), true)
        ));
    }

    #[test]
    fn links_only_when_verified_and_trusted() {
        assert!(may_link_by_email(&trusting(), &claims(Some("a@b.c"), true)));
        assert!(!may_link_by_email(
            &trusting(),
            &claims(Some("a@b.c"), false)
        ));
        let mut untrusted = trusting();
        untrusted.trust_email = false;
        assert!(!may_link_by_email(&untrusted, &claims(Some("a@b.c"), true)));
        assert!(!may_link_by_email(&trusting(), &claims(None, true)));
        assert!(!may_link_by_email(
            &trusting(),
            &claims(Some("x@kc.sso.invalid"), true)
        ));
    }

    #[test]
    fn hosted_domain_gate() {
        let p = FederationPolicy {
            allowed_hosted_domains: vec!["example.com".into()],
            ..FederationPolicy::default()
        };
        let mut c = claims(Some("a@example.com"), true);
        assert_eq!(
            check_provider_gate(&p, &c),
            Err(ProviderGateDenial::HostedDomainMismatch)
        );
        c.hosted_domain = Some("other.com".into());
        assert!(check_provider_gate(&p, &c).is_err());
        c.hosted_domain = Some("Example.com".into());
        assert!(check_provider_gate(&p, &c).is_ok());
    }

    #[test]
    fn entra_tid_gate() {
        let p = FederationPolicy {
            allowed_idp_tenant_ids: vec!["home-tid".into()],
            ..FederationPolicy::default()
        };
        let mut c = claims(None, false);
        c.idp_tenant_id = Some("guest-tid".into());
        assert_eq!(
            check_provider_gate(&p, &c),
            Err(ProviderGateDenial::IdpTenantNotAllowed)
        );
        c.idp_tenant_id = Some("home-tid".into());
        assert!(check_provider_gate(&p, &c).is_ok());
    }
}
