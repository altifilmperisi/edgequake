//! SPEC-154 Wave 6 — auth-off on non-local DB is fatal (EC-154-12 / EC-154-22).
//!
//! Named binary gate; implementation lives in `startup_security` unit tests.

use edgequake_api::startup_security::{validate_startup_security, StartupSecurityOutcome};
use edgequake_api::state::security_config::ApiSecurityConfig;
use edgequake_auth::AuthConfig;

#[test]
fn ec_154_12_non_local_auth_off_fatal() {
    let auth = AuthConfig {
        auth_enabled: false,
        dev_mode: false,
        jwt_secret: "secure-test-secret-spec154-wave6-long".to_string(),
        ..AuthConfig::default()
    };
    let security = ApiSecurityConfig {
        cors_origins: Some(vec!["https://app.example.com".into()]),
        ..Default::default()
    };
    let outcome = validate_startup_security(
        Some("postgres://user:pass@db.cloud.example:5432/edgequake"),
        &auth,
        &security,
    );
    assert!(
        matches!(outcome, StartupSecurityOutcome::Fatal(_)),
        "expected Fatal, got {outcome:?}"
    );
}

#[test]
fn ec_154_22_dev_mode_bypasses_auth_off_fatal() {
    let auth = AuthConfig {
        auth_enabled: false,
        dev_mode: true,
        jwt_secret: "secure-test-secret-spec154-wave6-long".to_string(),
        ..AuthConfig::default()
    };
    let security = ApiSecurityConfig {
        cors_origins: Some(vec!["https://app.example.com".into()]),
        ..Default::default()
    };
    let outcome = validate_startup_security(
        Some("postgres://user:pass@db.cloud.example:5432/edgequake"),
        &auth,
        &security,
    );
    assert!(
        !matches!(outcome, StartupSecurityOutcome::Fatal(_)),
        "DEV_MODE must allow auth-off on non-local URL: {outcome:?}"
    );
}
