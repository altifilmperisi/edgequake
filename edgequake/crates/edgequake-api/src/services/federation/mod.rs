//! SPEC-158 enterprise federation (SSO) — provider-agnostic building blocks.
//!
//! Layering (dependencies point downward only):
//!
//! ```text
//!   handlers/auth/{oidc,handoff,backchannel,sso}      HTTP surface
//!        │
//!   resolver ── membership_sync ── tenant_resolver     orchestration
//!        │
//!   claims · org_claim · policy · link_policy · role_map · username · redirect   pure rules
//!        │
//!   store (trait) ◄── memory_store | pg_store          persistence port + adapters
//! ```

pub mod access_jti;
pub mod backchannel;
pub mod claims;
pub mod handoff;
pub mod link_policy;
pub mod membership_sync;
pub mod memory_store;
pub mod org_claim;
pub mod policy;
pub mod redirect;
pub mod registry;
pub mod resolver;
pub mod role_map;
pub mod scope_guard;
pub mod store;
pub mod tenant_resolver;
pub mod username;

#[cfg(feature = "postgres")]
pub mod pg_store;

pub use memory_store::MemoryFederationStore;
pub use store::{FederationStore, SharedFederationStore};
