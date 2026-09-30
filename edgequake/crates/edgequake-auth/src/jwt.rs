//! JWT (JSON Web Token) service for authentication.
//!
//! ## Implements
//!
//! - **FEAT0850**: JWT token generation with configurable TTL
//! - **FEAT0851**: JWT token validation and claims extraction
//! - **FEAT0852**: Refresh token support
//!
//! ## Use Cases
//!
//! - **UC2501**: System generates JWT on successful login
//! - **UC2502**: System validates JWT on API request
//! - **UC2503**: Client refreshes expired access token
//!
//! ## Enforces
//!
//! - **BR0850**: Tokens must include expiration claim
//! - **BR0851**: Secret key must be at least 32 bytes

use std::collections::HashSet;
use std::sync::{Arc, RwLock};

use chrono::{Duration, Utc};
use jsonwebtoken::{
    dangerous, decode, encode, Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config::AuthConfig;
use crate::error::AuthError;
use crate::types::Role;

/// JWT claims structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user ID).
    pub sub: String,

    /// User's role.
    pub role: String,

    /// Issued at timestamp.
    pub iat: i64,

    /// Expiration timestamp.
    pub exp: i64,

    /// Not before timestamp.
    pub nbf: i64,

    /// JWT ID (unique identifier for this token).
    pub jti: String,

    /// Issuer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iss: Option<String>,

    /// Audience.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aud: Option<Vec<String>>,

    /// OAuth 2.1 space-delimited scopes (MCP resource tokens).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,

    /// Tenant ID (for multi-tenancy).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,

    /// Workspace ID (for multi-tenancy).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,

    /// Additional custom claims.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl Claims {
    /// Create new claims for a user.
    pub fn new(user_id: Uuid, role: Role, expiry_seconds: i64) -> Self {
        let now = Utc::now();
        Self {
            sub: user_id.to_string(),
            role: role.as_str().to_string(),
            iat: now.timestamp(),
            exp: (now + Duration::seconds(expiry_seconds)).timestamp(),
            nbf: now.timestamp(),
            jti: Uuid::new_v4().to_string(),
            iss: None,
            aud: None,
            scope: None,
            tenant_id: None,
            workspace_id: None,
            metadata: None,
        }
    }

    /// Set issuer.
    pub fn with_issuer(mut self, issuer: impl Into<String>) -> Self {
        self.iss = Some(issuer.into());
        self
    }

    /// Set audience.
    pub fn with_audience(mut self, audience: Vec<String>) -> Self {
        self.aud = Some(audience);
        self
    }

    /// Set OAuth scope claim (space-delimited).
    pub fn with_scope(mut self, scope: impl Into<String>) -> Self {
        self.scope = Some(scope.into());
        self
    }

    /// Parsed scope tokens from the `scope` claim.
    pub fn scopes(&self) -> Vec<&str> {
        self.scope
            .as_deref()
            .map(|s| s.split_whitespace().filter(|p| !p.is_empty()).collect())
            .unwrap_or_default()
    }

    /// Set tenant ID.
    pub fn with_tenant_id(mut self, tenant_id: impl Into<String>) -> Self {
        self.tenant_id = Some(tenant_id.into());
        self
    }

    /// Set workspace ID.
    pub fn with_workspace_id(mut self, workspace_id: impl Into<String>) -> Self {
        self.workspace_id = Some(workspace_id.into());
        self
    }

    /// Set metadata.
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Get user ID from claims.
    pub fn user_id(&self) -> Result<Uuid, AuthError> {
        Uuid::parse_str(&self.sub).map_err(|_| AuthError::InvalidToken {
            reason: "Invalid user ID in token".to_string(),
        })
    }

    /// Get role from claims (fail-closed — SPEC-083 S-08).
    pub fn role(&self) -> Result<Role, AuthError> {
        Role::try_parse(&self.role).map_err(|reason| AuthError::InvalidToken { reason })
    }

    /// Check if token is expired.
    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp() > self.exp
    }

    /// Get time until expiration in seconds.
    pub fn expires_in(&self) -> i64 {
        self.exp - Utc::now().timestamp()
    }
}

/// JWT service for generating and validating tokens.
#[derive(Clone)]
pub struct JwtService {
    config: Arc<AuthConfig>,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    validation: Validation,
    /// SPEC-083 S-07: in-process jti denylist (logout / explicit revoke).
    revoked_jti: Arc<RwLock<HashSet<String>>>,
}

