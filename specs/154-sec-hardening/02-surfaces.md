# 02 — Surfaces (code map)

Parent: [README](README.md) · Laws: [01-first-principles](01-first-principles.md) · Next: [03-standards-crosswalk](03-standards-crosswalk.md)

## Surface inventory

```text
  +------------------------------------------------------------------+
  |                         Clients                                    |
  |  WebUI SPA   Claude/Codex/Cursor   Grok bearer   WS browser      |
  +------+-------------+--------------------+------------+-----------+
         |             |                    |            |
         v             v                    v            v
  /api/v1/auth/*   /oauth/*            X-API-Key     /ws/*
  /api/v1/*        /mcp                Bearer eq_*
         |             |                    |            |
         +------+------+----------+---------+------------+
                |                 |
                v                 v
     protected_api_auth    mcp_gateway_auth
                |                 |
                +--------+--------+
                         |
                         v
              auth_validation + JwtService
                         |
                         v
              Postgres identity / sessions / oauth_*
```

## File → responsibility

| Path | Responsibility | Auth concern |
|------|----------------|--------------|
| `edgequake/crates/edgequake-auth/src/jwt.rs` | Sign/verify HS256 JWT; process-local `jti` denylist | F-154-01, F-154-05 |
| `edgequake/crates/edgequake-auth/src/password.rs` | Argon2id hash/verify; strength | Retained |
| `edgequake/crates/edgequake-auth/src/config.rs` | `auth_enabled`, `dev_mode`, `allow_anonymous`, TTLs | F-154-07 |
| `edgequake/crates/edgequake-auth/src/rbac.rs` | Role → Permission | Retained |
| `edgequake-api/src/services/auth_validation.rs` | Master key, stored `eq_` key, JWT → `AuthenticatedRequest` | Wave 1 SSOT base |
| `edgequake-api/src/services/identity_storage.rs` | `access_token_claims` (tenant/workspace, no aud/scope) | F-154-01 |
| `edgequake-api/src/services/login_lockout.rs` | Failed attempts → lock | Retained |
| `edgequake-api/src/middleware.rs` | `protected_api_auth`, membership bind, rate limit, WS validate | F-154-04, F-154-06 |
| `edgequake-api/src/mcp/auth/gateway_auth.rs` | MCP Bearer; aud gate; API key full | F-154-01…03 |
| `edgequake-api/src/mcp/auth/protected_resource.rs` | RFC 9728 PRM | Retained |
| `edgequake-api/src/mcp/auth/www_authenticate.rs` | 401/403 challenges | Retained |
| `edgequake-api/src/mcp/config.rs` | `MCP_RESOURCE_SCOPES` = read, query | F-154-03 |
| `edgequake-api/src/mcp/gateway/mod.rs` | Scope enforce via `allows` | F-154-02 |
| `edgequake-api/src/oauth/types.rs` | `McpAuthScopes::{allows,api_key_full}` | F-154-02, F-154-03 |
| `edgequake-api/src/oauth/scopes.rs` | `scopes_cover` (empty = allow) + tool map | F-154-02 |
| `edgequake-api/src/oauth/token.rs` | Issue MCP ATTs; ACCESS_TTL=900; rotate refresh | Retained (web must match) |
| `edgequake-api/src/oauth/store.rs` | Hash refresh; family revoke | Retained |
| `edgequake-api/src/oauth/authorize.rs` | PKCE S256 only | Retained |
| `edgequake-api/src/oauth/cimd.rs` | CIMD + loopback redirect rules | Retained |
| `edgequake-api/src/handlers/auth/session.rs` | Login / refresh / logout | F-154-05 |
| `edgequake-api/src/handlers/websocket.rs` | `?token=` + header | F-154-06 |
| `edgequake-api/src/startup_security.rs` | Fatal default secret; warn auth-off | F-154-07 |
| `edgequake-api/src/routes.rs` | Mount MCP layers: rate limit + gateway auth | Retained topology |
| `edgequake_webui/src/stores/use-auth-store.ts` | Persist tokens in localStorage | F-154-06 |

## Credential paths today

### REST (`protected_api_auth`)

```text
  extract Bearer | X-API-Key
       |
       v
  validate_presented_token
       |-- master / env API keys --> Admin, no tenant claim
       |-- stored eq_*           --> User|Admin from scopes contains "admin"
       |-- JWT verify_token      --> Claims (aud optional)
       v
  apply_authenticated_context (claim vs header merge)
       |
       v
  [postgres] enforce_membership_bind if strict_tenant_bind
       |
       v
  next
```

### MCP (`mcp_gateway_auth`)

```text
  if !auth_enabled --> insert api_key_full(); next
       |
  extract token
       |
       +-- master/stored key --> apply_authenticated_context; api_key_full(); next
       |                        (NO membership bind)
       |
       +-- JWT verify_token
       |     aud must contain McpPublicConfig.resource_url
       |     else 401 + WWW-Authenticate
       |     scopes from scope claim
       |     apply_authenticated_context; insert scopes; next
       |
       +-- else 401 + WWW-Authenticate
```

### WebSocket (`authorize_ws_upgrade`)

```text
  origin allow-list
       |
  token = query.token OR Authorization / X-API-Key
       |
  ws_validate_token --> WsSession (default tenant/workspace for keys)
       |
  (NO membership bind, NO scope profile)
```

## Token mint paths

| Issuer | Function | Claims | TTL |
|--------|----------|--------|-----|
| Web login | `access_token_claims` + `generate_token_with_claims` | sub, role, tenant, workspace; **no aud/scope** | `jwt_expiry` default **24h** |
| MCP token endpoint | `issue_tokens` in `oauth/token.rs` | aud=resource, scope, iss, tenant, workspace | **900s** access; refresh **30d** rotating |
| API key create | Persisted Argon2 hash + scopes list | Opaque `eq_…` | Optional `expires_at` |

## Public paths (REST)

From `is_public_request` in `middleware.rs`:

- `/health`, `/ready`, `/live`
- `/auth/login`, `/auth/refresh`, `/auth/oidc/*`
- `/setup/status`, `/setup/initialize`
- Documentation (`/swagger-ui`, `/api-docs`)
- `POST /users` when registration allowed
- Note: `POST /mcp` is listed in the public-path helper for the **REST** middleware
  tree; the MCP router applies `mcp_gateway_auth` separately. Do not treat this
  dual mount as “MCP is public.”

## Desired end state (Waves 1–6)

```text
                 +------------------------+
                 |  CredentialDecision    |
                 |  profile + principal   |
                 |  + scopes + membership |
                 +-----------+------------+
                             |
            +----------------+----------------+
            |                |                |
            v                v                v
     REST profile      MCP profile      WS profile
     reject mcp aud    require mcp aud   header/ticket
     role RBAC         tool scopes       bind membership
```

Cross-ref: [04-findings](04-findings.md) · [06-implementation-plan](06-implementation-plan.md).
