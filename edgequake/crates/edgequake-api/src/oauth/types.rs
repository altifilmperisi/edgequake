//! Shared OAuth AS types for MCP.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Scopes granted for the current MCP request (request extension).
#[derive(Debug, Clone)]
pub struct McpAuthScopes {
    /// Granted scopes (OAuth resource scopes).
    pub scopes: Vec<String>,
    /// True when the credential was an API key (skip audience checks).
    pub is_api_key: bool,
    /// Master / break-glass key — unrestricted MCP tools (SPEC-154 LAW-154-5).
    pub break_glass: bool,
}

impl McpAuthScopes {
    /// Master / auth-disabled break-glass (full MCP surface).
    pub fn api_key_full() -> Self {
        Self {
            scopes: Vec::new(),
            is_api_key: true,
            break_glass: true,
        }
    }

    /// Scoped API key (least privilege).
    pub fn from_api_key_scopes(scopes: Vec<String>) -> Self {
        Self {
            scopes,
            is_api_key: true,
            break_glass: false,
        }
    }

    pub fn from_scope_claim(scope: Option<&str>) -> Self {
        let scopes = scope
            .unwrap_or("")
            .split_whitespace()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        Self {
            scopes,
            is_api_key: false,
            break_glass: false,
        }
    }

    pub fn allows(&self, required: &str) -> bool {
        if self.break_glass {
            return true;
        }
        // OAuth JWTs and scoped API keys: empty never allows (LAW-154-4 / LAW-154-5).
        self.scopes.iter().any(|s| s == required || s == "*")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthClientRegistration {
    pub client_id: String,
    #[serde(default)]
    pub client_name: Option<String>,
    pub redirect_uris: Vec<String>,
    #[serde(default)]
    pub grant_types: Vec<String>,
    #[serde(default)]
    pub response_types: Vec<String>,
    #[serde(default)]
    pub token_endpoint_auth_method: Option<String>,
    /// When true, client_id is a CIMD HTTPS URL (not a DCR-issued id).
    #[serde(default)]
    pub is_cimd: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthAuthorizationCode {
    pub code: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub code_challenge: String,
    pub code_challenge_method: String,
    pub resource: String,
    pub scope: String,
    pub user_id: String,
    pub role: String,
    pub tenant_id: Option<String>,
    pub workspace_id: Option<String>,
    pub expires_at: DateTime<Utc>,
}

/// Status of a stored refresh grant (hashed at rest).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OAuthRefreshStatus {
    Active,
    Rotated,
    Revoked,
}

impl OAuthRefreshStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Rotated => "rotated",
            Self::Revoked => "revoked",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "active" => Some(Self::Active),
            "rotated" => Some(Self::Rotated),
            "revoked" => Some(Self::Revoked),
            _ => None,
        }
    }
}

/// Outcome of consuming a refresh token (rotation / reuse detection).
#[derive(Debug, Clone)]
pub enum TakeRefreshOutcome {
    /// Active grant consumed (marked rotated); issue new tokens with same family_id.
    Consumed(Box<OAuthRefreshGrant>),
    /// Token unknown (never issued or already purged).
    Invalid,
    /// Previously rotated/revoked token presented → family revoked.
    ReuseDetected { family_id: Uuid },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthRefreshGrant {
    /// Plaintext token — only present when issuing; never persisted.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub token: String,
    pub family_id: Uuid,
    pub client_id: String,
    pub resource: String,
    pub scope: String,
    pub user_id: String,
    pub role: String,
    pub tenant_id: Option<String>,
    pub workspace_id: Option<String>,
    pub expires_at: DateTime<Utc>,
    #[serde(default = "default_refresh_status")]
    pub status: OAuthRefreshStatus,
}

fn default_refresh_status() -> OAuthRefreshStatus {
    OAuthRefreshStatus::Active
}
