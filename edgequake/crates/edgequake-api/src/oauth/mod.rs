//! MCP OAuth 2.1 authorization server (SPEC-028 / MCP Authorization 2026-07-28).
//!
//! EdgeQuake acts as a thin AS that reuses the existing user store and JWT signer.
//! The MCP gateway remains the OAuth resource server (RFC 9728).

pub mod authorize;
pub mod cimd;
pub mod metadata;
pub mod pkce;
pub mod register;
pub mod revoke;
pub mod scopes;
pub mod store;
pub mod token;
pub mod types;

pub use scopes::{required_scope_for_tool, scopes_cover, MCP_SCOPE_QUERY, MCP_SCOPE_READ};
pub use types::McpAuthScopes;
