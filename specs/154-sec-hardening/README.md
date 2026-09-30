# SPEC-154 — Authentication & MCP Security Hardening

> **Status:** Waves 0–6 implemented (code + e2e gates); honesty pass in v0.28.5  
> **Product pin:** EdgeQuake v0.28.5  

> **Scope:** Verify REST and MCP authentication against MCP Authorization 2026-07-28,
> OAuth 2.1, RFC 9728/8707/9207, RFC 8725/9068, and OWASP API2:2023; close
> parity gaps so every surface applies the **same** security level.  
> **Inherits:** [SPEC-027](../027-auth/) · [SPEC-028 MCP](../028-edgequake-query-service/mcp/) ·
> [SPEC-083](../083-improvements/) S-07…S-10 · [SPEC-087](../087-anonymous-chat/) ·
> [SPEC-152 §08](../152-new-mcp-contract/08-security-observability.md)  
> **Peers:** [SPEC-098](../098-data-access-hardening/) (spec pack shape) ·
> [docs/security/best-practices.md](../../docs/security/best-practices.md)

## Start here

1. [00-why.md](00-why.md) — Five WHYs + causal ASCII (one signer, two policies)
2. [01-first-principles.md](01-first-principles.md) — LAW-154-1…12 + DRY/SOLID
3. [02-surfaces.md](02-surfaces.md) — Code map of REST / MCP / AS / WS / WebUI
4. [03-standards-crosswalk.md](03-standards-crosswalk.md) — Normative requirement → status
5. [04-findings.md](04-findings.md) — F-154-* with file and symbol citations
6. [05-edge-cases.md](05-edge-cases.md) — EC-154 register + mitigations
7. [06-implementation-plan.md](06-implementation-plan.md) — Waves 0–6 + DoD
8. [07-e2e-test-matrix.md](07-e2e-test-matrix.md) — Gates (one row per EC)
9. [08-cross-ref.md](08-cross-ref.md) — Law ↔ finding ↔ EC ↔ wave ↔ test ↔ lens
10. [08-cross-ref.md](08-cross-ref.md) — Law ↔ finding ↔ EC ↔ wave ↔ test ↔ lens
11. Lenses → [`lenses/`](lenses/)
    - [Product Owner](lenses/LENS-product-owner.md)
    - [Full Stack](lenses/LENS-full-stack.md)
    - [Database](lenses/LENS-database.md)
    - [UX / UI](lenses/LENS-ux-ui.md)
    - [Front](lenses/LENS-front.md)
    - [Security](lenses/LENS-security.md)
    - [AI Engineer](lenses/LENS-ai-engineer.md)

## Locked decisions (Wave 0)

1. **One verifier** — REST, MCP, and WebSocket share one `CredentialDecision`
   (LAW-154-1). No parallel policy trees.
2. **Audience is a capability** — MCP resource tokens (`aud` = MCP URL) are
   rejected on REST; session tokens without MCP `aud` are rejected on `/mcp`
   (LAW-154-3). Closes F-154-01.
3. **Empty scope never means allow** — `scopes_cover` must match
   `McpAuthScopes::allows` (LAW-154-4). Closes F-154-02.
4. **API keys carry OAuth scopes** — Master key is break-glass with audit;
   user keys without `edgequake:write` cannot call write tools (LAW-154-5).
   Closes F-154-03.
5. **Membership bind is universal** — Same `enforce_membership_bind` for REST,
   MCP, and WS when `strict_tenant_bind` (LAW-154-6). Closes F-154-04.
6. **One refresh algorithm** — Web session refresh uses the OAuth rotation
   path (hash at rest, rotate, family revoke on reuse) (LAW-154-7).
   Closes F-154-05.
7. **Revocation is durable** — `jti` denylist is PostgreSQL with TTL, not an
   in-process `HashSet` (LAW-154-8). Closes F-154-05.
