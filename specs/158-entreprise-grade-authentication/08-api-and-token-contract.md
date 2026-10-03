# 08 — API and token contract

Parent: [README](README.md) · Architecture: [04](04-architecture.md) ·
DB: [07](07-data-and-db-contract.md)

## Public auth routes (no Bearer required)

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/v1/auth/sso/providers` | List enabled providers + display names (no secrets) |
| GET | `/api/v1/auth/sso/start` | Begin SSO (`provider`, optional `org`, `redirect`) → 302 |
| GET | `/api/v1/auth/oidc/login` | **Compat** alias → default/keycloak provider |
| GET | `/api/v1/auth/oidc/callback` | Code + state → set cookie + 302 handoff |
| POST | `/api/v1/auth/handoff` | Exchange one-time code → `LoginResponse` (SPA) |
| POST | `/api/v1/auth/oidc/backchannel-logout` | OIDC logout_token form POST |
| POST | `/api/v1/auth/login` | Password (existing) |
| POST | `/api/v1/auth/refresh` | Existing + cookie |
| POST | `/api/v1/auth/logout` | Existing + local revoke |

Middleware: extend `is_public_request` for `sso/*`, `handoff`, `backchannel-logout`.

## Query parameters

### `GET /auth/sso/start`

| Param | Required | Rules |
|-------|----------|-------|
| `provider` | yes* | slug; *default `keycloak` or sole enabled |
| `org` | no | Organization alias → `organization:{alias}` scope |
| `redirect` | no | Same-origin relative path only (mirror WebUI `safeRedirectPath`) |

### `GET /auth/oidc/callback`

| Param | Required |
|-------|----------|
| `code` | yes |
| `state` | yes |
| error / error_description | IdP errors → mapped JSON or error redirect |

**Success redirect:** `EDGEQUAKE_OIDC_SUCCESS_REDIRECT_URL` + `?code={handoff}`
**only** — never access/refresh tokens (LAW-158-4). Also `Set-Cookie: eq_refresh`.

### `POST /auth/handoff`

```json
{ "code": "opaque-one-time" }
```

Response: existing `LoginResponse` shape; SPA clients omit refresh in body
(`omit_refresh_in_json_body`).

## EdgeQuake access token claims (SSO)

```json
{
  "sub": "<user_uuid>",
  "role": "user|admin|readonly",
  "tenant_id": "<resolved_tenant_uuid>",
  "workspace_id": "<default_or_selected_workspace_uuid>",
  "iat": 0, "exp": 0, "nbf": 0,
  "jti": "<uuid>",
  "iss": "<optional JWT_ISSUER>",
  "aud": "<optional — not MCP URL for web_session>",
  "metadata": {
    "auth_provider": "oidc",
    "idp_slug": "keycloak",
    "org_alias": "acme"
  }
}
```

Built via shared `login_tokens::issue_for_user(scope)` — **not**
unconditional `default_identity_scope()` (F-158-03).

## Tenant API AuthZ (LAW-158-6)

| Endpoint | Who |
|----------|-----|
| `POST /tenants` | Platform admin (or bootstrap) |
| `GET /tenants` | Membership-scoped; platform admin sees all |
| `GET /tenants/{id}` | Member of tenant or platform admin |
| `PUT /tenants/{id}` | Tenant owner/admin or platform admin |
| `DELETE /tenants/{id}` | Tenant owner or platform admin |

Use `ApiRequireAdmin` for platform-only ops; membership checks for tenant-scoped.

## Errors (stable codes)

| HTTP | `error` / reason | When |
|------|------------------|------|
| 401 | `state_mismatch` / `state_expired` | Pending missing/TTL |
| 401 | `oidc_token_invalid` | id_token verify fail |
| 403 | `email_unverified` | Link policy blocked |
| 403 | `org_unknown` | Organization not mapped |
| 403 | `tenant_inactive` | Suspended tenant |
| 403 | `max_users` | JIT blocked by plan |
| 409 | `username_conflict` | Exhausted suffix strategy |
| 503 | `oidc_not_configured` | No providers |

## Back-channel logout

`POST /api/v1/auth/oidc/backchannel-logout`  
`Content-Type: application/x-www-form-urlencoded`  
`logout_token=<JWT>`

Validate per [OIDC Back-Channel Logout](https://openid.net/specs/openid-connect-backchannel-1_0.html):

- Signature via issuer JWKS
- `iss`, `aud` (client_id), `events` contains backchannel-logout
- Reject if `nonce` present
- Replay cache on `jti`
- Revoke by `sid` → refresh rows; else all sessions for mapped `sub`

Response: `200` even on unknown sid (avoid user enumeration) with audit log.

## Health extensions

```json
{
  "auth_mechanisms": ["jwt_password", "api_key", "oidc", "sso"],
  "oauth2_oidc_builtin": true,
  "external_sso_pattern": "keycloak",
  "sso_providers": [{ "slug": "keycloak", "kind": "oidc" }]
}
```

Update `EXTERNAL_SSO_PATTERN` docs from `oauth2-proxy` → `keycloak` as primary
(F-158-10).

## OpenAPI

utoipa annotations for new routes; regenerate client via existing
`make codegen-openapi-refresh` when implementing.

## Compatibility

| Old | New |
|-----|-----|
| Tokens in success URL | Handoff code (breaking for anyone parsing tokens from URL — document migration) |
| Single env IdP | Still works via shim |
| `/auth/oidc/login` | Kept as alias |
