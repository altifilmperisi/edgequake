# 08 — Cross-reference matrix

Parent: [README](README.md)

## Law ↔ Finding ↔ EC ↔ Wave ↔ Test ↔ Lens

| Law | Finding | ECs | Wave | Primary test | Primary lens |
|-----|---------|-----|------|--------------|--------------|
| LAW-154-1 | F-154-01, F-154-02 | 01–04 | 1 | `e2e_spec154_audience_parity`, `scope_predicate` | Full stack |
| LAW-154-2 | retained + all | 14–17 | — / 1 | jwt unit, oauth e2e | Security |
| LAW-154-3 | F-154-01 | 01, 02 | 1 | `audience_parity` | Security / AI |
| LAW-154-4 | F-154-02 | 03, 04, 20 | 1 | `scope_predicate` | Full stack |
| LAW-154-5 | F-154-03 | 05, 06 | 3 | `api_key_scopes` | AI / Product |
| LAW-154-6 | F-154-04 | 07, 13, 30 | 2 | `membership_bind_mcp` | Full stack / DB |
| LAW-154-7 | F-154-05 | 08, 24 | 4 | `web_refresh_rotation` | Database / UX |
| LAW-154-8 | F-154-05 | 09 | 4 | `jti_durable` | Database / Security |
| LAW-154-9 | F-154-06 | 10, 11 | 5 | `ws_no_query_token` + Playwright | Front / UX |
| LAW-154-10 | F-154-07 | 12, 21, 22 | 6 | `startup_auth_off_fatal` | Security / Product |
| LAW-154-11 | all | all | all | [07-e2e-test-matrix](07-e2e-test-matrix.md) | Full stack |
| LAW-154-12 | — | — | 0–6 | (no forked identity store) | Product / Full stack |

## Prior specs

| Prior | Topic | SPEC-154 link |
|-------|-------|---------------|
| [SPEC-027](../027-auth/) | Identity, login, OIDC RP | LAW-154-12; session handlers |
| [SPEC-028 MCP 002](../028-edgequake-query-service/mcp/002-oauth2-authorization-crossref.md) | OAuth RS A1–A12 | [03-standards-crosswalk](03-standards-crosswalk.md) MET rows |
| [SPEC-028 MCP 006](../028-edgequake-query-service/mcp/006-edge-cases-invariants.md) | EC-MCP-* | Keep green; EC-154 extends |
| [SPEC-083 S-07](../083-improvements/) | jti denylist | Upgrade → LAW-154-8 |
| [SPEC-083 S-08](../083-improvements/) | Unknown role | LAW-154-2 |
| [SPEC-083 S-09/S-10](../083-improvements/) | JWT secret / CORS fatal | Pattern for LAW-154-10 |
| [SPEC-087](../087-anonymous-chat/) | Anonymous chat | EC-154-21 |
| [SPEC-152 §08](../152-new-mcp-contract/08-security-observability.md) | Scopes, workspace, confirm | Waves 2–3; AI lens |
| [docs/security/best-practices.md](../../docs/security/best-practices.md) | Ops guide | Update in Waves 4–6 |

## Code symbols

| Symbol | Path | Findings |
|--------|------|----------|
| `JwtService::new` / `verify_token` | `edgequake-auth/src/jwt.rs` | F-154-01, F-154-05 |
| `access_token_claims` | `services/identity_storage.rs` | F-154-01 |
| `validate_presented_token` | `services/auth_validation.rs` | F-154-01, Wave 1 SSOT |
| `protected_api_auth` | `middleware.rs` | F-154-01, F-154-04 |
| `mcp_gateway_auth` | `mcp/auth/gateway_auth.rs` | F-154-01…04 |
| `McpAuthScopes::allows` / `api_key_full` | `oauth/types.rs` | F-154-02, F-154-03 |
| `scopes_cover` | `oauth/scopes.rs` | F-154-02 |
| `issue_tokens` / `ACCESS_TTL_SECS` | `oauth/token.rs` | retained MCP; Wave 4 web parity |
| `hash_refresh_token` / family | `oauth/store.rs` | F-154-05 pattern |
| `refresh_token` / `logout` | `handlers/auth/session.rs` | F-154-05 |
| `authorize_ws_upgrade` | `handlers/websocket.rs` | F-154-06 |
| `validate_startup_security` | `startup_security.rs` | F-154-07 |
| `useAuthStore` | `edgequake_webui/.../use-auth-store.ts` | F-154-06 |

