# 04 — Findings register (F-154-*)

Parent: [README](README.md) · Crosswalk: [03-standards-crosswalk](03-standards-crosswalk.md) · Next: [05-edge-cases](05-edge-cases.md)

Severity: **P0** exploit path across surfaces · **P1** privilege / revocation gap · **P2** hygiene / ops footgun

---

## F-154-01 — Confused deputy: MCP resource JWT accepted on REST

| Field | Value |
|-------|-------|
| Severity | **P0** |
| Law | LAW-154-1, LAW-154-3 |
| Wave | 1 |
| EC | EC-154-01, EC-154-02 |

**Observation.** MCP access tokens are minted with `aud = resource URL` and
`scope = edgequake:…` in `oauth/token.rs::issue_tokens`. Web session tokens from
`identity_storage::access_token_claims` set tenant/workspace and omit `aud` /
`scope`. Both are signed by the same `JwtService`.

On MCP, `gateway_auth.rs` requires:

```text
claims.aud.iter().any(|a| a == &cfg.resource_url)
```

On REST, `validate_presented_token` → `jwt.verify_token`. When
`AuthConfig.jwt_audience` is `None`, `JwtService::new` sets
`validation.validate_aud = false` (`jwt.rs` ~192–203). An attacker with a
stolen MCP JWT (read-only, 15m) can call `/api/v1/*` as a full principal using
the `role` claim.

**Mitigation.** Introduce token profiles in one verifier: REST rejects any JWT
whose `aud` contains the MCP resource URL; MCP continues to require it. Optionally
mint web tokens with an explicit `aud=api` later (same wave or follow-up).

**Proof.** `e2e_spec154_audience_parity` — MCP JWT → REST 401; session JWT → MCP 401.

---

## F-154-02 — Two scope predicates (DRY footgun)

| Field | Value |
|-------|-------|
| Severity | **P1** |
| Law | LAW-154-4 |
| Wave | 1 |
| EC | EC-154-03, EC-154-04 |

**Observation.**

| Predicate | Empty grant behavior | Used by |
|-----------|----------------------|---------|
| `McpAuthScopes::allows` (`oauth/types.rs`) | Deny for non-API-key | `mcp/gateway/mod.rs` |
| `scopes_cover` (`oauth/scopes.rs`) | **Allow-all** if `granted.is_empty()` | Re-exported; risk if reused |

Gateway dispatch correctly uses `allows`. The duplicate empty=allow semantics
will silently reopen privilege if a future caller switches helpers.

**Mitigation.** Make `scopes_cover` call `allows` (or delete it). Unit contract:
empty OAuth grant never covers any required scope.

**Proof.** `e2e_spec154_scope_predicate` + lib contract test.

---

## F-154-03 — API keys skip MCP least privilege

| Field | Value |
|-------|-------|
| Severity | **P0** |
| Law | LAW-154-5 |
| Wave | 3 |
| EC | EC-154-05, EC-154-06 |

**Observation.** `McpAuthScopes::api_key_full()` sets `is_api_key: true` and
empty `scopes`. `allows` returns `true` for every required tool scope, including
`edgequake:write`. PRM `MCP_RESOURCE_SCOPES` advertises only read+query
(`mcp/config.rs`). Env/master keys map to `Role::Admin` in
`validate_master_or_stored_api_key`. Membership bind skips
`user_id == "master-api-key"`.

**Mitigation.** Persist OAuth-compatible scopes on stored keys. Default mint
`edgequake:read edgequake:query`. Write tools require `edgequake:write`. Master
key remains break-glass: full scopes + **audit event** on every MCP call (Wave 2
pairs with bind exception). PRM advertises `write` only when write tools are
enabled for the deployment.

**Proof.** `e2e_spec154_api_key_scopes`.

---

## F-154-04 — Membership bind is REST-only

| Field | Value |
|-------|-------|
| Severity | **P1** |
| Law | LAW-154-6 |
| Wave | 2 |
| EC | EC-154-07 |

**Observation.** `protected_api_auth` calls `enforce_membership_bind` when
`state.security.strict_tenant_bind`. `mcp_gateway_auth` calls
`apply_authenticated_context` and returns. `ws_validate_token` builds
`WsSession` without membership verification.

A JWT with spoofable or stale `workspace_id` claim can reach MCP tools under
strict bind deployments that operators believe are protected.

