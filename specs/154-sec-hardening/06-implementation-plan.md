# 06 — Implementation plan (Waves 0–6)

Parent: [README](README.md) · Laws: [01-first-principles](01-first-principles.md) · Findings: [04-findings](04-findings.md)

**Status (product ≥ 0.28.4 / honesty in 0.28.5):** Waves 0–6 **Done** in code +
named Rust e2e (postgres-integration) + WebUI vitest in `release_gates.sh`.
Sign-off tracker: [08-cross-ref](08-cross-ref.md). Residuals: non-HttpOnly
access cookie, EC-154-23 log redaction, Playwright soft-skip.

This document remains the historical wave plan. Each wave lists: goal, primary
module, fail-first tests, ECs closed, DoD.

Principles: **DRY** (one decision, one bind, one refresh), **SOLID** (verifier ≠
resource policy ≠ storage), **First Principles** (fail closed, audience =
capability), **e2e before merge**.

---

## Wave 0 — Documents (this pack)

| | |
|--|--|
| Goal | Finding register + laws + lenses + test matrix |
| Code | None |
| DoD | `specs/154-sec-hardening/**` reviewed; README locked decisions |

---

## Wave 1 — Single verifier + audience + scope DRY

```text
  auth_validation / new credentials module
           |
           v
  CredentialDecision { profile, principal, scopes, claims }
           |
     +-----+------+
     |            |
  REST apply   MCP apply
  reject mcp   require mcp
  aud          aud
```

| | |
|--|--|
| Goal | LAW-154-1, 154-3, 154-4; close F-154-01, F-154-02 |
| Primary modules | `services/auth_validation.rs` (extend), `mcp/auth/gateway_auth.rs`, `middleware.rs`, `oauth/scopes.rs`, optionally `edgequake-auth` Validation pin |
| Changes | Token profiles `web_session` \| `mcp_resource` \| `api_key`. REST rejects JWT if any `aud` equals MCP resource URL. MCP unchanged aud check. `scopes_cover` → delegate to `allows` / delete empty-allow. Pin `Algorithm::HS256` explicitly on Validation. |
| Fail-first tests | `e2e_spec154_audience_parity`, `e2e_spec154_scope_predicate` |
| ECs closed | EC-154-01…04, EC-154-20 |
| DoD | Tests green; no duplicate scope helpers; REST/MCP share decode path |

**Compatibility:** Existing MCP OAuth clients unchanged. WebUI tokens still work
on REST until Wave 4 TTL change. Document that MCP tokens must not be pasted
into REST clients (now enforced).

---

## Wave 2 — Membership bind on MCP + WS

| | |
|--|--|
| Goal | LAW-154-6; close F-154-04 |
| Primary modules | `middleware.rs` (export bind helper), `mcp/auth/gateway_auth.rs`, `handlers/websocket.rs` / `ws_validate_token` |
| Changes | After `apply_authenticated_context`, call same `enforce_membership_bind` when `strict_tenant_bind`. Master key: skip bind **and** emit audit (`AuditEventType` / compliance). |
| Fail-first | `e2e_spec154_membership_bind_mcp` |
| ECs closed | EC-154-07, EC-154-13, EC-154-30 |
| DoD | Strict-bind deployment: non-member MCP call = 403 |

---

## Wave 3 — Scoped API keys

| | |
|--|--|
| Goal | LAW-154-5; close F-154-03 |
| Primary modules | API key create handler + schema, `oauth/types.rs`, `mcp/config.rs`, gateway scope insert for keys |
| Changes | Stored keys carry `scopes: Vec<String>` using OAuth names. Default `edgequake:read edgequake:query`. `api_key_full` only for audited master. `allows` uses key scopes when `is_api_key`. PRM includes `edgequake:write` iff write tools enabled. |
| Fail-first | `e2e_spec154_api_key_scopes` |
| ECs closed | EC-154-05, EC-154-06 |
| DoD | Read-only key cannot ingest/delete via MCP |

