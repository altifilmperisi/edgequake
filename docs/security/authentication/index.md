# Authentication & SSO

EdgeQuake authenticates humans with **passwords** or **enterprise SSO (OIDC)**, and machines with
**API keys / MCP OAuth**. SSO is built in (SPEC-158): Keycloak is the primary route, and any
OIDC issuer (Google, Microsoft Entra, AWS Cognito, ...) can be used directly or brokered.

## Pick a path

```text
Need SSO for humans?
  No  -> password login / API keys            (EDGEQUAKE_AUTH_ENABLED=true)
         local hacking only: EDGEQUAKE_DEV_MODE=true
  Yes -> Run the shipped Keycloak?
         Yes -> make dev-sso / compose overlay / Helm   -> keycloak-quickstart.md
                Organizations = tenants, brokers = Google/Entra/GitHub/AWS
         No  -> point EDGEQUAKE_OIDC_* at your issuer     -> google.md / microsoft-entra.md / aws-cognito.md
         Legacy -> oauth2-proxy in front of the API      (still supported, see best-practices.md)
```

## How it works (one picture)

```text
Browser            EdgeQuake API (BFF)                 Identity provider
  | GET /auth/oidc/login?org=acme ->|                          |
  |                                  |-- state+nonce+PKCE ----->| (login, MFA, brokering)
  |<-------------------------------- 303 authorize ------------|
  | ...user authenticates at the IdP...                         |
  |------------- GET /auth/oidc/callback?code&state ----------->|
  |                      verify id_token (iss, aud, nonce, sig)  |
  |                      resolve (issuer, sub) -> user, tenant   |
  |                      JIT provision + role map, mint session  |
  |<-- 303 {SPA}/auth/callback?code=<opaque, single use> --------|
  | POST /auth/handoff {code} -> access token (memory) +         |
  |        HttpOnly eq_refresh cookie                            |
```

Design rules (see [specs/158](../../../specs/158-entreprise-grade-authentication/01-first-principles.md)):

1. IdP tokens are verified **once**, at the callback. EdgeQuake mints its own session JWT.
2. Identity key is `(issuer, subject)`; email never merges accounts unless explicitly trusted.
3. The tenant comes from the IdP organization claim, never from user input. Unknown org → denied.
4. No token ever travels in a URL; the redirect carries only a single-use code.

## Pages

| Page | Audience |
|------|----------|
| [keycloak-quickstart.md](keycloak-quickstart.md) | Operator: run Keycloak + EdgeQuake locally in minutes |
| [tenant-and-roles.md](tenant-and-roles.md) | Admin: org alias <-> tenant slug, role map, capacity |
| [google.md](google.md) · [microsoft-entra.md](microsoft-entra.md) · [aws-cognito.md](aws-cognito.md) · [github.md](github.md) | Tenant admin: connect an IdP |
| [api-keys-and-mcp.md](api-keys-and-mcp.md) | Integrator |
| [production-hardening.md](production-hardening.md) | Security |
| [troubleshooting.md](troubleshooting.md) · [env-reference.md](env-reference.md) | Operator |