## Document map

| Doc | Role |
|-----|------|
| [README](README.md) | Entry, locked decisions, verification |
| [00-why](00-why.md) | 5-WHY + causal ASCII |
| [01-first-principles](01-first-principles.md) | LAW-154-* |
| [02-surfaces](02-surfaces.md) | Code map |
| [03-standards-crosswalk](03-standards-crosswalk.md) | Normative MET/GAP |
| [04-findings](04-findings.md) | F-154-* |
| [05-edge-cases](05-edge-cases.md) | EC-154-* |
| [06-implementation-plan](06-implementation-plan.md) | Waves 0–6 |
| [07-e2e-test-matrix](07-e2e-test-matrix.md) | Gates |
| [lenses/LENS-product-owner](lenses/LENS-product-owner.md) | Commercial / compat |
| [lenses/LENS-full-stack](lenses/LENS-full-stack.md) | Module design |
| [lenses/LENS-database](lenses/LENS-database.md) | Schema / refresh / jti |
| [lenses/LENS-ux-ui](lenses/LENS-ux-ui.md) | Journeys / consent |
| [lenses/LENS-front](lenses/LENS-front.md) | WebUI storage / cookies |
| [lenses/LENS-security](lenses/LENS-security.md) | Threat model |
| [lenses/LENS-ai-engineer](lenses/LENS-ai-engineer.md) | Agent capabilities |

## Wave status tracker

| Wave | Status | Sign-off lenses |
|------|--------|-----------------|
| 0 Documents | **Done** | All (review) |
| 1 Verifier | **Done** | Full stack, Security, AI |
| 2 Membership bind | **Done** (gap-close: EC-07 PG + EC-30) | Full stack, Database, Security |
| 3 API key scopes | **Done** (gap-close: env ≠ break-glass; HTTP 403) | AI, Product, Security |
| 4 Refresh + jti | **Done** (gap-close: PG family via pool, not SessionStore approx) | Database, UX, Security |
| 5 Cookie + WS | **Done** (gap-close: Sec-WebSocket-Protocol + multipart cookie refresh) | Front, UX, Security |
| 6 Auth-off fatal | **Done** | Product, Security |

## Closed ECs (Waves 1–6 + gap-close)

Do **not** mark an EC closed without the named gate below green (LAW-154-11).

| EC | Gate | Notes |
|----|------|-------|
| EC-154-01…04, 20 | `e2e_spec154_audience_parity`, `e2e_spec154_scope_predicate` | Wave 1 |
| EC-154-05, 06 | `e2e_spec154_api_key_scopes` | HTTP ingest 403 + master write; env keys not break-glass |
| EC-154-07, 13, 30 | `e2e_spec154_membership_bind_mcp` | EC-07 needs `DATABASE_URL` (PG membership); EC-30 no-claim tool ws |
| EC-154-08, 24 | `e2e_spec154_web_refresh_rotation` | Memory **and** PG+SessionStore production-like |
| EC-154-09 | `e2e_spec154_jti_durable` | PG; also `validate_presented_token` durable check |
| EC-154-10, 11 | `e2e_spec154_ws_no_query_token` + WebUI vitest | Protocol auth accepted; query rejected |
| EC-154-12, 22 | `e2e_spec154_startup_auth_off_fatal` / `startup_security` | |
| EC-154-21 | `e2e_spec154_anonymous_vs_mcp` | Auth-on → UsePrincipal; MCP write without creds → 401 |
| Access cookie SSR | residual | Non-HttpOnly `edgequake_access_token` for Next middleware — Secure on HTTPS; prefer HttpOnly session later |

CI: `.github/workflows/postgres-integration.yml` → **Run SPEC-154 security hardening e2e**.

## ASCII dependency graph

```text
  00-why
     |
     v
  01-laws <------------------ lenses/*
     |
     +--> 02-surfaces
     +--> 03-crosswalk
     +--> 04-findings ----+
     |                    |
     +--> 05-edge-cases <-+
     |         |
     |         v
     +--> 06-plan -----> 07-tests
     |
     v
  08-cross-ref (this file)
```
