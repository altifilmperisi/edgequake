//! Normalised federated claims (SPEC-158 LAW-158-3 / LAW-158-12).
//!
//! The OIDC library verifies signature, issuer, audience, nonce and expiry. This module only
//! *reads* provider-specific claims out of the already-verified payload and normalises them so
//! the rest of the federation stack never touches raw JSON.

use serde_json::Value;

use super::org_claim::{parse_organization_claim, OrgRef};

/// Provider-agnostic identity assertion. The federation key is `(issuer, subject)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FederatedClaims {
    pub issuer: String,
    pub subject: String,
    pub email: Option<String>,
    pub email_verified: bool,
    pub preferred_username: Option<String>,
    pub name: Option<String>,
    pub organizations: Vec<OrgRef>,
    pub roles: Vec<String>,
    /// Google Workspace hosted domain (`hd`).
    pub hosted_domain: Option<String>,
    /// Entra tenant id (`tid`).
    pub idp_tenant_id: Option<String>,
    /// IdP session id (`sid`) — used for back-channel logout.
    pub session_id: Option<String>,
}

impl FederatedClaims {
    /// Build from a verified id_token payload. `role_claim` is a dotted path
    /// (`realm_access.roles`, `roles`, `groups`, `cognito:groups`).
    pub fn from_payload(issuer: &str, subject: &str, payload: &Value, role_claim: &str) -> Self {
        Self {
            issuer: issuer.to_string(),
            subject: subject.to_string(),
            email: string_claim(payload, "email").map(|e| e.trim().to_ascii_lowercase()),
            email_verified: bool_claim(payload, "email_verified"),
            preferred_username: string_claim(payload, "preferred_username"),
            name: string_claim(payload, "name"),
            organizations: payload
                .get("organization")
                .map(parse_organization_claim)
                .unwrap_or_default(),
            roles: string_list_at_path(payload, role_claim),
            hosted_domain: string_claim(payload, "hd"),
            idp_tenant_id: string_claim(payload, "tid"),
            session_id: string_claim(payload, "sid"),
        }
    }

    /// Email domain (lower-case) when an email is present.
    pub fn email_domain(&self) -> Option<&str> {
        self.email.as_deref()?.rsplit_once('@').map(|(_, d)| d)
    }
}

fn string_claim(payload: &Value, key: &str) -> Option<String> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
}

/// `email_verified` may be a JSON bool or the string `"true"` (Cognito, some brokers).
fn bool_claim(payload: &Value, key: &str) -> bool {
    match payload.get(key) {
        Some(Value::Bool(b)) => *b,
        Some(Value::String(s)) => s.eq_ignore_ascii_case("true"),
        _ => false,
    }
}

/// Walk a dotted path; accept JSON arrays of strings or a delimited string.
pub fn string_list_at_path(payload: &Value, path: &str) -> Vec<String> {
    if path.trim().is_empty() {
        return Vec::new();
    }
    // A literal top-level key containing dots/colons (e.g. `cognito:groups`) wins.
    let direct = payload.get(path);
    let node = direct.or_else(|| {
        path.split('.')
            .try_fold(payload, |cur, segment| cur.get(segment))
    });
    match node {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(Value::as_str)
            .map(ToOwned::to_owned)
            .collect(),
        Some(Value::String(s)) => s
            .split(|c: char| c == ',' || c.is_whitespace())
            .filter(|p| !p.is_empty())
            .map(ToOwned::to_owned)
            .collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extracts_standard_and_provider_claims() {
        let payload = json!({
            "email": " Alice@Example.COM ",
            "email_verified": true,
            "preferred_username": "alice",
            "hd": "example.com",
            "tid": "tenant-guid",
            "sid": "sess-1",
            "organization": {"acme": {"id": "org-1"}},
            "realm_access": {"roles": ["eq-admin", "offline_access"]}
        });
        let c =
            FederatedClaims::from_payload("https://idp", "sub-1", &payload, "realm_access.roles");
        assert_eq!(c.email.as_deref(), Some("alice@example.com"));
        assert!(c.email_verified);
        assert_eq!(c.hosted_domain.as_deref(), Some("example.com"));
        assert_eq!(c.session_id.as_deref(), Some("sess-1"));
        assert_eq!(c.organizations.len(), 1);
        assert_eq!(c.roles, vec!["eq-admin", "offline_access"]);
        assert_eq!(c.email_domain(), Some("example.com"));
    }

    #[test]
    fn email_verified_string_true_accepted_and_missing_is_false() {
        let c = FederatedClaims::from_payload(
            "i",
            "s",
            &json!({"email": "a@b.c", "email_verified": "true"}),
            "",
        );
        assert!(c.email_verified);
        let c = FederatedClaims::from_payload("i", "s", &json!({"email": "a@b.c"}), "");
        assert!(!c.email_verified);
    }

    #[test]
    fn colon_claim_names_resolve_directly() {
        let payload = json!({"cognito:groups": ["admins"]});
        assert_eq!(
            string_list_at_path(&payload, "cognito:groups"),
            vec!["admins"]
        );
    }

    #[test]
    fn delimited_string_roles_split() {
        let payload = json!({"roles": "a, b c"});
        assert_eq!(string_list_at_path(&payload, "roles"), vec!["a", "b", "c"]);
    }
}
