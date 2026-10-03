# 11 — Security threat model

Parent: [README](README.md) · Laws: [01](01-first-principles.md) ·
Findings: [04](04-findings.md) · Edge cases: [12](12-edge-cases.md)

## Trust boundaries

```text
  +------------------+     TLS      +------------------+
  | Browser          | -----------> | Edge proxy       |
  +------------------+              +--------+---------+
                                             |
                    +------------------------+------------------------+
                    v                                                 v
           +------------------+                              +------------------+
           | edgequake-api    |  OIDC / logout               | Keycloak         |
           | (session AS+RS)  | <--------------------------> | (identity AS)    |
           +--------+---------+                              +--------+---------+
                    |                                                 |
                    v                                                 v
           +------------------+                              +------------------+
           | EdgeQuake PG     |                              | Upstream IdPs    |
           +------------------+                              +------------------+
```

## STRIDE (SSO focus)

| Asset | Threat | Finding / control |
|-------|--------|-------------------|
| User account | Spoofing via unverified email link | F-158-01 · LAW-158-3 |
| Session | Tampering / theft via URL tokens | F-158-02 · LAW-158-4 |
| Tenant data | Elevation via default tenant + open CRUD | F-158-03/04 · LAW-158-5/6 |
| Session | Repudiation without logout audit | LAW-158-10 + audit events |
| Info disclosure | Org enumeration (KC CVE-2026-4633) | Pin KC ≥ 26.8.0 |
| Availability | IdP outage bricks cluster | Break-glass password · EC-158-16 |
| Confused deputy | MCP JWT on REST | LAW-154-3 (retain) |

## OIDC / OAuth controls (RFC 9700 + OIDC Core)

| Control | EdgeQuake requirement |
|---------|----------------------|
| PKCE S256 | Required on authorize |
| Exact redirect URI | Registered; no wildcards in prod |
| state + nonce | Required; durable store |
| id_token signature | JWKS; iss/aud/exp/nonce |
| No tokens in URLs | LAW-158-4 |
| Confidential client | Client secret for edgequake-web |
| Refresh rotation | LAW-154-7 |
| Logout token | LAW-158-10 |

## Account linking policy matrix

| IdP email_verified | trust_email | link_policy | Result |
|--------------------|-------------|-------------|--------|
| false | * | * | No email link; JIT new user on (iss,sub) only |
| true | false | never | No email link |
| true | true | verified_email | May link to existing user with same email |
| * | * | always | **Rejected at startup** in non-DEV |

## Back-channel logout validation checklist

- [ ] JWKS verify
- [ ] iss matches provider
- [ ] aud includes client_id
- [ ] events claim present
- [ ] nonce absent
- [ ] jti replay cache
- [ ] sid or sub revoke
- [ ] audit success/failure

## Residual risks

| Risk | Status | Trigger to revisit |
|------|--------|--------------------|
| HS256 session JWT | Inherited SPEC-154 | Second verifier service |
| No DPoP | Accepted | Bearer theft incidents |
| Keycloak availability | Ops | Multi-site KC |
| SCIM lag | Deferred T3 | Enterprise RFP |
| Admin Console mistakes (Trust Email on GitHub) | Docs + realm template defaults | — |

## Review checklist (every SSO PR)

- [ ] No email-as-primary-key
- [ ] No token query params
- [ ] Tenant AuthZ on new admin routes
- [ ] Fail-first e2e for security ECs
- [ ] SPEC-154 suite still green
- [ ] Secrets not committed
- [ ] Keycloak image pin documented