**Migration:** Existing keys without scopes → treat as read+query (fail closed
for write) or one-time expand with admin notice — **prefer fail closed for write**.

---

## Wave 4 — Web refresh rotation + durable jti

| | |
|--|--|
| Goal | LAW-154-7, LAW-154-8; close F-154-05 |
| Primary modules | `handlers/auth/session.rs`, shared refresh helper with `oauth/store.rs` patterns, new migration `jti_denylist` / reuse oauth family tables for web |
| Changes | Web refresh: rotate, hash, family revoke on reuse. Access TTL default → 900s (env override). Persist jti revoke with TTL; `JwtService::verify_token` checks PG (or port) not only `HashSet`. |
| Fail-first | `e2e_spec154_web_refresh_rotation`, `e2e_spec154_jti_durable` |
| ECs closed | EC-154-08, EC-154-09, EC-154-24 |
| DoD | Reuse of rotated refresh fails; logout visible on second process |

**DRY:** Prefer extracting `RefreshFamilyStore` used by MCP `eqr_*` and web
tokens rather than copying SQL.

---

## Wave 5 — Cookie refresh + WS without query token

| | |
|--|--|
| Goal | LAW-154-9; close F-154-06 |
| Primary modules | `handlers/auth/session.rs` (Set-Cookie), WebUI `use-auth-store.ts` + API client, `handlers/websocket.rs` |
| Changes | Refresh in HttpOnly Secure SameSite=Lax/Strict cookie on `/api/v1/auth/*`. Access token in memory only. Remove `?token=` acceptance (breaking: update WS clients). Optional short-lived WS ticket endpoint. |
| Fail-first | `e2e_spec154_ws_no_query_token`; Playwright login/logout |
| ECs closed | EC-154-10, EC-154-11 |
| DoD | No access/refresh in localStorage; WS query token returns 401 |

**UX:** Silent refresh via cookie on 401; login form unchanged.

---

## Wave 6 — Auth-off fatal on non-local DB

| | |
|--|--|
| Goal | LAW-154-10; close F-154-07 |
| Primary modules | `startup_security.rs` |
| Changes | `production_db && !auth_enabled && !dev_mode` → `Fatal` (not Warn). Keep DEV_MODE local bypass. Document `allow_anonymous` interaction. |
| Fail-first | `e2e_spec154_startup_auth_off_fatal` (unit) |
| ECs closed | EC-154-12, EC-154-21, EC-154-22 |
| DoD | Cloud URL without auth refuses boot |

---

## Ordering and dependencies

```text
  Wave 0 (docs)
     |
     v
  Wave 1 (verifier) ----+
     |                  |
     v                  v
  Wave 2 (bind)      Wave 3 (key scopes)  [can parallel after 1]
     |
     v
  Wave 4 (refresh/jti)
     |
     v
  Wave 5 (cookie/WS)     [needs Wave 4 refresh semantics]
     |
     v
  Wave 6 (startup fatal) [independent; can ship after 1]
```

---

## Explicit non-goals (residual)

| Item | Trigger |
|------|---------|
| EdDSA + JWKS | Second service verifies tokens |
| DPoP / mTLS | T3 / bank checklist |
| Passkeys | Product MFA epic |
| Replacing thin AS with external IdP only | Enterprise IdP mode (oauth2-proxy pattern remains documented) |

---

## Definition of Done (pack + product)

SPEC-154 **implementation** is done when:

1. All F-154-01…07 closed with green gates in [07](07-e2e-test-matrix.md).
2. `cargo test -p edgequake-api --test spec028_mcp_oauth_e2e` still green.
3. Parity matrix in [03](03-standards-crosswalk.md) shows Target column achieved.
4. Lenses sign-off rows in [08](08-cross-ref.md) checked.
5. Ops docs (`docs/security/best-practices.md`) updated in the wave that changes operator behavior (4–6).
