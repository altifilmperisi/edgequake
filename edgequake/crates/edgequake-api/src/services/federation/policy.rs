//! Per-provider federation policy (SPEC-158 LAW-158-3/5/7/8/12).
//!
//! `OidcConfig` (edgequake-auth) answers *how to talk to the IdP*; `FederationPolicy` answers
//! *what we are willing to believe from it*. Defaults are the safest values (no email linking,
//! no IdP-granted owner).

use std::collections::HashMap;

use edgequake_core::MembershipRole;

/// Known provider families — only used to pick sane defaults, never for trust decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKind {
    Generic,
    Keycloak,
    Google,
    Entra,
    Cognito,
}

impl ProviderKind {
    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "keycloak" => Self::Keycloak,
            "google" => Self::Google,
            "entra" | "azure" | "microsoft" => Self::Entra,
            "cognito" | "aws" => Self::Cognito,
            _ => Self::Generic,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Generic => "generic",
            Self::Keycloak => "keycloak",
            Self::Google => "google",
            Self::Entra => "entra",
            Self::Cognito => "cognito",
        }
    }
}

/// Whether an email match may attach an IdP identity to an existing local user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkPolicy {
    /// Never link by email (default — LAW-158-3).
    Never,
    /// Link only when `email_verified` AND the provider is `trust_email`.
    VerifiedEmail,
}

impl LinkPolicy {
    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "verified_email" | "verified-email" => Self::VerifiedEmail,
            // `always` is intentionally unsupported (takeover vector) → falls back to Never.
            _ => Self::Never,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FederationPolicy {
    pub slug: String,
    pub display_name: String,
    pub kind: ProviderKind,
    pub trust_email: bool,
    pub link_policy: LinkPolicy,
    pub jit_enabled: bool,
    /// Deny logins with no organization claim (otherwise fall back to the default tenant).
    pub require_org: bool,
    /// Pin every login of this provider to one tenant slug.
    pub fixed_tenant_slug: Option<String>,
    /// Extra scopes beyond `openid email`.
    pub scopes: Vec<String>,
    /// Dotted claim path carrying IdP roles/groups.
    pub role_claim: String,
    pub role_map: HashMap<String, MembershipRole>,
    pub default_role: MembershipRole,
    /// IdP can never grant more than this (default Admin — Owner is explicit opt-in).
    pub max_role: MembershipRole,
    /// Google `hd` allow-list (EC-158-27). Non-empty ⇒ `hd` must match.
    pub allowed_hosted_domains: Vec<String>,
    /// Entra `tid` allow-list (EC-158-26). Non-empty ⇒ `tid` must match.
    pub allowed_idp_tenant_ids: Vec<String>,
}

impl Default for FederationPolicy {
    fn default() -> Self {
        Self::for_kind(ProviderKind::Generic)
    }
}

impl FederationPolicy {
    /// Kind-specific safe defaults.
    pub fn for_kind(kind: ProviderKind) -> Self {
        let (scopes, role_claim, require_org): (Vec<String>, &str, bool) = match kind {
            ProviderKind::Keycloak => (vec!["profile".into()], "realm_access.roles", true),
            ProviderKind::Google => (vec!["profile".into()], "", false),
            ProviderKind::Entra => (vec!["profile".into()], "roles", false),
            ProviderKind::Cognito => (vec!["profile".into()], "cognito:groups", false),
            ProviderKind::Generic => (Vec::new(), "", false),
        };
        Self {
            slug: if kind == ProviderKind::Generic {
                "oidc".into()
            } else {
                kind.as_str().into()
            },
            display_name: "Single sign-on".into(),
            kind,
            trust_email: false,
            link_policy: LinkPolicy::Never,
            jit_enabled: true,
            require_org,
            fixed_tenant_slug: None,
            scopes,
            role_claim: role_claim.into(),
            role_map: HashMap::new(),
            default_role: MembershipRole::Member,
            max_role: MembershipRole::Admin,
            allowed_hosted_domains: Vec::new(),
            allowed_idp_tenant_ids: Vec::new(),
        }
    }

