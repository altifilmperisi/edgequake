# Lens — Product Owner

Parent: [README](../README.md) · WHY: [00-why](../00-why.md) · Plan: [06-implementation-plan](../06-implementation-plan.md)

## Job

Protect customer corpus and demo tenant credibility while keeping Claude, Codex,
Cursor, and Grok integrations working.

## Threats that matter commercially

| Threat | Business impact | Spec answer |
|--------|-----------------|-------------|
| Stolen MCP read token used as full REST admin | Data exfil / delete via API | F-154-01 / Wave 1 |
| Static agent key can ingest/delete | Accidental wipe from agent tool use | F-154-03 / Wave 3 |
| Session XSS steals refresh | Persistent account takeover | F-154-06 / Wave 5 |
| Cloud deploy with auth off | Public demo becomes public corpus | F-154-07 / Wave 6 |

## Compatibility promises

1. **MCP OAuth clients** (CIMD/DCR + PKCE) keep working unchanged through Wave 6.
2. **Grok / CI bearer keys** keep working; after Wave 3 they need scopes (default
   read+query). Write access is an explicit grant — product decision, not surprise.
3. **WebUI login** UX stays email/password or OIDC; Wave 5 changes storage, not
   the form.
4. **No second identity product** — enterprise SSO remains oauth2-proxy / OIDC
   path from SPEC-027 (LAW-154-12).

## Out of scope for this epic

- Passkeys / MFA marketing feature
- Charging for “enterprise mTLS”
- Rewriting MCP tool names (SPEC-152)

## Success metrics

| Metric | Target |
|--------|--------|
| Cross-surface confused deputy | Zero passing e2e that allows MCP JWT on REST |
| Agent write without write scope | Zero |
| Auth-off on non-local DB | Process exits before listen |
| SPEC-028 OAuth suite | Remains 100% green |

## Sign-off checklist

- [ ] Accept Wave 3 default: existing API keys lose write until re-scoped
- [ ] Accept Wave 5: WS clients must stop using `?token=`
- [ ] Accept Wave 4: web access tokens expire ~15m (refresh silent)
- [ ] Residual EdDSA/DPoP deferred with documented trigger
