# Lens — Full Stack Developer

Parent: [README](../README.md) · Surfaces: [02-surfaces](../02-surfaces.md) · Plan: [06](../06-implementation-plan.md)

## Design target

```text
  handlers / middleware / mcp gateway
              |
              v
  +---------------------------+
  | CredentialDecision SSOT   |  <-- extend auth_validation.rs
  +---------------------------+
     | profile | scopes | principal
     v
  surface policy (thin)
```

## Module ownership

| Concern | Own in | Do not |
|---------|--------|--------|
| Decode + classify credential | `auth_validation` (+ auth crate) | Re-parse JWT in gateway only |
| MCP WWW-Authenticate / PRM | `mcp/auth/*` | Duplicate challenge strings in REST |
| Refresh family persistence | shared store helper | Copy SQL in session.rs and oauth/store.rs |
| Route public paths | `middleware::is_public_request` | Special-case MCP as “public” without gateway layer |

## Wave 1 sketch (DRY)

```rust
// Conceptual — not shipping in Wave 0
pub enum TokenProfile { WebSession, McpResource, ApiKey }

pub struct CredentialDecision {
    pub profile: TokenProfile,
    pub auth: RequestAuthContext,
    pub scopes: McpAuthScopes,
    pub jwt_tenant_id: Option<String>,
    pub jwt_workspace_id: Option<String>,
}

pub async fn decide(state: &AppState, token: &str, want: TokenProfile)
    -> Result<CredentialDecision, ApiError>;
```

REST calls `decide(..., WebSession)` and rejects `McpResource` profile.
MCP calls `decide(..., McpResource)`.

## SOLID map

| Principle | Practice |
|-----------|----------|
| SRP | Gateway does not mint tokens; AS does not enforce tool scopes |
| OCP | New surface adds an apply function, not a new JWT decoder |
| DIP | Tests inject AppState; avoid hard-wiring PG in unit tests where ports exist |

## Anti-patterns to avoid

- Second `verify_token` with different Validation flags per route
- `scopes_cover` empty-allow creeping back
- Logging raw Bearer or refresh tokens
- Feature-flagging aud checks behind “compat mode” without expiry date

## Dev workflow

1. Fail-first e2e from [07](../07-e2e-test-matrix.md)
2. Minimal change in SSOT module
3. `cargo test -p edgequake-api --test spec028_mcp_oauth_e2e`
4. Clippy + fmt on touched crates
