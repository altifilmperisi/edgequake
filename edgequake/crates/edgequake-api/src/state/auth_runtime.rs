//! Authentication service runtime bundle (SPEC-017 P1-04).

use std::sync::Arc;

use edgequake_auth::{AuthConfig, JwtService, OidcConfig, PasswordService, RbacService};

use crate::services::federation::policy::FederationPolicy;
use crate::services::federation::registry::{service_from_stored, ProviderRegistry};
use crate::services::federation::{MemoryFederationStore, SharedFederationStore};
use crate::services::oidc_flow::{OidcFlowService, SharedOidcFlowService};

/// JWT, password hashing, RBAC, and optional OIDC services.
#[derive(Clone)]
pub struct AuthRuntime {
    pub config: AuthConfig,
    pub oidc_config: OidcConfig,
    /// Default provider (legacy `EDGEQUAKE_OIDC_*` shim — LAW-158-7).
    pub oidc_service: Option<SharedOidcFlowService>,
    /// Additional providers loaded from `identity_providers` (SPEC-158).
    pub providers: Arc<ProviderRegistry>,
    /// Durable federation state: identities, login attempts, handoffs, sessions (SPEC-158).
    pub federation: SharedFederationStore,
    pub jwt: Arc<JwtService>,
    pub password: Arc<PasswordService>,
    pub rbac: Arc<RbacService>,
}

impl AuthRuntime {
    pub fn new(config: AuthConfig) -> Self {
        let oidc_config = OidcConfig::from_env();
        let oidc_service = if oidc_config.is_runtime_builtin() {
            Some(Arc::new(OidcFlowService::with_policy(
                oidc_config.clone(),
                FederationPolicy::from_env(),
            )))
        } else {
            None
        };
        Self {
            jwt: Arc::new(JwtService::new(config.clone())),
            password: Arc::new(PasswordService::new(config.clone())),
            rbac: Arc::new(RbacService::new()),
            config,
            oidc_config,
            oidc_service,
            providers: Arc::new(ProviderRegistry::new()),
            federation: Arc::new(MemoryFederationStore::new()),
        }
    }

    /// Swap the federation store (PostgreSQL adapter in production — LAW-158-11).
    pub fn with_federation_store(mut self, store: SharedFederationStore) -> Self {
        self.federation = store;
        self
    }

    /// Every configured provider: the env shim first, then registry rows (unique by slug).
    pub fn all_providers(&self) -> Vec<SharedOidcFlowService> {
        let mut all: Vec<SharedOidcFlowService> = self.oidc_service.iter().cloned().collect();
        for svc in self.providers.all() {
            if !all.iter().any(|s| s.slug() == svc.slug()) {
                all.push(svc);
            }
        }
        all
    }

    pub fn provider(&self, slug: &str) -> Option<SharedOidcFlowService> {
        self.all_providers().into_iter().find(|s| s.slug() == slug)
    }

    /// Provider for `slug`, or the default (env shim, else the first registry entry).
    pub fn provider_or_default(&self, slug: Option<&str>) -> Option<SharedOidcFlowService> {
        match slug.map(str::trim).filter(|s| !s.is_empty()) {
            Some(slug) => self.provider(slug),
            None => self.all_providers().into_iter().next(),
        }
    }

    pub fn provider_for_issuer(&self, issuer: &str) -> Option<SharedOidcFlowService> {
        self.all_providers()
            .into_iter()
            .find(|s| s.issuer() == issuer)
    }

    /// True when any SSO provider can start a login (drives startup gates).
    /// Redirect URIs of every active provider (inputs to the SPEC-158 startup gates).
    pub fn startup_facts(&self) -> crate::startup_sso::SsoStartupFacts {
        crate::startup_sso::SsoStartupFacts {
            redirect_uris: self
                .all_providers()
                .iter()
                .map(|p| p.config().redirect_uri.clone())
                .collect(),
        }
    }

    /// Built-in auth mechanisms, including `oidc` when any provider (env or registry) is active.
    pub fn auth_mechanisms(&self) -> Vec<String> {
        let mut mechanisms = self.oidc_config.resolved_auth_mechanisms();
        if self.sso_active() && !mechanisms.iter().any(|m| m == "oidc") {
            mechanisms.push("oidc".to_string());
        }
        mechanisms
    }

    pub fn sso_active(&self) -> bool {
        !self.all_providers().is_empty()
    }

    /// Reload registry providers from the store (startup, admin changes).
    pub async fn reload_providers(&self) -> Result<usize, crate::error::ApiError> {
        let rows = self.federation.list_providers().await?;
        let services: Vec<SharedOidcFlowService> = rows
            .iter()
            .filter(|r| r.enabled)
            .map(|r| service_from_stored(r, |name| std::env::var(name).ok()))
            .filter(|svc| svc.config().is_runtime_builtin())
            .collect();
        let count = services.len();
        self.providers.replace_all(services);
        Ok(count)
    }

    pub fn from_env() -> Self {
        Self::new(AuthConfig::from_env())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_runtime_from_default_config() {
        let runtime = AuthRuntime::new(AuthConfig::default());
        assert!(runtime.oidc_service.is_none());
        assert!(Arc::strong_count(&runtime.jwt) >= 1);
    }
}
