//! OIDC authorization-code flow with PKCE (SPEC-027 phase 54, generalised by SPEC-158).
//!
//! One instance = one identity provider. Discovery + JWKS are cached (LAW-158-7) and refreshed
//! once when id_token verification fails (key rotation, EC-158-15). The library verifies
//! signature / issuer / audience / nonce / expiry; this module then lifts provider-specific
//! claims out of the *already verified* payload into [`FederatedClaims`].

use std::sync::Arc;
use std::time::{Duration, Instant};

use base64::Engine;
use openidconnect::core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata};
use openidconnect::{
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, Scope, TokenResponse,
};
use reqwest::redirect::Policy;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::RwLock;

use edgequake_auth::OidcConfig;

use super::federation::claims::FederatedClaims;
use super::federation::policy::{FederationPolicy, ProviderKind};

const DISCOVERY_TTL: Duration = Duration::from_secs(600);

/// EC-158-17: OIDC Core checks `exp` with no skew. Accept 30s of clock drift.
const ID_TOKEN_SKEW: chrono::Duration = chrono::Duration::seconds(30);

fn with_clock_skew<'a, K>(
    verifier: openidconnect::IdTokenVerifier<'a, K>,
) -> openidconnect::IdTokenVerifier<'a, K>
where
    K: openidconnect::JsonWebKey,
{
    verifier
        .set_time_fn(|| chrono::Utc::now() - ID_TOKEN_SKEW)
        .set_issue_time_verifier_fn(|issued| {
            if issued > chrono::Utc::now() + ID_TOKEN_SKEW {
                Err("iat_skew".to_string())
            } else {
                Ok(())
            }
        })
}

#[derive(Debug, Error)]
pub enum OidcServiceError {
    #[error("OIDC not configured")]
    NotConfigured,
    #[error("OIDC provider error: {0}")]
    Provider(String),
    #[error("OIDC state mismatch")]
    StateMismatch,
    /// Discovery / network failure — IdP unreachable (EC-158-16: break-glass password still works).
    #[error("OIDC provider unavailable: {0}")]
    Unavailable(String),
    /// IdP rejected the code or the id_token failed verification.
    #[error("OIDC provider rejected the login: {0}")]
    Rejected(String),
}

/// OIDC pending session produced by [`OidcFlowService::begin_login`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcPendingSession {
    pub csrf_token: String,
    pub pkce_verifier: String,
    pub nonce: String,
}

/// Result of starting an OIDC login.
pub struct OidcLoginStart {
    pub authorization_url: String,
    pub pending: OidcPendingSession,
}

/// Verified identity from the IdP.
#[derive(Debug, Clone)]
pub struct OidcIdentity {
    pub claims: FederatedClaims,
}

struct CachedDiscovery {
    fetched_at: Instant,
    metadata: CoreProviderMetadata,
}

pub struct OidcFlowService {
    config: OidcConfig,
    policy: FederationPolicy,
    http_client: reqwest::Client,
    discovery: RwLock<Option<CachedDiscovery>>,
}

impl OidcFlowService {
    /// Legacy constructor: safest default policy (SPEC-027 call sites, tests).
    pub fn new(config: OidcConfig) -> Self {
        Self::with_policy(config, FederationPolicy::default())
    }