8. **Secrets never in URLs or localStorage** — No `?token=`; refresh in
   HttpOnly Secure cookie; SPA access token memory-only (LAW-154-9). Residual:
   non-HttpOnly `edgequake_access_token` for Next middleware. Closes F-154-06
   except that residual.
9. **Auth-off on non-local DB is fatal** — Matches JWT-secret and CORS startup
   gates; `EDGEQUAKE_DEV_MODE` is the only local bypass (LAW-154-10).
   Closes F-154-07.
10. **CI is proof** — Every EC has a named e2e gate (LAW-154-11).
11. **No second user store** — MCP OAuth reuses SPEC-027 identity (FP-AUTH-01 /
    LAW-154-12).
12. **Residual risk stays residual** — EdDSA, DPoP, mTLS are T3; trigger is
    “second service verifies tokens.”

## Surfaces

| Surface | Role | Primary files |
|---------|------|---------------|
| `edgequake-auth` | JWT / Argon2 / RBAC | `crates/edgequake-auth/src/{jwt,password,rbac,config}.rs` |
| REST middleware | `protected_api_auth` | `edgequake-api/src/middleware.rs` |
| Auth validation SSOT | Master key + stored key + JWT | `edgequake-api/src/services/auth_validation.rs` |
| MCP gateway auth | Bearer + `aud` + scopes | `edgequake-api/src/mcp/auth/gateway_auth.rs` |
| Thin AS | PKCE / PRM / CIMD / DCR / refresh | `edgequake-api/src/oauth/*` |
| Web session | Login / refresh / logout | `edgequake-api/src/handlers/auth/session.rs` |
| WebSocket | Upgrade gate | `edgequake-api/src/handlers/websocket.rs` |
| WebUI | Token store | `edgequake_webui/src/stores/use-auth-store.ts` |
| Startup | Secret / CORS / auth-off | `edgequake-api/src/startup_security.rs` |

## Architecture (one screen)

```text
  Client credential
         |
         v
  +------------------+
  | CredentialDecision|  <-- Wave 1 SSOT (LAW-154-1)
  |  profile:         |
  |   web_session     |
  |   mcp_resource    |
  |   api_key         |
  +--------+---------+
           |
     +-----+------+----------+
     |            |          |
     v            v          v
   REST         MCP         WS
   (aud !=      (aud ==     (header or
    mcp URL)     resource)   short ticket)
     |            |          |
     +-----+------+----------+
           |
           v
  membership bind (strict) + RBAC / scopes
```

## Verification (post Wave 1+)

```bash
# Existing MCP OAuth suite (must keep green)
cargo test -p edgequake-api --test spec028_mcp_oauth_e2e

# SPEC-154 gates (added per wave)
cargo test -p edgequake-api --test e2e_spec154_audience_parity
cargo test -p edgequake-api --test e2e_spec154_scope_predicate
cargo test -p edgequake-api --test e2e_spec154_api_key_scopes
cargo test -p edgequake-api --test e2e_spec154_membership_bind_mcp
cargo test -p edgequake-api --test e2e_spec154_web_refresh_rotation
cargo test -p edgequake-api --test e2e_spec154_jti_durable
cargo test -p edgequake-api --test e2e_spec154_ws_no_query_token
cargo test -p edgequake-api --test e2e_spec154_startup_auth_off_fatal
cargo test -p edgequake-auth --lib
```

WebUI gates (Wave 5): Playwright auth flows under `edgequake_webui/e2e/`.

## Wave status

| Wave | Focus | Status |
|------|-------|--------|
| 0 | Documents + finding register | **Done** |
| 1 | Single verifier + audience + scope DRY | **Done** |
| 2 | Membership bind on MCP + WS | **Done** |
| 3 | API key scopes / least privilege | **Done** |
| 4 | Web refresh rotation + durable jti | **Done** |
| 5 | Cookie refresh + WS no query token | **Done** |
| 6 | Auth-off fatal on non-local DB | **Done** |
