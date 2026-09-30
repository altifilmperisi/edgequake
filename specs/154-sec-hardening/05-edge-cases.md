# 05 — Edge cases (EC-154)

Parent: [README](README.md) · Findings: [04-findings](04-findings.md) · Tests: [07-e2e-test-matrix](07-e2e-test-matrix.md)

Each row: **mitigation** must be implemented before the EC is marked closed.
Existing SPEC-028 EC-MCP-* remain in force; this register adds cross-surface cases.

| ID | Scenario | Expected | Finding | Wave | Test |
|----|----------|----------|---------|------|------|
| EC-154-01 | MCP resource JWT presented to REST `/api/v1/documents` | **401/403** — wrong audience profile | F-154-01 | 1 | `e2e_spec154_audience_parity` |
| EC-154-02 | Web session JWT (no MCP aud) presented to `POST /mcp` | **401** + WWW-Authenticate PRM | F-154-01 | 1 | same + existing EC-MCP-11 |
| EC-154-03 | OAuth JWT with empty/missing `scope` on `tools/call eq_search` | **403** insufficient_scope | F-154-02 | 1 | `e2e_spec154_scope_predicate` |
| EC-154-04 | `scopes_cover([], "edgequake:read")` | **false** (never allow-all) | F-154-02 | 1 | lib contract |
| EC-154-05 | Stored API key with scopes `edgequake:read` only calls `eq_ingest` | **403** insufficient_scope | F-154-03 | 3 | `e2e_spec154_api_key_scopes` |
| EC-154-06 | Master API key calls write tool | Allowed **and** audit event recorded | F-154-03 | 3 | same |
| EC-154-07 | Strict bind: JWT user not member of claimed workspace on MCP | **403** | F-154-04 | 2 | `e2e_spec154_membership_bind_mcp` |
| EC-154-08 | Web refresh token reused after rotation | Second use → family revoke, **401** | F-154-05 | 4 | `e2e_spec154_web_refresh_rotation` |
| EC-154-09 | Logout on replica A; access JWT on replica B before exp | **401** (durable jti) | F-154-05 | 4 | `e2e_spec154_jti_durable` |
| EC-154-10 | WS upgrade with `?token=` only | **401** (rejected) | F-154-06 | 5 | `e2e_spec154_ws_no_query_token` |
| EC-154-11 | WS upgrade with `Authorization: Bearer` | **101** when valid | F-154-06 | 5 | same |
| EC-154-12 | Non-local DATABASE_URL, auth_enabled=false, !dev_mode | Startup **Fatal** | F-154-07 | 6 | `e2e_spec154_startup_auth_off_fatal` |
| EC-154-13 | Header `X-Workspace-Id` ≠ JWT `workspace_id` under strict bind | **403** | F-154-04 | 2 | existing + MCP assert |
| EC-154-14 | JWT with `alg: none` or wrong HMAC | **401** | retained | — | auth lib / e2e |
| EC-154-15 | Default `JWT_SECRET` without DEV_MODE | Startup **Fatal** | retained | — | `startup_security` tests |
| EC-154-16 | Clock skew ≤ 30s on `nbf`/`exp` | Accepted within leeway | retained | — | jwt unit |
| EC-154-17 | Clock skew > 30s expired | **401** TokenExpired | retained | — | jwt unit |
| EC-154-18 | PRM path-inserted `/.well-known/oauth-protected-resource/mcp` | 200 JSON with resource URL | retained | — | `spec028_mcp_oauth_e2e` |
| EC-154-19 | Missing Authorization on `/mcp` when auth on | 401 + `WWW-Authenticate` title-case | retained | — | EC-MCP-11 |
| EC-154-20 | Scope `*` on OAuth JWT | Treated as full MCP scopes **only if** explicitly issued by AS (deny by default mint) | F-154-02 | 1 | scope contract |
| EC-154-21 | `allow_anonymous=true` with auth_enabled | Guest paths only where SPEC-087 allows; never MCP write | F-154-07 | 6 | chat + MCP |
| EC-154-22 | Auth disabled + DEV_MODE local | MCP/REST open; documented | F-154-07 | 6 | startup warn OK |
| EC-154-23 | Refresh token plaintext logged | Must not appear in tracing fields | hygiene | 4–5 | **Residual** — no named CI gate yet; ops hygiene |
| EC-154-24 | Concurrent refresh races (two tabs) | Exactly one succeeds; loser treated as reuse → family revoke | F-154-05 | 4 | refresh race test |
| EC-154-25 | CIMD client_id HTTPS fetch timeout / SSRF | Timeout + allow-list hosts; no file:// | retained | — | `cimd` unit |
| EC-154-26 | http redirect_uri non-loopback | Reject | retained | — | `cimd::validate_redirect_uri` |
| EC-154-27 | Token in query on REST | Not accepted (header/X-API-Key only) | retained | — | middleware |
| EC-154-28 | MCP body > `MCP_MAX_BODY_BYTES` | 413 / transport error before tool | retained | — | gateway body |
| EC-154-29 | Rate limit exceeded on `/mcp` | 429 with retry | retained | — | rate limit e2e |
| EC-154-30 | Workspace claim missing; tool passes foreign workspace_id | `eq/forbidden` (SPEC-152) | F-154-04 | 2 | MCP e2e |

---

## Mitigation patterns (ASCII)

### Audience gate (EC-154-01/02)

```text
  verify signature+exp
         |
         +-- profile mcp_resource required?
         |         |
         |         +-- aud contains resource_url? --no--> 401
         |         +-- yes --> continue with scopes
         |
         +-- profile web_session / REST?
                   |
                   +-- aud contains resource_url? --yes--> 401 (wrong surface)
                   +-- no  --> continue with RBAC
```

### Refresh reuse (EC-154-08/24)

```text
  present refresh R
         |
         v
  lookup hash(R)
         |
         +-- missing / revoked / expired --> 401
         +-- active
                |
                v
           mark old revoked; mint R2 + access
                |
                v
           if R already revoked (reuse) --> revoke entire family
```

### WS credential (EC-154-10/11)

```text
  Upgrade request
         |
         +-- Authorization Bearer | Sec-WebSocket-Protocol ticket --> validate
         +-- ?token= --> 401 (hard reject after Wave 5)
```

---

## Closed when

An EC is **closed** only when:

1. Mitigation code merged.
2. Named test in [07-e2e-test-matrix](07-e2e-test-matrix.md) is green in CI.
3. Cross-ref row updated in [08-cross-ref](08-cross-ref.md).
