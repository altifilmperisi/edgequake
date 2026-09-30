# 00 — WHY (5-WHY)

Parent: [README](README.md) · Next: [01-first-principles](01-first-principles.md)

## The job to be done

An operator, customer, or AI host connecting to EdgeQuake must get **one**
security bar:

1. A stolen credential cannot escalate across surfaces (REST vs MCP vs WS).
2. MCP Authorization 2026-07-28 (OAuth 2.1 RS + PRM + PKCE) is not undermined
   by a weaker REST path that accepts the same signer.
3. Browser sessions are at least as hard to steal and revoke as agent tokens.
4. Dev shortcuts (`EDGEQUAKE_DEV_MODE`, auth-off) cannot ship on a non-local
   database by accident.

Everything below is why today's code fails that job even though MCP OAuth
itself is largely correct.

---

## 5-WHY chain A — One signer, two policies

| # | Question | Answer |
|---|----------|--------|
| 1 | Why can an MCP access JWT call `/api/v1/*`? | REST calls `verify_token` and does not require `aud` when `JWT_AUDIENCE` is unset. |
| 2 | Why is `aud` optional on the shared verifier? | Web session tokens historically omit `aud`; MCP resource tokens set `aud` to the MCP URL; one `JwtService` serves both. |
| 3 | Why share one signer? | DRY identity (FP-AUTH-01) — no parallel user store; thin AS reuses `Claims`. |
| 4 | Why does that create a confused deputy? | A least-privilege MCP token (`scope=edgequake:read`, 15m) becomes a full REST principal via `role` in claims. |
| 5 | **Root cause** | **Audience is not treated as a capability.** Same cryptographic identity, different resource servers, no profile gate. |

```text
  Stolen MCP access JWT
  aud=https://host/mcp  scope=edgequake:read  exp=+15m
           |
           +-- POST /mcp -------- aud checked, scope checked ----- least privilege
           |
           +-- POST /api/v1/* --- validate_aud=false (default) --- full API principal
                                  (role claim wins)
```

Cross-ref: [LAW-154-1](01-first-principles.md) · [LAW-154-3](01-first-principles.md) ·
[F-154-01](04-findings.md).

---

## 5-WHY chain B — MCP stricter on OAuth, looser on API keys

| # | Question | Answer |
|---|----------|--------|
| 1 | Why do OAuth MCP clients get least privilege? | Gateway uses `McpAuthScopes::allows`; empty JWT scope fails; tools map to `edgequake:read|query|write`. |
| 2 | Why do API keys bypass that? | `api_key_full()` sets `is_api_key=true` and `allows` returns true for every tool. |
| 3 | Why was that designed? | Grok / CI / headless T1 agents need a static bearer without OAuth dance. |
| 4 | Why is that unsafe at the product bar? | PRM advertises only read+query; a stored `eq_` key or master key can still call write tools and skip membership bind. |
| 5 | **Root cause** | **API keys are treated as user-equivalent Admin, not as scoped client credentials.** |

```text
  OAuth JWT path                    API key path (today)
  --------------                    --------------------
  scope claim required              is_api_key => allow all
  aud == resource URL               no aud check
  write needs edgequake:write       write always allowed
  membership bind (planned)         master key skips bind forever
```

Cross-ref: [LAW-154-5](01-first-principles.md) · [F-154-03](04-findings.md) ·
[F-154-04](04-findings.md).

---

## 5-WHY chain C — Browser session weaker than agent token

| # | Question | Answer |
|---|----------|--------|
| 1 | Why is a stolen refresh token worse for the WebUI than for MCP? | Web refresh reissues access tokens without rotating the refresh token; MCP rotates and family-revokes on reuse. |
| 2 | Why is logout incomplete across replicas? | `jti` denylist is an in-process `HashSet` in `JwtService`. |
| 3 | Why can XSS steal the session? | Access and refresh tokens live in `localStorage` via Zustand persist. |
| 4 | Why do access logs capture credentials on WS? | Upgrade accepts `?token=` in addition to `Authorization`. |
| 5 | **Root cause** | **Web session lifecycle never adopted the OAuth refresh algorithm or OWASP storage guidance.** |

```text
  MCP refresh (good)                 Web refresh (today)
  ------------------                 -------------------
  hash at rest (SHA-256)             hash at rest (lookup hash)
  rotate on use                      same token reused 30d
  family revoke on reuse             no family
  access TTL 15m                     access TTL 24h default
  jti process-local on logout        same (shared defect)
```

Cross-ref: [LAW-154-7](01-first-principles.md) · [LAW-154-8](01-first-principles.md) ·
[LAW-154-9](01-first-principles.md) · [F-154-05](04-findings.md) ·
[F-154-06](04-findings.md).

---

## 5-WHY chain D — Dev mode ships as production by accident

| # | Question | Answer |
|---|----------|--------|
| 1 | Why can a cloud DB run with auth disabled? | `validate_startup_security` only **warns** when `production_db && !auth_enabled`. |
| 2 | Why isn't that fatal like the default JWT secret? | Strictness is gated behind `EDGEQUAKE_STRICT_STARTUP`; JWT secret is always fatal without `DEV_MODE`. |
| 3 | Why does MCP also open? | `mcp_gateway_auth` inserts `api_key_full()` when `!auth_enabled`. |
| 4 | Why does anonymous chat widen the blast radius? | `allow_anonymous` defaults true when auth is off. |
| 5 | **Root cause** | **Auth-off is treated as a soft warning, not a production stop condition.** |

Cross-ref: [LAW-154-10](01-first-principles.md) · [F-154-07](04-findings.md).

---

## Causal stack (what must be true)

```text
  LAW-154-1  One verifier (CredentialDecision)
       |
       +--> LAW-154-3  Audience = capability (closes A)
       +--> LAW-154-4  Explicit scopes (closes B predicate)
       +--> LAW-154-5  Scoped API keys (closes B keys)
       +--> LAW-154-6  Membership bind everywhere (closes B bind)
       |
  LAW-154-7  One refresh algorithm ----+
  LAW-154-8  Durable revocation        +--> closes C
  LAW-154-9  No URL / localStorage secrets
       |
  LAW-154-10 Auth-off fatal on non-local DB --> closes D
       |
  LAW-154-11 CI is proof
  LAW-154-12 Single identity store (FP-AUTH-01)
```

## What this pack does **not** claim

- Replacing HS256 with EdDSA (residual — before a second verifier service).
- DPoP / mTLS (T3 enterprise).
- Rewriting SPEC-028 OAuth discovery (already met for MCP RS).
- Changing SPEC-152 tool contracts beyond scope enforcement.
