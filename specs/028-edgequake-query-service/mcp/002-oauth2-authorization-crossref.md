# 002 — OAuth 2.0 / OIDC Authorization Cross-Reference

**Cross-ref:** [005-client-compatibility-matrix.md](./005-client-compatibility-matrix.md) | SPEC-027 OIDC  
**Sources:**
- [MCP Authorization 2026-07-28](https://modelcontextprotocol.io/specification/2026-07-28/basic/authorization)
- [MCP Authorization 2025-11-25](https://modelcontextprotocol.io/specification/2025-11-25/basic/authorization)
- [Authorization tutorial](https://modelcontextprotocol.io/docs/tutorials/security/authorization)
- [Claude Connector auth](https://claude.com/docs/connectors/building/authentication)
- [Notion MCP client guide](https://developers.notion.com/guides/mcp/build-mcp-client)
- [RFC 9728](https://datatracker.ietf.org/doc/html/rfc9728/)

---

## Normative Requirements (MCP 2026-07-28)

| # | Requirement | RFC/SEP | EdgeQuake |
|---|-------------|---------|-----------|
| A1 | MCP server acts as **OAuth 2.1 resource server** | OAuth 2.1 | JWT + API-key validation on `/mcp` |
| A2 | Publish **Protected Resource Metadata** (PRM) | RFC 9728 | `/.well-known/oauth-protected-resource` **and** path-inserted `/mcp` |
| A3 | PRM includes `authorization_servers[]` | RFC 9728 §3 | Public origin issuer (thin AS) |
| A4 | PRM SHOULD include `scopes_supported` | MCP auth | `edgequake:read`, `edgequake:query` only |
| A5 | Clients use **Resource Indicators** | RFC 8707 | `resource=` = canonical MCP URL |
| A6 | Unauthorized → **401 + WWW-Authenticate** | RFC 6750 | Includes `resource_metadata` + `scope` |
| A7 | Support **Authorization Code + PKCE (S256)** | RFC 7636 | `/oauth/authorize` + `/oauth/token` |
| A8 | Prefer **CIMD**; DCR optional/legacy | SEP-991 / draft CIMD | HTTPS `client_id` fetch + `POST /oauth/register` |
| A9 | Validate **`iss`** on auth responses | RFC 9207 | AS emits `iss`; metadata flag `true` |
| A10 | Token in `Authorization: Bearer` | OAuth 2.1 §5 | Header only |
| A11 | **`token_endpoint` MUST be public HTTPS** | Cowork bug class | `EDGEQUAKE_PUBLIC_URL` |
| A12 | Refresh tokens are AS concern | MCP auth / SEP-2207 | Rotating hashed `eqr_*` (15m access / 30d RT); family revoke on reuse; `/oauth/revoke`; **not** in PRM scopes |

---

## Discovery Flow (Cross-Client SSOT)

```
  Client                          EdgeQuake MCP Gateway              Auth Server (thin AS)
    │                                      │                                │
    │  POST /mcp (no token)                │                                │
    │ ───────────────────────────────────► │                                │
    │ ◄─────────────────────────────────── │ 401 WWW-Authenticate: Bearer   │
    │         resource_metadata=…/oauth-protected-resource/mcp              │
    │         scope="edgequake:read edgequake:query"                        │
    │                                      │                                │
    │  GET /.well-known/oauth-protected-resource/mcp                         │
    │ ───────────────────────────────────► │                                │
    │ ◄── { resource, authorization_servers: [issuer] }                      │
    │                                      │                                │
    │  GET issuer /.well-known/oauth-authorization-server                  │
    │ ─────────────────────────────────────────────────────────────────────►│
    │ ◄── authorize, token, revoke, registration, S256                      │
    │                                      │                                │
    │  [CIMD or DCR POST /oauth/register] ────────────────────────────────►│
    │  [Browser PKCE /oauth/authorize] ───────────────────────────────────►│
    │  [POST /oauth/token code+verifier+resource] ────────────────────────►│
    │                                      │                                │
    │  POST /mcp  Authorization: Bearer …  │                                │
    │ ───────────────────────────────────► │ aud=/mcp + scope check         │
    │ ◄── tools/list / tools/call          │                                │
```

---

## Protected Resource Metadata (EdgeQuake)

**Path (RFC 9728 path-inserted):** `GET https://{host}/.well-known/oauth-protected-resource/mcp`  
**Fallback:** `GET https://{host}/.well-known/oauth-protected-resource`

```json
{
  "resource": "https://demo.edgequake.com/mcp",
  "authorization_servers": [
    "https://demo.edgequake.com"
  ],
  "scopes_supported": [
    "edgequake:read",
    "edgequake:query"
  ],
  "bearer_methods_supported": ["header"],
  "resource_documentation": "https://github.com/raphaelmansuy/edgequake/tree/main/specs/028-edgequake-query-service/mcp"
}
```

### WWW-Authenticate (401 template)

```http
HTTP/1.1 401 Unauthorized
WWW-Authenticate: Bearer realm="edgequake-mcp",
  resource_metadata="https://demo.edgequake.com/.well-known/oauth-protected-resource/mcp",
  scope="edgequake:read edgequake:query"
```

**EC-MCP-28:** Use title-case `WWW-Authenticate` — some Claude proxies ignore lowercase variants.

### Insufficient scope (403)

```http
HTTP/1.1 403 Forbidden
WWW-Authenticate: Bearer error="insufficient_scope",
  scope="edgequake:query",
  resource_metadata="https://demo.edgequake.com/.well-known/oauth-protected-resource/mcp"
```

---

## Client Registration Matrix

| Client | Registration mode | Redirect URI |
|--------|-------------------|--------------|
| Claude.ai / Cowork | DCR or CIMD | `https://claude.ai/api/mcp/auth_callback` |
| Claude Code | CIMD + loopback | `http://127.0.0.1:{port}/callback`, `http://localhost:{port}/callback` |
| OpenAI Codex | DCR | Configurable via `mcp_oauth_callback_url` |
| Cursor (remote) | DCR or API key | Ephemeral localhost or custom |
| Grok remote MCP | **Bearer only** | No OAuth — pass `authorization: Bearer …` in tool config |
| Notion (reference) | OAuth only | Hosted at `https://mcp.notion.com/mcp` |

---

## Auth Modes for EdgeQuake (tiered)

| Tier | Mode | Use case | Implementation |
|------|------|----------|----------------|
| **T0 Dev** | No auth / API key | Local Cursor, tests | `EDGEQUAKE_DEV_MODE`, `X-API-Key` |
| **T1 Agent** | Static bearer | Grok, CI, headless | `Authorization: Bearer` + API keys table |
| **T2 Interactive** | OAuth 2.1 + PKCE | Claude, Codex, ChatGPT, Cursor | Thin AS `/oauth/*` + PRM |
| **T3 Enterprise** | OAuth + mTLS gateway | Bank deployments | Future SEP-990 IdP policies |

**Invariant FP-AUTH-01:** MCP OAuth reuses SPEC-027 identity — **no parallel user store**.  
**Invariant FP-AUTH-02:** Session JWTs from `/auth/login` or OIDC RP login are **not** accepted on `/mcp` unless `aud` equals the MCP resource URL.

---

## Scope Design

| Scope | Grants |
|-------|--------|
| `edgequake:read` | tools/list, initialize, edgequake_fetch |
| `edgequake:query` | edgequake_search, edgequake_retrieve |
| `edgequake:admin` | Denied on MCP surface |

Do **not** advertise `openid`, `profile`, or `offline_access` on the protected resource.

---

## Token Validation Checklist

- [x] Verify JWT signature against EdgeQuake signer
- [x] Validate `aud` contains canonical MCP resource URL
- [x] Check `exp` / `nbf`; clock skew ≤ 30s
- [x] Enforce tool scopes (`edgequake:read` / `edgequake:query`)
- [x] API keys bypass audience (full MCP access when scopes empty/`*`)
- [ ] Rate-limit per `sub` + client_id (follow-up)
- [ ] Audit log: tool name, retrieval_id, workspace_id (no chunk content)

---

## Implementation Files

```
edgequake-api/src/mcp/auth/
  protected_resource.rs   # RFC 9728 PRM
  www_authenticate.rs     # 401/403 challenges
  gateway_auth.rs         # Bearer + aud gate
edgequake-api/src/oauth/
  metadata.rs             # RFC 8414 AS metadata
  authorize.rs            # PKCE authorize + consent
  token.rs                # code + refresh exchange
  register.rs             # DCR
  cimd.rs                 # Client ID Metadata Documents
deploy/gcp/compose/snippets.caddy   # proxy /mcp + well-known + /oauth
```

---

## Anti-Patterns (observed in production)

| Anti-pattern | Symptom | Fix |
|--------------|---------|-----|
| Caddy sends `/.well-known/*` and `/mcp` to Next.js | 307 login, no OAuth | Proxy those paths to `api:8080` |
| `token_endpoint` in metadata points to localhost | Cowork: OAuth succeeds, zero POST /token | Set `EDGEQUAKE_PUBLIC_URL=https://…` |
| Root-only PRM for `/mcp` resource | Clients miss path-inserted discovery | Serve `/oauth-protected-resource/mcp` |
| Session JWT accepted on MCP | Confused deputy / over-broad tokens | Require `aud` = MCP resource |
| Identity scopes on PRM | Clients request openid/profile | Resource scopes only |
| Missing PRM on 401 | Client cannot start OAuth | Always include `resource_metadata` + `scope` |
