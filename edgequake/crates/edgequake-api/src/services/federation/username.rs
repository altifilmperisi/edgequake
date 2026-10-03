//! Deterministic username / synthetic email derivation (SPEC-158 EC-158-03, EC-158-05).

/// Sanitise an IdP-provided handle into `[a-z0-9._-]` (max 40 chars).
pub fn sanitize_username(raw: &str) -> String {
    let cleaned: String = raw
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '_'
            }
        })
        .take(40)
        .collect();
    if cleaned.is_empty() {
        "sso_user".to_string()
    } else {
        cleaned
    }
}

/// First free username: `base`, then `base-<n>` for n in 2.. — deterministic, never errors.
pub fn first_free_username(base: &str, is_taken: impl Fn(&str) -> bool) -> String {
    let base = sanitize_username(base);
    if !is_taken(&base) {
        return base;
    }
    (2u32..)
        .map(|n| format!("{base}-{n}"))
        .find(|candidate| !is_taken(candidate))
        .unwrap_or(base)
}

/// Email used when the IdP supplies none. The `.invalid` TLD (RFC 2606) can never collide with a
/// real mailbox and is never used for linking (LAW-158-3).
pub fn synthetic_email(provider_slug: &str, subject: &str) -> String {
    let local: String = subject
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .take(48)
        .collect();
    let slug = sanitize_username(provider_slug);
    format!("{local}@{slug}.sso.invalid")
}

/// True for addresses created by [`synthetic_email`].
pub fn is_synthetic_email(email: &str) -> bool {
    email.to_ascii_lowercase().ends_with(".sso.invalid")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_replaces_and_defaults() {
        assert_eq!(sanitize_username(" Alice Smith@x "), "alice_smith_x");
        assert_eq!(sanitize_username("   "), "sso_user");
    }

    #[test]
    fn suffixes_on_collision() {
        let taken = ["alice".to_string(), "alice-2".to_string()];
        let name = first_free_username("Alice", |c| taken.iter().any(|t| t == c));
        assert_eq!(name, "alice-3");
    }

    #[test]
    fn synthetic_email_is_flagged_and_invalid_tld() {
        let e = synthetic_email("keycloak", "abc-123");
        assert_eq!(e, "abc-123@keycloak.sso.invalid");
        assert!(is_synthetic_email(&e));
        assert!(!is_synthetic_email("a@example.com"));
    }
}
