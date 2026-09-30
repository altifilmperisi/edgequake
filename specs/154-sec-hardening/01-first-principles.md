# 01 — First Principles (SPEC-154)

Parent: [README](README.md) · WHY: [00-why](00-why.md) · Next: [02-surfaces](02-surfaces.md)

## Axioms

1. **Authentication answers “who”; authorization answers “what.”** A valid JWT
   is not a blank check for every route.
2. **Audience is a capability token.** A token minted for resource R must not
   authorize resource S.
3. **Scopes are explicit.** Absence of a scope claim means deny for OAuth JWTs;
   API keys declare scopes at mint time.
4. **One identity store.** MCP thin AS and Web login share SPEC-027 users
   (FP-AUTH-01). No parallel principal table.
5. **Revocation must survive process death and horizontal scale.** In-memory
   denylists are not SSOT.
6. **Fail closed.** Unknown role, missing `aud` where required, claim/header
   mismatch under strict bind, and auth-off on non-local DB are hard errors.
7. **Evidence beats vibes.** Every law maps to a named e2e gate (LAW-154-11).

## Laws

| Law | Statement |
|-----|-----------|
| **LAW-154-1** | One verifier — REST, MCP, and WebSocket obtain a single `CredentialDecision` from one module; route layers only apply profile policy. |
| **LAW-154-2** | Fail closed — unknown roles, expired tokens, revoked `jti`, and missing required claims reject the request. |
| **LAW-154-3** | Audience is a capability — MCP resource tokens (`aud` contains MCP resource URL) are rejected on REST; non-resource tokens are rejected on `/mcp`. |
| **LAW-154-4** | Scopes are explicit — empty grant never means allow-all for OAuth JWTs; one predicate (`allows`) is SSOT; `scopes_cover` is a thin alias or deleted. |
| **LAW-154-5** | API keys are scoped clients — stored keys carry OAuth-compatible scopes; without `edgequake:write` write tools fail; master key is break-glass with audit. |
| **LAW-154-6** | Membership bind is universal — when `strict_tenant_bind`, the same `enforce_membership_bind` runs for REST, MCP, and WS (master key audited exception). |
| **LAW-154-7** | One refresh algorithm — web session refresh uses hash-at-rest, rotate-on-use, family revoke on reuse (same semantics as MCP `eqr_*`). |
| **LAW-154-8** | Revocation is durable — access-token `jti` denylist lives in PostgreSQL with TTL ≈ remaining `exp`; every replica consults it. |
| **LAW-154-9** | Secrets never in URLs or JS-readable storage — no `?token=` on WS; refresh in HttpOnly Secure cookie; SPA access token memory-only. |
| **LAW-154-10** | Auth-off on non-local DB is fatal — same class as default JWT secret and open CORS; only `EDGEQUAKE_DEV_MODE` bypasses locally. |
| **LAW-154-11** | CI is proof — every EC-154 has a named test; green suite is the DoD for each wave. |
| **LAW-154-12** | Single identity store — MCP OAuth and Web auth share users/memberships; no second principal database. |

## DRY / SOLID

| Principle | Application |
|-----------|-------------|
| **DRY** | `CredentialDecision` + `auth_validation` SSOT; one scope predicate; one membership bind; one refresh store helper for web + MCP families. |
| **SRP** | Verifier decides identity+profile; resource server (REST/MCP/WS) enforces route policy; thin AS mints tokens; storage persists refresh/jti. |
| **OCP** | New surfaces consume `CredentialDecision` without forking JWT decode logic. |
| **LSP** | Every surface that claims “authenticated” must accept the same decision shape (scopes, aud profile, principal). |
| **ISP** | Token profiles (`web_session`, `mcp_resource`, `api_key`) expose only the claims that profile needs. |
| **DIP** | Handlers depend on the decision trait/module, not on raw `Claims` decoding in three places. |

## Token profiles (normative)

```text
  Profile        aud requirement              scope requirement
  -------        -----------------            -----------------
  web_session    MUST NOT equal MCP URL       role + tenant/workspace claims
                 (optional own aud later)
  mcp_resource   MUST contain MCP resource    edgequake:read|query|write explicit
  api_key        N/A (opaque secret)          scopes stored on key record
```

## Relationship to retained controls

These remain binding; SPEC-154 does not reopen them:

- Argon2id passwords + strength check (`edgequake-auth::PasswordService`)
- Login lockout (`login_lockout.rs`)
- MCP PRM / WWW-Authenticate / PKCE S256 / CIMD / rotating `eqr_*`
- Startup refuse of default/short `JWT_SECRET` without `DEV_MODE`
- HS256-only verification (rejects `alg: none`)
- Bearer header-only on `/mcp`
- Rate limit on MCP POST

## Residual risk (explicitly out of Waves 1–6)

| Item | Trigger to act |
|------|----------------|
| HS256 shared secret → EdDSA / JWKS | Second independent service must verify tokens |
| DPoP / mTLS sender-constrained tokens | Bank / T3 deployment checklist |
| Passkeys for human login | Product prioritizes phishing-resistant UX |

Cross-ref: [03-standards-crosswalk](03-standards-crosswalk.md) · [06-implementation-plan](06-implementation-plan.md).
