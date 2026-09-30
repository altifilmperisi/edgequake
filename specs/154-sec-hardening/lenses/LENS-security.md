# Lens — Security Expert

Parent: [README](../README.md) · Findings: [04](../04-findings.md) · Crosswalk: [03](../03-standards-crosswalk.md)

## Threat model (STRIDE light)

| Asset | Threat | Finding |
|-------|--------|---------|
| Document corpus | Elevation via MCP JWT on REST | F-154-01 |
| Write tools | Confused API key as admin | F-154-03 |
| Session | XSS → localStorage refresh | F-154-06 |
| Session | Logout not global | F-154-05 |
| Deployment | Auth disabled in cloud | F-154-07 |
| Tenant boundary | Bind only on REST | F-154-04 |

## Trust boundaries

```text
  +------------------+     TLS      +------------------+
  | Browser / Agent  | -----------> | Edge reverse     |
  +------------------+              | proxy (Caddy)    |
                                    +--------+---------+
                                             |
                                             v
                                    +------------------+
                                    | edgequake-api    |
                                    |  REST RS         |
                                    |  MCP RS          |
                                    |  thin AS         |
                                    +--------+---------+
                                             |
                                             v
                                    +------------------+
                                    | PostgreSQL       |
                                    | identity+oauth   |
                                    +------------------+
```

Thin AS colocated with RS is acceptable for MCP 2026-07-28; audience separation
is still mandatory (LAW-154-3).

## Control effectiveness (today → target)

| Control | Today | Target |
|---------|-------|--------|
| Argon2id passwords | Effective | Keep |
| Login lockout | Effective | Keep |
| MCP aud + PKCE | Effective for MCP | Keep |
| Cross-surface aud | **Ineffective** | Wave 1 |
| API key scopes | **Ineffective** | Wave 3 |
| Refresh rotation (web) | **Ineffective** | Wave 4 |
| Durable revoke | **Ineffective** | Wave 4 |
| Browser storage | **Ineffective** | Wave 5 |
| Auth-off gate | Partial (warn) | Wave 6 |

## Residual risk acceptance

| Risk | Why deferred | Revisit when |
|------|--------------|--------------|
| HS256 shared secret | Single binary verifier | Second service must verify |
| No DPoP | Bearer theft window = access TTL | T3 / regulated deploy |
| No MFA | Product backlog | Phishing incidents / enterprise RFP |
| Master API key | Break-glass ops | Replace with short-lived bootstrap |

## Review checklist per PR (Waves 1–6)

- [ ] No new empty-scope allow-all
- [ ] No token in URL or logs
- [ ] Fail-first e2e attached
- [ ] Startup fatals not weakened
- [ ] Secrets not committed

## References

- MCP Authorization 2026-07-28
- OWASP API2:2023
- RFC 8725 / RFC 9068
- Internal SPEC-028 MCP 002 + 006
