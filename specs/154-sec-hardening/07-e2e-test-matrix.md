# 07 — E2E test matrix

Parent: [README](README.md) · Edge cases: [05-edge-cases](05-edge-cases.md) · Plan: [06-implementation-plan](06-implementation-plan.md)

LAW-154-11: CI is proof. Unwired tests are documentation, not gates.

## Existing suites (must stay green)

| Suite | Covers |
|-------|--------|
| `edgequake-api/tests/spec028_mcp_oauth_e2e.rs` | PRM, WWW-Authenticate, PKCE, refresh reuse/revoke, CIMD/DCR |
| `edgequake-auth` lib tests | JWT exp, iss/aud when configured, unknown role |
| `startup_security` unit tests | Default/short JWT secret |
| SPEC-028 MCP 006 EC-MCP-* | Transport + OAuth edge cases |

## New SPEC-154 gates

| Test binary / module | Wave | ECs | Assert |
|----------------------|------|-----|--------|
| `e2e_spec154_audience_parity` | 1 | 01, 02 | MCP JWT → REST 401; session JWT → MCP 401 + WWW-Authenticate |
| `e2e_spec154_scope_predicate` | 1 | 03, 04, 20 | Empty scope 403 on tools/call; `scopes_cover` empty = false; `*` only if minted |
| `e2e_spec154_membership_bind_mcp` | 2 | 07, 13, 30 | Strict bind non-member MCP 403; header≠claim 403; foreign workspace forbidden |
| `e2e_spec154_api_key_scopes` | 3 | 05, 06 | Read-only key write tool 403; master write + audit |
| `e2e_spec154_web_refresh_rotation` | 4 | 08, 24 | Rotate issues new refresh; reuse → 401 family revoke; race loser fails |
| `e2e_spec154_jti_durable` | 4 | 09 | After logout, second AppState/process rejects access JWT |
| `e2e_spec154_ws_no_query_token` | 5 | 10, 11 | `?token=` → 401; Bearer header → 101 |
| `e2e_spec154_startup_auth_off_fatal` | 6 | 12, 22 | Non-local URL + auth off + !dev → Fatal; DEV_MODE local OK |
| WebUI Playwright `auth-storage.spec.ts` | 5 | 11 (UX) | No access/refresh in localStorage after login; cookie refresh works |

Suggested location: `edgequake/crates/edgequake-api/tests/e2e_spec154_*.rs` mirroring
SPEC-098 naming.

## Lib / contract companions

| Test | Wave | Purpose |
|------|------|---------|
| `oauth::scopes` / `types` unit | 1 | Empty grant never covers |
| `JwtService` Validation algorithms | 1 | Explicit HS256; reject none |
| `validate_startup_security` unit | 6 | Auth-off fatal |
| Refresh family SQL contract | 4 | Hash lookup + status transitions |

## Per-wave fail-first order

```text
  1. Write failing e2e for the wave's EC rows
  2. Implement minimal CredentialDecision / bind / scopes / refresh / cookie / startup change
  3. Re-run wave tests + full spec028_mcp_oauth_e2e
  4. Update 08-cross-ref status column
```

## Commands (target)

```bash
cargo test -p edgequake-api --test spec028_mcp_oauth_e2e
cargo test -p edgequake-api --test e2e_spec154_audience_parity
cargo test -p edgequake-api --test e2e_spec154_scope_predicate
cargo test -p edgequake-api --test e2e_spec154_membership_bind_mcp
cargo test -p edgequake-api --test e2e_spec154_api_key_scopes
cargo test -p edgequake-api --test e2e_spec154_web_refresh_rotation
cargo test -p edgequake-api --test e2e_spec154_jti_durable
cargo test -p edgequake-api --test e2e_spec154_ws_no_query_token
cargo test -p edgequake-api --lib startup_security
cargo test -p edgequake-auth --lib
cd edgequake_webui && pnpm exec playwright test e2e/auth-storage.spec.ts
```

## Coverage checklist vs findings

| Finding | Gate present |
|---------|--------------|
| F-154-01 | audience_parity |
| F-154-02 | scope_predicate |
| F-154-03 | api_key_scopes |
| F-154-04 | membership_bind_mcp |
| F-154-05 | web_refresh_rotation + jti_durable |
| F-154-06 | ws_no_query_token + Playwright |
| F-154-07 | startup_auth_off_fatal |
