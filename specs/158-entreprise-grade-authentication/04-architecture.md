# 04 — Architecture

Parent: [README](README.md) · Prev: [03-product-spec](03-product-spec.md) ·
Next: [05-keycloak-integration](05-keycloak-integration.md)

## Target flow (BFF)

```mermaid
sequenceDiagram
  participant B as Browser
  participant W as WebUI
  participant A as edgequake-api
  participant K as Keycloak
  participant I as Upstream IdP
  participant DB as PostgreSQL

  B->>W: Click SSO / org hint
  W->>A: GET /api/v1/auth/sso/start?provider=keycloak&org=acme
  A->>DB: Insert oidc_login_attempts (state, pkce, nonce, ttl)
  A-->>B: 302 to Keycloak authorize
  B->>K: AuthZ + organization scope
  K->>I: Broker login
  I-->>K: Assert identity
  K-->>B: 302 code to EdgeQuake callback
  B->>A: GET /api/v1/auth/oidc/callback?code&state
  A->>DB: Take pending; verify state
  A->>K: Token exchange + id_token verify
  A->>DB: Upsert federated_identities; JIT user+membership
  A->>DB: Insert handoff + refresh (hashed)
  A-->>B: 302 WebUI /auth/callback?code=handoff (Set-Cookie eq_refresh)
  B->>W: Load callback
  W->>A: POST /api/v1/auth/handoff {code}
  A-->>W: access_token JSON (no refresh body for SPA)
```

## ASCII system context

```text
                         +------------------+
                         |  Upstream IdPs   |
                         | G / Entra / AWS  |
                         | GitHub / SAML    |
                         +--------+---------+
                                  | broker
                                  v
  +----------+   OIDC    +--------+---------+   SQL    +-------------+
  | WebUI    | <-------> | Keycloak 26.8.x  | <------> | KC Postgres |
  | Next.js  |           | Organizations     |          +-------------+
  +----+-----+           +--------+---------+
       | BFF handoff              |
       |                          | OIDC (EdgeQuake as RP)
       v                          v
  +----+--------------------------+-----+
  |           edgequake-api             |
  |  ProviderRegistry | TenantResolver  |
  |  CredentialDecision (SPEC-154)      |
  +----+--------------------------+-----+
       |                          |
       v                          v
  +----+-----+              +-----+-----+
  | EQ PG    |              | MCP AS    |
  | identity  |              | (unchanged|
  | RLS      |              |  SPEC-154)|
  +----------+              +-----------+
```

## Module design (SOLID)

```text
edgequake-auth/
  federation/
    types.rs          FederatedIdentity, LinkPolicy
    mapper.rs         claims → MembershipRole (pure)
    org_claim.rs      parse Keycloak organization claim
  provider.rs         IdentityProvider trait (begin/complete/logout)

edgequake-api/src/services/
  provider_registry.rs    load IdPs, cache discovery/JWKS
  oidc_provider.rs        generic OIDC (openidconnect)
  tenant_resolver.rs      org/idp → tenant_id
  federation_jit.rs       transactional provision
  auth_handoff.rs         single-use codes
  oidc_pending.rs         → PG
  login_tokens.rs         shared issuer (password + SSO)
  backchannel_logout.rs   logout token verify + revoke
```

Legacy `OidcConfig::from_env` becomes `ProviderRegistry::seed_from_env()` —
one optional row — so existing deploys keep working (LAW-158-7).

## Trust boundaries

| Boundary | Rule |
|----------|------|
| Browser → WebUI | Same-origin; no tokens in query |
| WebUI → API | Bearer access (memory) + cookie refresh |
| API → Keycloak | Confidential client + PKCE; verify id_token |
| Keycloak → upstream | Broker trust; Trust Email off for GitHub |
| API → PG | RLS + membership bind |
| Keycloak → API logout | Back-channel; validate logout JWT |

## Token model

| Token | Issuer | Consumer | Lifetime |
|-------|--------|----------|----------|
| Keycloak id_token | Keycloak | API callback only | Short |
| Keycloak access (optional M2M) | Keycloak | API bearer profile | Per realm |
| EdgeQuake access JWT | EdgeQuake HS256 | REST/WS (web_session) | ~15m (SPEC-154) |
| EdgeQuake refresh | EdgeQuake | `/auth/refresh` cookie | Rotating family |
| MCP resource JWT | EdgeQuake thin AS | `/mcp` only | SPEC-154 aud |

## Tenant resolution algorithm

```text
1. Parse id_token / userinfo claims
2. If organization claim present:
     alias = first org key OR requested organization:alias
     tenant = tenants.slug == alias AND is_active
     if missing → DENY (do not fall back to default)
3. Else if identity_providers.tenant_id bound → that tenant
4. Else if native hint configured (hd/tid map table) → tenant
5. Else DENY
6. Ensure membership (JIT or existing); sync role
7. Build Claims with resolved tenant_id + default/selected workspace_id
```

## Multi-replica

Pending login rows and handoff codes are PostgreSQL with TTL indexes.
Any API replica can complete the callback (LAW-158-11).

## Compatibility modes

| Mode | Description |
|------|-------------|
| `password` | Existing |
| `sso_bff` | Default SPEC-158 path |
| `sso_bearer` | Optional: verify Keycloak access JWT via JWKS into `CredentialDecision` |
| `external_proxy` | oauth2-proxy still documented |

## Non-architecture (explicit)

- Keycloak inside the Rust binary
- Second user database for SSO
- Trusting `X-Tenant-ID` without membership bind in SSO mode
