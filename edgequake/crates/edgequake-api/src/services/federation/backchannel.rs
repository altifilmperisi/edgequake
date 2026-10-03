//! OIDC Back-Channel Logout 1.0 receiver (SPEC-158 LAW-158-10, EC-158-22/23).
//!
//! Validation follows OIDC BCL §2.6: signature (JWKS / client secret), `iss`, `aud`, `exp`/`iat`,
//! `events` contains the BCL event, **no** `nonce`, `jti` present and never replayed, and at least
//! one of `sid` / `sub`. Matching refresh families are revoked, and every access-token `jti`
//! recorded for those families is copied onto the SPEC-154 denylist (EC-158-22).

use chrono::{DateTime, TimeZone, Utc};
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde_json::Value;

use crate::services::oidc_flow::OidcFlowService;
use crate::state::{AppState, PostgresRuntime};
use axum::extract::FromRef;

use super::store::SessionSelector;

pub const BACKCHANNEL_LOGOUT_EVENT: &str = "http://schemas.openid.net/event/backchannel-logout";

#[derive(Debug, PartialEq, Eq)]
pub enum LogoutError {
    /// 400 `invalid_request` — never reveals which check failed beyond a short code.
    Invalid(&'static str),
    Internal(String),
}

#[derive(Debug, PartialEq, Eq)]
pub struct LogoutOutcome {
    pub revoked_sessions: usize,
}

/// Verify the logout token and revoke the sessions it names.
pub async fn process_logout_token(
    state: &AppState,
    token: &str,
) -> Result<LogoutOutcome, LogoutError> {
    let issuer = unverified_issuer(token).ok_or(LogoutError::Invalid("malformed_token"))?;
    let provider = state
        .auth
        .provider_for_issuer(&issuer)
        .ok_or(LogoutError::Invalid("unknown_issuer"))?;

    let claims = verify_signature_and_claims(&provider, token).await?;
    validate_logout_claims(&claims)?;

    let jti = claims["jti"]
        .as_str()
        .ok_or(LogoutError::Invalid("missing_jti"))?;
    let expires_at =
        exp_to_utc(&claims).unwrap_or_else(|| Utc::now() + chrono::Duration::minutes(10));
    let first_seen = state
        .auth
        .federation
        .record_logout_jti(&issuer, jti, expires_at)
        .await
        .map_err(|e| LogoutError::Internal(e.to_string()))?;
    if !first_seen {
        return Err(LogoutError::Invalid("replayed_token"));
    }

    revoke_selected(state, &issuer, &claims).await
}

fn unverified_issuer(token: &str) -> Option<String> {
    let payload = token.split('.').nth(1)?;
    use base64::Engine;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .ok()?;
    let value: Value = serde_json::from_slice(&bytes).ok()?;
    value.get("iss")?.as_str().map(str::to_string)
}

/// Pure claim-shape checks (no I/O) — unit-tested directly.
pub fn validate_logout_claims(claims: &Value) -> Result<(), LogoutError> {
    let has_event = claims
        .get("events")
        .and_then(|e| e.get(BACKCHANNEL_LOGOUT_EVENT))
        .is_some_and(Value::is_object);
    if !has_event {
        return Err(LogoutError::Invalid("missing_event"));
    }
    if claims.get("nonce").is_some() {
        return Err(LogoutError::Invalid("nonce_present"));
    }
    if claims.get("jti").and_then(Value::as_str).is_none() {
        return Err(LogoutError::Invalid("missing_jti"));
    }
    let has_sid = claims.get("sid").and_then(Value::as_str).is_some();
    let has_sub = claims.get("sub").and_then(Value::as_str).is_some();
    if !has_sid && !has_sub {
        return Err(LogoutError::Invalid("missing_sid_or_sub"));
    }
    Ok(())
}

fn exp_to_utc(claims: &Value) -> Option<DateTime<Utc>> {
    Utc.timestamp_opt(claims.get("exp")?.as_i64()?, 0).single()
}

async fn verify_signature_and_claims(
    provider: &OidcFlowService,
    token: &str,
) -> Result<Value, LogoutError> {
    let header = decode_header(token).map_err(|_| LogoutError::Invalid("malformed_token"))?;
    if matches!(
        header.alg,
        Algorithm::HS256 | Algorithm::HS384 | Algorithm::HS512
    ) {
        let secret = provider
            .config()
            .client_secret
            .as_deref()
            .ok_or(LogoutError::Invalid("no_verification_key"))?;
        return decode_with(
            provider,
            token,
            header.alg,
            &DecodingKey::from_secret(secret.as_bytes()),
        );
    }
    // Asymmetric: look the kid up in the cached JWKS, refresh once on miss (key rotation).
    for force in [false, true] {
        if let Some(key) = jwks_key(provider, header.kid.as_deref(), force).await? {
            return decode_with(provider, token, header.alg, &key);
        }
    }
    Err(LogoutError::Invalid("unknown_key"))
}

async fn jwks_key(
    provider: &OidcFlowService,
    kid: Option<&str>,
    force: bool,
) -> Result<Option<DecodingKey>, LogoutError> {
    let metadata = provider
        .metadata(force)
        .await
        .map_err(|e| LogoutError::Internal(e.to_string()))?;
    for jwk in metadata.jwks().keys() {
        let value = serde_json::to_value(jwk).map_err(|e| LogoutError::Internal(e.to_string()))?;
        let jwk_kid = value.get("kid").and_then(Value::as_str);
        if kid.is_some() && jwk_kid != kid {
            continue;
        }
        let parsed: jsonwebtoken::jwk::Jwk =
            serde_json::from_value(value).map_err(|_| LogoutError::Invalid("bad_jwk"))?;
        return DecodingKey::from_jwk(&parsed)
            .map(Some)
            .map_err(|_| LogoutError::Invalid("bad_jwk"));
    }
    Ok(None)
}

fn decode_with(
    provider: &OidcFlowService,
    token: &str,
    alg: Algorithm,
    key: &DecodingKey,
) -> Result<Value, LogoutError> {
    let mut validation = Validation::new(alg);
    validation.set_audience(&[provider.config().client_id.as_str()]);
    validation.set_issuer(&[provider.issuer()]);
    validation.set_required_spec_claims(&["iss", "aud", "exp"]);
    validation.leeway = 30;
    decode::<Value>(token, key, &validation)
        .map(|data| data.claims)
        .map_err(|_| LogoutError::Invalid("verification_failed"))
}

async fn revoke_selected(
    state: &AppState,
    issuer: &str,
    claims: &Value,
) -> Result<LogoutOutcome, LogoutError> {
    let selector = match (
        claims.get("sid").and_then(Value::as_str),
        claims.get("sub").and_then(Value::as_str),
    ) {
        (Some(sid), _) => SessionSelector::Sid {
            issuer: issuer.into(),
            sid: sid.into(),
        },
        (None, Some(sub)) => SessionSelector::Subject {
            issuer: issuer.into(),
            subject: sub.into(),
        },
        (None, None) => return Err(LogoutError::Invalid("missing_sid_or_sub")),
    };
    let internal = |e: crate::error::ApiError| LogoutError::Internal(e.to_string());
    let sessions = state
        .auth
        .federation
        .sessions_for(&selector)
        .await
        .map_err(internal)?;
    let pg = PostgresRuntime::from_ref(state);
    for session in &sessions {
        crate::services::session_storage::revoke_refresh_family(
            &state.storage,
            Some(&pg),
            &state.security,
            session.family_id,
        )
        .await
        .map_err(internal)?;
        let jtis = state
            .auth
            .federation
            .access_jtis(session.family_id)
            .await
            .map_err(internal)?;
        for (jti, expires_at) in jtis {
            crate::services::jti_denylist::revoke_jti_parts(
                &state.auth.jwt,
                pg.optional_pg_pool(),
                &jti,
                expires_at,
                "backchannel_logout",
            )
            .await
            .map_err(internal)?;
        }
        state
            .auth
            .federation
            .mark_session_revoked(session.family_id)
            .await
            .map_err(internal)?;
    }
    Ok(LogoutOutcome {
        revoked_sessions: sessions.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn good() -> Value {
        json!({
            "iss": "https://idp", "aud": "c", "iat": 1, "jti": "j1", "sid": "s1",
            "events": {BACKCHANNEL_LOGOUT_EVENT: {}}
        })
    }

    #[test]
    fn accepts_well_formed_claims() {
        assert!(validate_logout_claims(&good()).is_ok());
    }

    #[test]
    fn rejects_missing_event_nonce_jti_and_subject() {
        let mut c = good();
        c.as_object_mut().unwrap().remove("events");
        assert_eq!(
            validate_logout_claims(&c),
            Err(LogoutError::Invalid("missing_event"))
        );

        let mut c = good();
        c["nonce"] = json!("n");
        assert_eq!(
            validate_logout_claims(&c),
            Err(LogoutError::Invalid("nonce_present"))
        );

        let mut c = good();
        c.as_object_mut().unwrap().remove("jti");
        assert_eq!(
            validate_logout_claims(&c),
            Err(LogoutError::Invalid("missing_jti"))
        );

        let mut c = good();
        c.as_object_mut().unwrap().remove("sid");
        assert_eq!(
            validate_logout_claims(&c),
            Err(LogoutError::Invalid("missing_sid_or_sub"))
        );
        c["sub"] = json!("user");
        assert!(validate_logout_claims(&c).is_ok());
    }

    #[test]
    fn issuer_peek_requires_wellformed_payload() {
        assert!(unverified_issuer("a.b.c").is_none());
        use base64::Engine;
        let p = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(br#"{"iss":"https://x"}"#);
        assert_eq!(
            unverified_issuer(&format!("h.{p}.s")).as_deref(),
            Some("https://x")
        );
    }
}
