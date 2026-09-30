# 03 — Standards crosswalk

Parent: [README](README.md) · Surfaces: [02-surfaces](02-surfaces.md) · Next: [04-findings](04-findings.md)

Sources fetched for this pack (2026-09-30):

- [MCP Authorization 2026-07-28](https://modelcontextprotocol.io/specification/2026-07-28/basic/authorization)
- [MCP Authorization 2025-11-25](https://modelcontextprotocol.io/specification/2025-11-25/basic/authorization) (subset still cited by clients)
- [OWASP API2:2023 Broken Authentication](https://owasp.org/API-Security/editions/2023/en/0xa2-broken-authentication/)
- RFC 9728 (PRM), RFC 8707 (resource indicators), RFC 9207 (`iss`), RFC 8414 (AS metadata)
- RFC 8725 (JWT BCP), RFC 9068 (JWT access token profile) — via 2026 practitioner guides
- Internal: [specs/028/…/002-oauth2-authorization-crossref.md](../028-edgequake-query-service/mcp/002-oauth2-authorization-crossref.md)

Status legend: **MET** · **GAP** · **PARTIAL** · **OOS** (out of scope / residual)

---

## MCP Authorization 2026-07-28

| # | Requirement | EdgeQuake evidence | Status | Finding / wave |
|---|-------------|--------------------|--------|----------------|
| A1 | HTTP MCP server is OAuth 2.1 resource server | `mcp_gateway_auth` + thin AS | **MET** | — |
| A2 | Protect with RFC 9728 PRM | `/.well-known/oauth-protected-resource` and `/…/mcp` | **MET** | — |
| A3 | PRM `authorization_servers[]` | `McpPublicConfig` | **MET** | — |
| A4 | `scopes_supported` | `edgequake:read`, `edgequake:query`, write when Memory/PRM profile | **MET** | Wave 3 PRM write for Memory |
| A5 | Resource indicators (RFC 8707) | `resource=` on authorize/token; JWT `aud`; REST `decide` profile gate | **MET** | Wave 1 audience parity |
| A6 | 401 + WWW-Authenticate with `resource_metadata` | `www_authenticate_bearer` | **MET** | — |
| A7 | Authorization Code + PKCE S256 | `authorize.rs` rejects non-S256 | **MET** | — |
| A8 | Prefer CIMD; DCR optional/legacy | `cimd.rs` + `register.rs` | **MET** | — |
| A9 | Validate `iss` on auth responses | AS emits `iss`; metadata flag | **MET** | — |
| A10 | Bearer in Authorization header only | MCP header; WS Authorization or `Sec-WebSocket-Protocol`; `?token=` rejected | **MET** | Wave 5 + gap-close |
| A11 | Public HTTPS token endpoint | `EDGEQUAKE_PUBLIC_URL` | **MET** (ops) | — |
| A12 | Refresh AS concern; rotate | MCP `eqr_*` + web PG family rotate (`take_web_refresh_pg`) | **MET** | Wave 4 + gap-close |
| A13 | Scope in WWW-Authenticate | Present | **MET** | — |
| A14 | STDIO uses env credentials | Adapter docs / Cursor examples | **MET** (docs) | — |

---

## OAuth 2.1 / JWT BCP / RFC 9068

| Requirement | Status | Notes |
|-------------|--------|-------|
| No Implicit / ROPC | **MET** | Code + PKCE only |
| Short access TTL (5–15m) for resource tokens | **MET** | MCP + web default 900s (SPEC-154 Wave 4) |
| Pin algorithm on verifier | **MET** | HS256 pinned in `jwt.rs` |
| Reject `alg: none` | **MET** | Default Validation |
| Validate `iss` / `aud` / `exp` / `nbf` | **MET** / **PARTIAL** | MCP + REST `decide` profile gate; `iss` optional unless configured |
| Refresh rotation for public clients | **MET** | MCP + web PG family consume |
| Sender-constrained tokens (DPoP/mTLS) | **OOS** | T3 residual |
| EdDSA / JWKS rotation | **OOS** | Trigger: second verifier service |
| `at+jwt` typ header | **OOS** | Nice-to-have; not blocking Waves 1–6 |

---

## OWASP API2:2023 / session guidance

| Requirement | Status | Finding / wave |
|-------------|--------|----------------|
| Protect credential stuffing (lockout) | **MET** | `login_lockout.rs` |
| No tokens in URL | **MET** | WS `?token=` rejected; protocol / Authorization only |
| Validate token authenticity | **MET** | Signature + exp + durable jti |
| No unsigned / weak JWT | **MET** / **PARTIAL** | HS256 shared secret OK for single service; secret length enforced at startup |
| Weak password rejected | **MET** | Strength score ≥ 3 classes |
| API keys ≠ user authentication | **MET** / **PARTIAL** | Master = break-glass; `EDGEQUAKE_API_KEYS` = read+query; stored keys scoped |
| Do not store session ids in localStorage | **MET** / **PARTIAL** | WebUI memory tokens + HttpOnly refresh; residual non-HttpOnly access cookie for Next SSR |
| Credential recovery = login strength | **OOS** | No forgot-password surface in scope |
| MFA | **OOS** | Product backlog |

---

## Internal prior specs

| Spec | Binding claim | SPEC-154 relationship |
|------|---------------|------------------------|
| SPEC-027 | Identity SSOT, login, OIDC RP | Inherit; do not fork user store (LAW-154-12) |
| SPEC-028 MCP 002 | OAuth RS requirements A1–A12 | Keep; extend REST parity |
| SPEC-028 MCP 006 | EC-MCP-* edge cases | Keep green; add EC-154-* |
| SPEC-083 S-07 | jti denylist on logout | Upgrade to durable (LAW-154-8) |
| SPEC-083 S-08 | Unknown role fail-closed | Keep |
| SPEC-083 S-09/S-10 | Fatal default JWT / CORS | Extend pattern to auth-off (LAW-154-10) |
| SPEC-087 | Anonymous chat when allowed | Constrain under Wave 6 |
| SPEC-152 §08 | MCP scopes + workspace claim | Align PRM write advertising (Wave 3) |

---

## Parity matrix (the product bar)

```text
  Control                    REST today   MCP today   Target (post Wave 6)
  -------                    ----------   ---------   --------------------
  Signature verify           yes          yes         yes
  exp / nbf                  yes          yes         yes
  aud capability             optional     required    bidirectional
  scope least privilege      role RBAC    JWT yes;    both + scoped keys
                                          keys no
  membership bind (strict)   yes          no          yes all surfaces
  refresh rotation           no           yes         yes
  durable jti revoke         no           no          yes
  no token in URL            yes          yes         yes (fix WS)
  no localStorage secrets    no           n/a         yes (cookie refresh)
  auth-off non-local fatal   warn         warn        fatal
```

Cross-ref: [04-findings](04-findings.md) · [08-cross-ref](08-cross-ref.md).
