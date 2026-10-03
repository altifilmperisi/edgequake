# Lens — Security Expert

Parent: [README](../README.md) · Threat model: [11](../11-security-threat-model.md) ·
Findings: [04](../04-findings.md)

## Top risks (ordered)

1. Email federation takeover (F-158-01) — **block merge if unfixed in W1**
2. Tokens in redirect URLs (F-158-02)
3. Unguarded tenant CRUD under JIT (F-158-04)
4. Missing logout → lingering refresh (F-158-09)
5. Misconfigured Trust Email on GitHub in realm templates

## Control inheritance

Must not regress SPEC-154:

- Audience capability (MCP vs web)
- Refresh rotation + durable jti
- No `?token=` on WS
- Auth-off fatal on non-local DB

## Standards map

| Control | Standard |
|---------|----------|
| PKCE, exact redirect, no URL tokens | RFC 9700 |
| id_token verify | OIDC Core |
| Back-channel logout | OIDC Back-Channel Logout 1.0 |
| Org claim | Keycloak Organizations docs |
| Subject stability | Google / Entra claim guides |

## Penetration-style tests (automated)

- Unverified email link attempt
- Code/state/handoff/logout replay
- Cross-tenant header spoof
- Member deletes foreign tenant
- Org unknown → no default tenant

## Residual acceptance

Documented in [11](../11-security-threat-model.md). Security sign-off required
before marking W1 and W6 done.