**Mitigation.** Extract bind into a shared helper already used by REST; call
from MCP and WS after context attach. Master key: explicit audited exception.

**Proof.** `e2e_spec154_membership_bind_mcp`.

---

## F-154-05 — Web refresh ≠ OAuth refresh; jti not durable

| Field | Value |
|-------|-------|
| Severity | **P1** |
| Law | LAW-154-7, LAW-154-8 |
| Wave | 4 |
| EC | EC-154-08, EC-154-09 |

**Observation.**

| Aspect | MCP (`oauth/token.rs` + `store.rs`) | Web (`handlers/auth/session.rs`) |
|--------|-------------------------------------|----------------------------------|
| Access TTL | 900s | Default 24h (`JWT_EXPIRY_HOURS`) |
| Refresh rotate | New `eqr_*` each exchange | Same UUID reused |
| Reuse detection | Family revoke | None |
| Hash at rest | SHA-256 | Lookup hash (OK) |
| Logout jti | Process `HashSet` | Same shared `JwtService` denylist |

Logout on replica A does not revoke access tokens on replica B until `exp`.

**Mitigation.** Route web refresh through the rotation algorithm (or shared
module). Shorten web access TTL toward 15m. Persist `jti` denylist in Postgres
with TTL.

**Proof.** `e2e_spec154_web_refresh_rotation`, `e2e_spec154_jti_durable`.

---

## F-154-06 — Browser and WebSocket token exposure

| Field    | Value                |
| ----------| ----------------------|
| Severity | **P1**               |
| Law      | LAW-154-9            |
| Wave     | 5                    |
| EC       | EC-154-10, EC-154-11 |

**Observation.**

1. `use-auth-store.ts` persists `accessToken` and `refreshToken` via Zustand
   `persist` into `localStorage`. Comment claims BR0505 “stored securely”;
   OWASP session guidance: XSS can read JS-accessible storage.
2. `handlers/websocket.rs::authorize_ws_upgrade` accepts
   `query.token.or(header_token)`. Query tokens appear in access logs, Referer,
   and browser history.

MCP itself uses header-only Bearer (compliant).

**Mitigation.** Refresh → HttpOnly Secure `SameSite` cookie on API-owned path;
access token memory-only (or short-lived). WS upgrade: Authorization header or
short-lived ticket exchange; **reject** `?token=`.

**Proof.** `e2e_spec154_ws_no_query_token` + WebUI Playwright Wave 5.

---

## F-154-07 — Auth-off is a warning, not a stop

| Field | Value |
|-------|-------|
| Severity | **P2** (P0 if cloud misconfig) |
| Law | LAW-154-10 |
| Wave | 6 |
| EC | EC-154-12 |

**Observation.** `startup_security.rs`: non-local `DATABASE_URL` +
`!auth_enabled` + `!dev_mode` → **Warn**, fatal only if
`EDGEQUAKE_STRICT_STARTUP`. Default JWT secret is already Fatal without
`DEV_MODE`. With auth off, `mcp_gateway_auth` injects `api_key_full()`.
`allow_anonymous` defaults true.

**Mitigation.** Promote auth-off + non-local DB to Fatal (mirror CORS / JWT
secret pattern). Document `DEV_MODE` as sole local bypass.

**Proof.** `e2e_spec154_startup_auth_off_fatal` (unit on
`validate_startup_security` is sufficient).

---

## Retained (not findings)

| Control | Location |
|---------|----------|
| Argon2id + strength | `password.rs` |
| Login lockout | `login_lockout.rs` |
| MCP PRM + WWW-Authenticate | `mcp/auth/*` |
| PKCE S256 + loopback redirects | `authorize.rs`, `cimd.rs` |
| MCP refresh rotation | `oauth/token.rs`, `oauth/store.rs` |
| Startup default JWT secret fatal | `startup_security.rs` |
| Unknown role fail-closed | `Claims::role` / SPEC-083 S-08 |
| Rate limit on `/mcp` | `routes.rs` |

---

## Finding → wave summary

```text
  F-154-01 --+
  F-154-02 --+--> Wave 1  verifier + aud + scope DRY
             |
  F-154-04 -----> Wave 2  membership bind MCP/WS
             |
  F-154-03 -----> Wave 3  API key scopes
             |
  F-154-05 -----> Wave 4  refresh + durable jti
             |
  F-154-06 -----> Wave 5  cookie + WS ticket
             |
  F-154-07 -----> Wave 6  auth-off fatal
```
