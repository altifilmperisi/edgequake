//! SPEC-158 LAW-158-9 — SSO mode is fail-closed at startup.
//!
//! When any identity provider is runtime-active the deployment must (1) authenticate requests,
//! (2) bind JWT tenant claims to memberships, and (3) use `https` redirect URIs. Local-only
//! `EDGEQUAKE_DEV_MODE` relaxes all three to warnings (local development only).

use edgequake_auth::AuthConfig;

use crate::startup_security::StartupSecurityOutcome;
use crate::state::security_config::ApiSecurityConfig;

/// Facts about the active SSO configuration (pure input ⇒ trivially testable).
#[derive(Debug, Clone, Default)]
pub struct SsoStartupFacts {
    pub redirect_uris: Vec<String>,
}

impl SsoStartupFacts {
    pub fn is_active(&self) -> bool {
        !self.redirect_uris.is_empty()
    }
}

/// Tighten `security` for SSO: tenant binding is mandatory outside dev mode.
pub fn enforce_sso_security(
    mut security: ApiSecurityConfig,
    facts: &SsoStartupFacts,
    dev_mode: bool,
) -> ApiSecurityConfig {
    if facts.is_active() && !dev_mode {
        security.strict_tenant_bind = true;
    }
    security
}

fn is_secure_redirect(uri: &str) -> bool {
    let lower = uri.to_ascii_lowercase();
    lower.starts_with("https://")
        || lower.starts_with("http://localhost")
        || lower.starts_with("http://127.0.0.1")
}

/// Validate SSO prerequisites. Returns `Ok` when SSO is inactive.
pub fn validate_sso_startup(
    facts: &SsoStartupFacts,
    auth: &AuthConfig,
    security: &ApiSecurityConfig,
) -> StartupSecurityOutcome {
    if !facts.is_active() {
        return StartupSecurityOutcome::Ok;
    }
    let mut warnings = Vec::new();
    if !auth.auth_enabled {
        let msg = "SSO providers are configured but authentication is disabled — enable auth (SPEC-158 LAW-158-9)";
        if auth.dev_mode {
            warnings.push(msg.to_string());
        } else {
            return StartupSecurityOutcome::Fatal(msg.to_string());
        }
    }
    if !security.strict_tenant_bind {
        let msg = "SSO requires EDGEQUAKE_STRICT_TENANT_BIND=true (SPEC-158 LAW-158-9)";
        if auth.dev_mode {
            warnings.push(msg.to_string());
        } else {
            return StartupSecurityOutcome::Fatal(msg.to_string());
        }
    }
    if let Some(bad) = facts.redirect_uris.iter().find(|u| !is_secure_redirect(u)) {
        let msg = format!("SSO redirect URI must use https: {bad} (SPEC-158 LAW-158-9)");
        if auth.dev_mode {
            warnings.push(msg);
        } else {
            return StartupSecurityOutcome::Fatal(msg);
        }
    }
    if warnings.is_empty() {
        StartupSecurityOutcome::Ok
    } else {
        StartupSecurityOutcome::Warn(warnings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn auth(enabled: bool, dev: bool) -> AuthConfig {
        let mut a = AuthConfig::new("secure-test-secret-spec158-long-enough-xx");
        a.auth_enabled = enabled;
        a.dev_mode = dev;
        a
    }

    fn facts(uri: &str) -> SsoStartupFacts {
        SsoStartupFacts {
            redirect_uris: vec![uri.to_string()],
        }
    }

    fn strict() -> ApiSecurityConfig {
        ApiSecurityConfig {
            strict_tenant_bind: true,
            ..Default::default()
        }
    }

    #[test]
    fn inactive_sso_is_ok_even_without_auth() {
        let out = validate_sso_startup(
            &SsoStartupFacts::default(),
            &auth(false, false),
            &ApiSecurityConfig::default(),
        );
        assert_eq!(out, StartupSecurityOutcome::Ok);
    }

    #[test]
    fn sso_without_auth_is_fatal_outside_dev_mode() {
        let out = validate_sso_startup(&facts("https://a/cb"), &auth(false, false), &strict());
        assert!(matches!(out, StartupSecurityOutcome::Fatal(_)));
        let out = validate_sso_startup(&facts("https://a/cb"), &auth(false, true), &strict());
        assert!(matches!(out, StartupSecurityOutcome::Warn(_)));
    }

    #[test]
    fn sso_without_strict_bind_is_fatal_outside_dev() {
        let out = validate_sso_startup(
            &facts("https://a/cb"),
            &auth(true, false),
            &ApiSecurityConfig::default(),
        );
        assert!(matches!(out, StartupSecurityOutcome::Fatal(_)));
        let out = validate_sso_startup(
            &facts("https://a/cb"),
            &auth(true, true),
            &ApiSecurityConfig::default(),
        );
        assert!(matches!(out, StartupSecurityOutcome::Warn(_)));
    }

    #[test]
    fn http_redirect_fatal_outside_dev_but_localhost_allowed() {
        let out = validate_sso_startup(
            &facts("http://idp.example/cb"),
            &auth(true, false),
            &strict(),
        );
        assert!(matches!(out, StartupSecurityOutcome::Fatal(_)));
        let out = validate_sso_startup(
            &facts("http://localhost:8080/cb"),
            &auth(true, false),
            &strict(),
        );
        assert_eq!(out, StartupSecurityOutcome::Ok);
    }

    #[test]
    fn enforce_forces_strict_bind_only_when_active_and_not_dev() {
        let f = facts("https://a/cb");
        assert!(enforce_sso_security(ApiSecurityConfig::default(), &f, false).strict_tenant_bind);
        assert!(!enforce_sso_security(ApiSecurityConfig::default(), &f, true).strict_tenant_bind);
        assert!(
            !enforce_sso_security(
                ApiSecurityConfig::default(),
                &SsoStartupFacts::default(),
                false
            )
            .strict_tenant_bind
        );
    }
}
