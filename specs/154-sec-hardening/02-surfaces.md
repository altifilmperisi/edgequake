# 02 — Surfaces (code map)

Parent: [README](README.md) · Laws: [01-first-principles](01-first-principles.md) · Next: [03-standards-crosswalk](03-standards-crosswalk.md)

> **Post-Wave 6 (product ≥ 0.28.4):** This map describes **current** code, not
> the pre-fix inventory. Residuals are labeled below.

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
              auth_validation.decide + JwtService
                         |
                         v
              Postgres identity / sessions / oauth_* / jwt_jti_denylist
```

## File → responsibility

| Path | Responsibility | Auth concern |
|------|----------------|--------------|
| `edgequake/crates/edgequake-auth/src/jwt.rs` | Sign/verify HS256 JWT; process-local `jti` cache | F-154-01, F-154-05 |
| `edgequake/crates/edgequake-auth/src/password.rs` | Argon2id hash/verify; strength | Retained |
| `edgequake/crates/edgequake-auth/src/config.rs` | `auth_enabled`, `dev_mode`, `allow_anonymous`, TTLs (default access **900s**) | F-154-07 |
| `edgequake/crates/edgequake-auth/src/rbac.rs` | Role → Permission | Retained |
| `edgequake-api/src/services/auth_validation.rs` | `decide` + profiles; master/env/stored key; durable jti on extractors | Wave 1 SSOT |
| `edgequake-api/src/services/identity_storage.rs` | Token claims mint; PG identity SSOT | F-154-01 |
| `edgequake-api/src/services/session_storage.rs` | `take_web_refresh` prefers PG family rotate | F-154-05 |
| `edgequake-api/src/services/login_lockout.rs` | Failed attempts → lock | Retained |
| `edgequake-api/src/middleware.rs` | REST auth; membership bind; WS protocol JWT; master bypass audit | F-154-04, F-154-06 |
| `edgequake-api/src/mcp/auth/gateway_auth.rs` | MCP Bearer; aud gate; scoped keys | F-154-01…03 |
| `edgequake-api/src/mcp/auth/protected_resource.rs` | RFC 9728 PRM (write when Memory profile) | Retained / Wave 3 |
| `edgequake-api/src/mcp/auth/www_authenticate.rs` | 401/403 challenges | Retained |
| `edgequake-api/src/oauth/types.rs` | `McpAuthScopes::{allows,api_key_full,from_api_key_scopes}` | F-154-02, F-154-03 |
| `edgequake-api/src/oauth/scopes.rs` | `scopes_cover` (**empty = deny**) + tool map | F-154-02 |
| `edgequake-api/src/oauth/token.rs` | Issue MCP ATTs; ACCESS_TTL=900; rotate refresh | Retained (web matched) |
| `edgequake-api/src/oauth/store.rs` | Hash refresh; family revoke | Retained |
| `edgequake-api/src/handlers/auth/session.rs` | Login / refresh / logout; SPA omit refresh JSON; HttpOnly `eq_refresh` | F-154-05 |
| `edgequake-api/src/handlers/websocket.rs` | Reject `?token=`; accept Authorization / `Sec-WebSocket-Protocol` | F-154-06 |
| `edgequake-api/src/startup_security.rs` | Fatal default secret; **Fatal** auth-off on non-local DB | F-154-07 |
| `edgequake_webui/src/stores/use-auth-store.ts` | Access token **in memory**; no localStorage secrets | F-154-06 |
| `edgequake_webui/src/lib/websocket/ws-auth.ts` | `withAuthToken` no-op on URL; protocol `edgequake.bearer` | F-154-06 |

## Credential paths today

### REST (`protected_api_auth`)

```text
  extract Bearer | X-API-Key
       |
       v
  decide(profile=web_session|api_key)
       |-- master_api_key     --> break_glass Admin + compliance audit on bind skip
       |-- EDGEQUAKE_API_KEYS --> read+query (not break-glass)
       |-- stored eq_*        --> from_api_key_scopes
       |-- JWT                --> aud profile gate; durable jti
       v
  apply_authenticated_context
       |
       v
  [postgres] enforce_membership_bind if strict_tenant_bind
       |
       v
  next
```

### MCP (`mcp_gateway_auth`)

```text
  if !auth_enabled --> Fatal at startup when non-local DB (!dev_mode)
       |
  extract token
       |
       +-- master/stored/env key --> scoped allows(); membership bind under strict
       |
       +-- JWT decide(mcp_resource)
       |     aud must contain McpPublicConfig.resource_url
       |     scopes_cover empty = deny
       |     membership bind under strict
       |
       +-- else 401 + WWW-Authenticate
```

### WebSocket (`authorize_ws_upgrade`)

```text
  origin allow-list
       |
       +-- ?token= --> 401 (hard reject)
       |
  token = Authorization | X-API-Key | Sec-WebSocket-Protocol edgequake.bearer
       |
  decide + membership bind under strict
```

## Token mint paths

| Issuer | Function | Claims | TTL |
|--------|----------|--------|-----|
| Web login | `access_token_claims` + mint | sub, role, tenant, workspace; SPA omits refresh JSON | Access **900s**; HttpOnly `eq_refresh` |
| MCP token endpoint | `issue_tokens` | aud=resource, scope, iss, tenant, workspace | **900s** access; refresh **30d** rotating |
| API key create | Argon2 hash + scopes | Opaque `eq_…` | Optional `expires_at` |

## Residual surface

| Item | Status |
|------|--------|
| Non-HttpOnly `edgequake_access_token` cookie (Next middleware) | **Residual** — XSS-readable; Secure on HTTPS; HttpOnly session later |
| Playwright `auth-storage.spec.ts` | Manual / soft-skip without `E2E_AUTH_*`; **CI gate = vitest** |

## Public paths (REST)

From `is_public_request` in `middleware.rs`:

- `/health`, `/ready`, `/live`
- `/auth/login`, `/auth/refresh`, `/auth/oidc/*`
- `/setup/status`, `/setup/initialize`
- Documentation (`/swagger-ui`, `/api-docs`)
- `POST /users` when registration allowed
- Note: `POST /mcp` may appear in the REST public-path helper; the MCP router
  applies `mcp_gateway_auth` separately. Do not treat MCP as public.

Cross-ref: [04-findings](04-findings.md) · [06-implementation-plan](06-implementation-plan.md) · [08-cross-ref](08-cross-ref.md).