    pub fn with_policy(config: OidcConfig, policy: FederationPolicy) -> Self {
        let http_client = reqwest::ClientBuilder::new()
            .redirect(Policy::none())
            .timeout(Duration::from_secs(15))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            config,
            policy,
            http_client,
            discovery: RwLock::new(None),
        }
    }

    pub fn config(&self) -> &OidcConfig {
        &self.config
    }

    pub fn policy(&self) -> &FederationPolicy {
        &self.policy
    }

    pub fn slug(&self) -> &str {
        &self.policy.slug
    }

    pub fn issuer(&self) -> &str {
        &self.config.issuer_url
    }

    /// Provider metadata (discovery + JWKS), cached; `force` bypasses the cache.
    pub async fn metadata(&self, force: bool) -> Result<CoreProviderMetadata, OidcServiceError> {
        if !force {
            if let Some(cached) = self.discovery.read().await.as_ref() {
                if cached.fetched_at.elapsed() < DISCOVERY_TTL {
                    return Ok(cached.metadata.clone());
                }
            }
        }
        let issuer = IssuerUrl::new(self.config.issuer_url.clone())
            .map_err(|e| OidcServiceError::Provider(format!("invalid issuer URL: {e}")))?;
        let metadata = CoreProviderMetadata::discover_async(issuer, &self.http_client)
            .await
            .map_err(|e| OidcServiceError::Unavailable(format!("discovery failed: {e}")))?;
        *self.discovery.write().await = Some(CachedDiscovery {
            fetched_at: Instant::now(),
            metadata: metadata.clone(),
        });
        Ok(metadata)
    }

    fn client(
        &self,
        metadata: CoreProviderMetadata,
    ) -> Result<
        CoreClient<
            openidconnect::EndpointSet,
            openidconnect::EndpointNotSet,
            openidconnect::EndpointNotSet,
            openidconnect::EndpointNotSet,
            openidconnect::EndpointMaybeSet,
            openidconnect::EndpointMaybeSet,
        >,
        OidcServiceError,
    > {
        let secret = self
            .config
            .client_secret
            .as_ref()
            .map(|s| ClientSecret::new(s.clone()));
        let redirect = RedirectUrl::new(self.config.redirect_uri.clone())
            .map_err(|e| OidcServiceError::Provider(format!("invalid redirect URI: {e}")))?;
        Ok(CoreClient::from_provider_metadata(
            metadata,
            ClientId::new(self.config.client_id.clone()),
            secret,
        )
        .set_redirect_uri(redirect))
    }

    /// Scopes beyond `openid email`, including the Keycloak organization scope.
    pub fn requested_scopes(&self, org_hint: Option<&str>) -> Vec<String> {
        let mut scopes = vec!["openid".to_string(), "email".to_string()];
        for s in &self.policy.scopes {
            if !scopes.contains(s) {
                scopes.push(s.clone());
            }
        }
        if self.policy.kind == ProviderKind::Keycloak {
            scopes.push(match org_hint {
                Some(alias) if !alias.is_empty() => format!("organization:{alias}"),
                _ => "organization:*".to_string(),
            });
        }
        scopes
    }

    pub async fn begin_login(
        &self,
        org_hint: Option<&str>,
    ) -> Result<OidcLoginStart, OidcServiceError> {
        if !self.config.is_runtime_builtin() {
            return Err(OidcServiceError::NotConfigured);
        }
        let client = self.client(self.metadata(false).await?)?;
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
        let mut request = client.authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        );
        for scope in self.requested_scopes(org_hint) {
            if scope != "openid" {
                request = request.add_scope(Scope::new(scope));
            }
        }
        let (auth_url, csrf_token, nonce) = request.set_pkce_challenge(pkce_challenge).url();
        Ok(OidcLoginStart {
            authorization_url: auth_url.to_string(),
            pending: OidcPendingSession {
                csrf_token: csrf_token.secret().to_string(),
                pkce_verifier: pkce_verifier.secret().to_string(),
                nonce: nonce.secret().to_string(),
            },
        })
    }

    pub async fn complete_login(
        &self,
        code: &str,
        state: &str,
        pending: &OidcPendingSession,
    ) -> Result<OidcIdentity, OidcServiceError> {
        if pending.csrf_token != state {
            return Err(OidcServiceError::StateMismatch);
        }
        let client = self.client(self.metadata(false).await?)?;
        let token_response = client
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .map_err(|e| OidcServiceError::Provider(format!("token exchange setup: {e}")))?
            .set_pkce_verifier(PkceCodeVerifier::new(pending.pkce_verifier.clone()))
            .request_async(&self.http_client)
            .await
            .map_err(|e| OidcServiceError::Rejected(format!("token exchange failed: {e}")))?;
        let id_token = token_response
            .id_token()
            .ok_or_else(|| OidcServiceError::Rejected("missing id_token".into()))?;
        let nonce = Nonce::new(pending.nonce.clone());

        let verifier = with_clock_skew(client.id_token_verifier());
        let subject = match id_token.claims(&verifier, &nonce) {
            Ok(claims) => claims.subject().to_string(),
            Err(_first) => {
                // EC-158-15: IdP may have rotated signing keys since the cached JWKS.
                let fresh = self.client(self.metadata(true).await?)?;
                let verifier = with_clock_skew(fresh.id_token_verifier());
                let verified = id_token
                    .claims(&verifier, &nonce)
                    .map(|c| c.subject().to_string())
                    .map_err(|e| {
                        OidcServiceError::Rejected(format!("id_token verify failed: {e}"))
                    });
                verified?
            }
        };

        let payload = decode_jwt_payload(&id_token.to_string())?;
        let claims = FederatedClaims::from_payload(
            &self.config.issuer_url,
            &subject,
            &payload,
            &self.policy.role_claim,
        );
        Ok(OidcIdentity { claims })
    }
}

/// Decode the (already verified) JWT payload segment.
fn decode_jwt_payload(jwt: &str) -> Result<serde_json::Value, OidcServiceError> {
    let segment = jwt
        .split('.')
        .nth(1)
        .ok_or_else(|| OidcServiceError::Provider("malformed id_token".into()))?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(segment.trim_end_matches('='))
        .map_err(|e| OidcServiceError::Provider(format!("id_token payload: {e}")))?;
    serde_json::from_slice(&bytes)
        .map_err(|e| OidcServiceError::Provider(format!("id_token payload json: {e}")))
}

pub type SharedOidcFlowService = Arc<OidcFlowService>;

#[cfg(test)]
mod tests {
    use super::*;

    fn svc(kind: ProviderKind) -> OidcFlowService {
        OidcFlowService::with_policy(OidcConfig::default(), FederationPolicy::for_kind(kind))
    }

    #[test]
    fn generic_scopes_are_openid_email_only() {
        assert_eq!(
            svc(ProviderKind::Generic).requested_scopes(None),
            ["openid", "email"]
        );
    }

    #[test]
    fn keycloak_requests_organization_scope() {
        let s = svc(ProviderKind::Keycloak);
        assert!(s
            .requested_scopes(None)
            .contains(&"organization:*".to_string()));
        assert!(s
            .requested_scopes(Some("acme"))
            .contains(&"organization:acme".to_string()));
    }

    #[test]
    fn payload_decoding_roundtrip() {
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(br#"{"sub":"x","email":"a@b.c"}"#);
        let jwt = format!("h.{payload}.s");
        assert_eq!(decode_jwt_payload(&jwt).unwrap()["sub"], "x");
        assert!(decode_jwt_payload("nodots").is_err());
    }
}
