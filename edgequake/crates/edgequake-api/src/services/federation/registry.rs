//! Provider registry (SPEC-158 LAW-158-7): env shim + persisted `identity_providers`.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use edgequake_auth::OidcConfig;
use edgequake_core::MembershipRole;
use serde::Serialize;

use crate::services::oidc_flow::{OidcFlowService, SharedOidcFlowService};

use super::policy::{parse_role_map, FederationPolicy, LinkPolicy, ProviderKind};
use super::store::StoredProvider;

/// Public, secret-free provider description for the login page.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema, PartialEq, Eq)]
pub struct ProviderSummary {
    pub slug: String,
    pub display_name: String,
    pub kind: String,
    /// Relative login entrypoint (the SPA appends `org` / `redirect`).
    pub login_path: String,
}

impl ProviderSummary {
    pub fn of(service: &OidcFlowService) -> Self {
        let policy = service.policy();
        Self {
            slug: policy.slug.clone(),
            display_name: policy.display_name.clone(),
            kind: policy.kind.as_str().to_string(),
            login_path: format!("/api/v1/auth/oidc/login?provider={}", policy.slug),
        }
    }
}

#[derive(Default)]
pub struct ProviderRegistry {
    inner: RwLock<HashMap<String, SharedOidcFlowService>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&self, service: SharedOidcFlowService) {
        if let Ok(mut map) = self.inner.write() {
            map.insert(service.slug().to_string(), service);
        }
    }

    pub fn get(&self, slug: &str) -> Option<SharedOidcFlowService> {
        self.inner.read().ok()?.get(slug).cloned()
    }

    pub fn remove(&self, slug: &str) {
        if let Ok(mut map) = self.inner.write() {
            map.remove(slug);
        }
    }

    /// Atomically swap the whole set (reload from the store).
    pub fn replace_all(&self, services: Vec<SharedOidcFlowService>) {
        if let Ok(mut map) = self.inner.write() {
            *map = services
                .into_iter()
                .map(|s| (s.slug().to_string(), s))
                .collect();
        }
    }

    pub fn all(&self) -> Vec<SharedOidcFlowService> {
        let mut all: Vec<_> = self
            .inner
            .read()
            .map(|m| m.values().cloned().collect())
            .unwrap_or_default();
        all.sort_by(|a: &SharedOidcFlowService, b| a.slug().cmp(b.slug()));
        all
    }
}

/// Build a runtime provider from a persisted row. `secret_lookup` resolves
/// `client_secret_ref` (an env var name) so secrets never live in the database.
pub fn service_from_stored(
    row: &StoredProvider,
    secret_lookup: impl Fn(&str) -> Option<String>,
) -> SharedOidcFlowService {
    let meta = &row.metadata;
    let string_list = |key: &str| -> Vec<String> {
        meta.get(key)
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default()
    };
    let kind = ProviderKind::parse(&row.kind);
    let mut policy = FederationPolicy::for_kind(kind);
    policy.slug = row.slug.clone();
    policy.display_name = row.display_name.clone();
    policy.trust_email = row.trust_email;
    policy.link_policy = LinkPolicy::parse(&row.link_policy);
    policy.jit_enabled = row.jit_enabled;
    if !row.scopes.is_empty() {
        policy.scopes = row.scopes.clone();
    }
    if !row.role_claim.is_empty() {
        policy.role_claim = row.role_claim.clone();
    }
    policy.role_map = parse_role_map(&row.role_map.to_string());
    policy.max_role = row.max_role.parse().unwrap_or(MembershipRole::Admin);
    if let Some(v) = meta.get("require_org").and_then(|v| v.as_bool()) {
        policy.require_org = v;
    }
    policy.fixed_tenant_slug = meta
        .get("tenant_slug")
        .and_then(|v| v.as_str())
        .map(|s| s.to_ascii_lowercase());
    policy.allowed_hosted_domains = string_list("allowed_hd")
        .into_iter()
        .map(|d| d.to_ascii_lowercase())
        .collect();
    policy.allowed_idp_tenant_ids = string_list("allowed_tid");

    let config = OidcConfig {
        enabled: row.enabled,
        issuer_url: row.issuer.clone(),
        client_id: row.client_id.clone(),
        client_secret: row
            .client_secret_ref
            .as_deref()
            .and_then(&secret_lookup)
            .filter(|s| !s.is_empty()),
        redirect_uri: row.redirect_uri.clone(),
        success_redirect_url: meta
            .get("success_redirect_url")
            .and_then(|v| v.as_str())
            .map(str::to_string),
    };
    Arc::new(OidcFlowService::with_policy(config, policy))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn row() -> StoredProvider {
        StoredProvider {
            slug: "corp-kc".into(),
            kind: "keycloak".into(),
            display_name: "Corp SSO".into(),
            issuer: "https://kc.example/realms/eq".into(),
            client_id: "edgequake".into(),
            client_secret_ref: Some("TEST_SECRET_REF".into()),
            redirect_uri: "https://eq.example/api/v1/auth/oidc/callback".into(),
            scopes: vec![],
            trust_email: true,
            link_policy: "verified_email".into(),
            jit_enabled: true,
            role_claim: String::new(),
            role_map: json!({"eq-admin": "admin"}),
            max_role: "owner".into(),
            enabled: true,
            metadata: json!({"require_org": true, "allowed_hd": ["Example.com"], "tenant_slug": "Acme"}),
        }
    }

    #[test]
    fn stored_row_maps_to_policy_and_config() {
        let svc = service_from_stored(&row(), |name| {
            (name == "TEST_SECRET_REF").then(|| "s3cret".into())
        });
        assert_eq!(svc.slug(), "corp-kc");
        assert_eq!(svc.config().client_secret.as_deref(), Some("s3cret"));
        let p = svc.policy();
        assert_eq!(p.kind, ProviderKind::Keycloak);
        assert_eq!(p.link_policy, LinkPolicy::VerifiedEmail);
        assert_eq!(p.max_role, MembershipRole::Owner);
        assert_eq!(p.fixed_tenant_slug.as_deref(), Some("acme"));
        assert_eq!(p.allowed_hosted_domains, vec!["example.com"]);
        assert_eq!(p.role_claim, "realm_access.roles");
        assert!(svc.config().is_runtime_builtin());
    }

    #[test]
    fn registry_insert_get_replace() {
        let reg = ProviderRegistry::new();
        let svc = service_from_stored(&row(), |_| None);
        reg.insert(svc);
        assert!(reg.get("corp-kc").is_some());
        assert_eq!(ProviderSummary::of(&reg.all()[0]).kind, "keycloak");
        reg.replace_all(vec![]);
        assert!(reg.get("corp-kc").is_none());
    }
}