    /// Load from `EDGEQUAKE_OIDC_*` (legacy shim, LAW-158-7) via an injectable getter.
    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Self {
        let kind = get("EDGEQUAKE_OIDC_KIND")
            .map(|k| ProviderKind::parse(&k))
            .unwrap_or(ProviderKind::Generic);
        let mut p = Self::for_kind(kind);
        let text = |key: &str| {
            get(key)
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        };
        let flag = |key: &str, default: bool| {
            text(key)
                .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
                .unwrap_or(default)
        };
        let list = |key: &str| -> Option<Vec<String>> {
            text(key).map(|v| {
                v.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
        };
        if let Some(v) = text("EDGEQUAKE_OIDC_SLUG") {
            p.slug = v.to_ascii_lowercase();
        }
        if let Some(v) = text("EDGEQUAKE_OIDC_DISPLAY_NAME") {
            p.display_name = v;
        }
        p.trust_email = flag("EDGEQUAKE_OIDC_TRUST_EMAIL", p.trust_email);
        if let Some(v) = text("EDGEQUAKE_OIDC_LINK_POLICY") {
            p.link_policy = LinkPolicy::parse(&v);
        }
        p.jit_enabled = flag("EDGEQUAKE_OIDC_JIT", p.jit_enabled);
        p.require_org = flag("EDGEQUAKE_OIDC_REQUIRE_ORG", p.require_org);
        p.fixed_tenant_slug = text("EDGEQUAKE_OIDC_TENANT_SLUG").map(|v| v.to_ascii_lowercase());
        if let Some(v) = list("EDGEQUAKE_OIDC_SCOPES") {
            p.scopes = v;
        }
        if let Some(v) = text("EDGEQUAKE_OIDC_ROLE_CLAIM") {
            p.role_claim = v;
        }
        if let Some(v) = text("EDGEQUAKE_OIDC_ROLE_MAP") {
            p.role_map = parse_role_map(&v);
        }
        if let Some(r) = text("EDGEQUAKE_OIDC_DEFAULT_ROLE").and_then(|v| v.parse().ok()) {
            p.default_role = r;
        }
        if let Some(r) = text("EDGEQUAKE_OIDC_MAX_ROLE").and_then(|v| v.parse().ok()) {
            p.max_role = r;
        }
        if let Some(v) = list("EDGEQUAKE_OIDC_ALLOWED_HD") {
            p.allowed_hosted_domains = v.into_iter().map(|d| d.to_ascii_lowercase()).collect();
        }
        if let Some(v) = list("EDGEQUAKE_OIDC_ALLOWED_TID") {
            p.allowed_idp_tenant_ids = v;
        }
        p
    }

    pub fn from_env() -> Self {
        Self::from_lookup(|k| std::env::var(k).ok())
    }
}

/// Parse `{"claim-value":"role"}`; unknown roles are dropped (fail-closed to default role).
pub fn parse_role_map(raw: &str) -> HashMap<String, MembershipRole> {
    serde_json::from_str::<HashMap<String, String>>(raw)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(claim, role)| role.parse().ok().map(|r| (claim, r)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_safest() {
        let p = FederationPolicy::default();
        assert!(!p.trust_email);
        assert_eq!(p.link_policy, LinkPolicy::Never);
        assert_eq!(p.max_role, MembershipRole::Admin);
        assert!(!p.require_org);
    }

    #[test]
    fn keycloak_kind_requires_org_and_realm_roles() {
        let p = FederationPolicy::for_kind(ProviderKind::Keycloak);
        assert!(p.require_org);
        assert_eq!(p.role_claim, "realm_access.roles");
    }

    #[test]
    fn env_lookup_overrides_and_always_policy_is_never() {
        let env: HashMap<&str, &str> = HashMap::from([
            ("EDGEQUAKE_OIDC_KIND", "google"),
            ("EDGEQUAKE_OIDC_TRUST_EMAIL", "true"),
            ("EDGEQUAKE_OIDC_LINK_POLICY", "always"),
            ("EDGEQUAKE_OIDC_ALLOWED_HD", "Example.com, corp.io"),
            (
                "EDGEQUAKE_OIDC_ROLE_MAP",
                r#"{"eq-admin":"admin","bogus":"god"}"#,
            ),
            ("EDGEQUAKE_OIDC_MAX_ROLE", "owner"),
        ]);
        let p = FederationPolicy::from_lookup(|k| env.get(k).map(|v| v.to_string()));
        assert_eq!(p.kind, ProviderKind::Google);
        assert!(p.trust_email);
        assert_eq!(p.link_policy, LinkPolicy::Never);
        assert_eq!(p.allowed_hosted_domains, vec!["example.com", "corp.io"]);
        assert_eq!(p.role_map.len(), 1);
        assert_eq!(p.max_role, MembershipRole::Owner);
    }
}