impl JwtService {
    /// Create a new JWT service with the given configuration.
    pub fn new(config: AuthConfig) -> Self {
        let encoding_key = EncodingKey::from_secret(config.jwt_secret.as_bytes());
        let decoding_key = DecodingKey::from_secret(config.jwt_secret.as_bytes());

        // SPEC-154 LAW-154-2: pin HS256 explicitly (reject alg:none / confusion).
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;
        validation.validate_nbf = true;
        // WHY: 30-second leeway accommodates clock skew between distributed servers.
        // Mobile clients and edge nodes often have slightly out-of-sync clocks.
        // 30 seconds is the industry standard balance between security and usability.
        validation.leeway = 30;

        // SPEC-083 S-07: validate iss/aud only when configured (fail-closed when set).
        // When audience is unset, disable aud checks so MCP resource-bound tokens
        // (aud=/mcp) and web session tokens share the same signer; MCP gateway enforces aud.
        if let Some(ref issuer) = config.jwt_issuer {
            validation.set_issuer(&[issuer.as_str()]);
        }
        if let Some(ref audience) = config.jwt_audience {
            let aud: Vec<&str> = audience.iter().map(String::as_str).collect();
            validation.set_audience(&aud);
        } else {
            validation.validate_aud = false;
        }

        Self {
            config: Arc::new(config),
            encoding_key,
            decoding_key,
            validation,
            revoked_jti: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Revoke an access-token jti (SPEC-083 S-07 logout denylist).
    pub fn revoke_jti(&self, jti: impl AsRef<str>) {
        if let Ok(mut set) = self.revoked_jti.write() {
            set.insert(jti.as_ref().to_string());
        }
    }

    /// True when jti was previously revoked.
    pub fn is_jti_revoked(&self, jti: &str) -> bool {
        self.revoked_jti
            .read()
            .map(|set| set.contains(jti))
            .unwrap_or(false)
    }

    /// Generate a new access token for a user.
    pub fn generate_token(&self, user_id: Uuid, role: Role) -> Result<String, AuthError> {
        let expiry_seconds = self.config.jwt_expiry.as_secs() as i64;
        let mut claims = Claims::new(user_id, role, expiry_seconds);
        if let Some(ref issuer) = self.config.jwt_issuer {
            claims = claims.with_issuer(issuer.clone());
        }
        if let Some(ref audience) = self.config.jwt_audience {
            claims = claims.with_audience(audience.clone());
        }
        self.encode_claims(&claims)
    }

    /// Generate a token with custom claims.
    pub fn generate_token_with_claims(&self, claims: Claims) -> Result<String, AuthError> {
        self.encode_claims(&claims)
    }

    /// Encode claims into a JWT.
    fn encode_claims(&self, claims: &Claims) -> Result<String, AuthError> {
        encode(&Header::default(), claims, &self.encoding_key).map_err(|e| {
            AuthError::TokenGenerationFailed {
                reason: e.to_string(),
            }
        })
    }

    /// Verify and decode a JWT.
    pub fn verify_token(&self, token: &str) -> Result<Claims, AuthError> {
        let token_data: TokenData<Claims> = decode(token, &self.decoding_key, &self.validation)
            .map_err(|e| {
                use jsonwebtoken::errors::ErrorKind;
                let auth_err = match e.kind() {
                    ErrorKind::ExpiredSignature => AuthError::TokenExpired,
                    ErrorKind::InvalidToken => AuthError::InvalidToken {
                        reason: "Malformed token".to_string(),
                    },
                    ErrorKind::InvalidSignature => AuthError::InvalidToken {
                        reason: "Invalid signature".to_string(),
                    },
                    ErrorKind::ImmatureSignature => AuthError::InvalidToken {
                        reason: "Token not yet valid".to_string(),
                    },
                    ErrorKind::InvalidIssuer => AuthError::InvalidToken {
                        reason: "Invalid issuer".to_string(),
                    },
                    ErrorKind::InvalidAudience => AuthError::InvalidToken {
                        reason: "Invalid audience".to_string(),
                    },
                    _ => AuthError::InvalidToken {
                        reason: e.to_string(),
                    },
                };
                tracing::warn!(
                    error.code = %auth_err.error_code(),
                    error.source = "jwt",
                    error.message = %auth_err,
                    "JWT verification failed"
                );
                auth_err
            })?;

        // SPEC-083 S-08: unknown roles must not become User.
        let _ = token_data.claims.role()?;

        // SPEC-083 S-07: reject logged-out / revoked jti.
        if self.is_jti_revoked(&token_data.claims.jti) {
            return Err(AuthError::InvalidToken {
                reason: "Token revoked (jti denylist)".to_string(),
            });
        }

        Ok(token_data.claims)
    }

    /// Extract claims from token without verification (for debugging).
    /// WARNING: This should only be used for logging/debugging, never for authentication.
    pub fn decode_unverified(&self, token: &str) -> Result<Claims, AuthError> {
        let token_data =
            dangerous::insecure_decode::<Claims>(token).map_err(|e| AuthError::InvalidToken {
                reason: e.to_string(),
            })?;

        Ok(token_data.claims)
    }

    /// Refresh an access token (generate new token from valid claims).
    pub fn refresh_token(&self, claims: &Claims) -> Result<String, AuthError> {
        let user_id = claims.user_id()?;
        let role = claims.role()?;
        let expiry_seconds = self.config.jwt_expiry.as_secs() as i64;

        let mut new_claims = Claims::new(user_id, role, expiry_seconds);
        new_claims.tenant_id = claims.tenant_id.clone();
        new_claims.workspace_id = claims.workspace_id.clone();
        new_claims.metadata = claims.metadata.clone();
        if let Some(ref issuer) = self.config.jwt_issuer {
            new_claims.iss = Some(issuer.clone());
        }
        if let Some(ref audience) = self.config.jwt_audience {
            new_claims.aud = Some(audience.clone());
        }

        self.encode_claims(&new_claims)
    }

    /// Get the configured expiry duration.
    pub fn expiry_duration(&self) -> std::time::Duration {
        self.config.jwt_expiry
    }

    /// Get refresh token expiry duration.
    pub fn refresh_token_expiry(&self) -> std::time::Duration {
        self.config.refresh_token_expiry
    }
}

impl std::fmt::Debug for JwtService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JwtService")
            .field("config", &self.config)
            .field("validation", &self.validation)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> AuthConfig {
        AuthConfig::new("test-secret-key-that-is-long-enough-256-bits")
    }

    #[test]
    fn test_generate_and_verify_token() {
        let service = JwtService::new(test_config());
        let user_id = Uuid::new_v4();
        let role = Role::User;

        let token = service.generate_token(user_id, role.clone()).unwrap();
        let claims = service.verify_token(&token).unwrap();

        assert_eq!(claims.user_id().unwrap(), user_id);
        assert_eq!(claims.role().unwrap(), role);
    }

    #[test]
    fn contract_unknown_role_rejected() {
        let service = JwtService::new(test_config());
        let user_id = Uuid::new_v4();
        let mut claims = Claims::new(user_id, Role::User, 3600);
        claims.role = "superadmin".to_string();
        let token = service.generate_token_with_claims(claims).unwrap();
        assert!(matches!(
            service.verify_token(&token),
            Err(AuthError::InvalidToken { .. })
        ));
    }

    #[test]
    fn contract_jwt_requires_iss_aud() {
        let mut config = test_config();
        config.jwt_issuer = Some("edgequake".to_string());
        config.jwt_audience = Some(vec!["api".to_string()]);
        let service = JwtService::new(config);
        let token = service.generate_token(Uuid::new_v4(), Role::User).unwrap();
        assert!(service.verify_token(&token).is_ok());

        let mut wrong = Claims::new(Uuid::new_v4(), Role::User, 3600)
            .with_issuer("other")
            .with_audience(vec!["api".to_string()]);
        let bad = service.generate_token_with_claims(wrong.clone()).unwrap();
        assert!(service.verify_token(&bad).is_err());
        wrong.iss = Some("edgequake".to_string());
        wrong.aud = Some(vec!["wrong-aud".to_string()]);
        let bad_aud = service.generate_token_with_claims(wrong).unwrap();
        assert!(service.verify_token(&bad_aud).is_err());
    }

    #[test]
    fn test_expired_token() {
        let mut config = test_config();
        config.jwt_expiry = std::time::Duration::from_secs(0);
        let service = JwtService::new(config);
        let user_id = Uuid::new_v4();

        // Create token that expired 60 seconds ago (past the 30-second leeway)
        let expiry_seconds = -60;
        let claims = Claims::new(user_id, Role::User, expiry_seconds);
        let token = service.generate_token_with_claims(claims).unwrap();

        // Verification should fail
        let result = service.verify_token(&token);
        assert!(matches!(result, Err(AuthError::TokenExpired)));
    }

    #[test]
    fn test_invalid_token() {
        let service = JwtService::new(test_config());
        let result = service.verify_token("invalid.token.here");
        assert!(matches!(result, Err(AuthError::InvalidToken { .. })));
    }

    #[test]
    fn test_claims_with_tenant() {
        let service = JwtService::new(test_config());
        let user_id = Uuid::new_v4();
        let tenant_id = "tenant-123";
        let workspace_id = "workspace-456";

        let claims = Claims::new(user_id, Role::Admin, 3600)
            .with_tenant_id(tenant_id)
            .with_workspace_id(workspace_id);

        let token = service.generate_token_with_claims(claims).unwrap();
        let decoded = service.verify_token(&token).unwrap();

        assert_eq!(decoded.tenant_id, Some(tenant_id.to_string()));
        assert_eq!(decoded.workspace_id, Some(workspace_id.to_string()));
    }

    #[test]
    fn test_refresh_token() {
        let service = JwtService::new(test_config());
        let user_id = Uuid::new_v4();

        let original = service.generate_token(user_id, Role::User).unwrap();
        let claims = service.verify_token(&original).unwrap();

        let refreshed = service.refresh_token(&claims).unwrap();
        let new_claims = service.verify_token(&refreshed).unwrap();

        assert_eq!(new_claims.user_id().unwrap(), user_id);
        assert_ne!(new_claims.jti, claims.jti); // New token ID
    }

    #[test]
    fn e2e_logout_rejects_access_jti() {
        // Matrix Cluster 02 / S-07: logout denylist rejects access token jti.
        let service = JwtService::new(test_config());
        let token = service.generate_token(Uuid::new_v4(), Role::User).unwrap();
        let claims = service.verify_token(&token).unwrap();
        service.revoke_jti(&claims.jti);
        assert!(matches!(
            service.verify_token(&token),
            Err(AuthError::InvalidToken { .. })
        ));
    }
}
