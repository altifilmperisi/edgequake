# Lens — UX / UI Designer

Parent: [README](../README.md) · Findings F-154-05/06 · Wave 5: [06](../06-implementation-plan.md)

## User journeys touched

1. **Login** — email/password or OIDC (unchanged entry).
2. **Stay signed in** — silent refresh when access expires (~15m after Wave 4).
3. **Logout** — must end session on all tabs and API replicas.
4. **MCP consent** — OAuth authorize page already exists; keep scope text honest
   (read / query / write).
5. **401 recovery** — WebUI shows re-login; MCP clients follow WWW-Authenticate.

## UX principles

| Principle | Application |
|-----------|-------------|
| Honest capability | Consent screen lists only scopes that will be granted |
| No surprise lockouts | Short access TTL + silent refresh; user feels continuous session |
| Logout means logout | Copy: “Signed out on all devices” only if durable jti works |
| Fail clearly | Insufficient MCP scope → host shows reconnect / re-consent, not empty tools |

## Consent copy (MCP)

```text
  EdgeQuake is requesting:
  [x] Read documents and entities     (edgequake:read)
  [x] Search and retrieve             (edgequake:query)
  [ ] Create or delete content        (edgequake:write)  -- only if requested
```

Do not show OpenID profile scopes on the resource consent (SPEC-028 anti-pattern).

## Error states

| HTTP / MCP | User-visible |
|------------|--------------|
| 401 REST | Session expired — Sign in |
| 403 membership | No access to this workspace — Switch workspace or ask admin |
| 403 insufficient_scope | This assistant needs additional permission — Reconnect |
| 423 lockout | Too many attempts — Try again in N minutes |

## Accessibility

- Consent Approve/Deny keyboard reachable
- Error alerts with `role="alert"`
- Do not put secrets in visible URL bars (WS ticket UX)

## Sign-off

- [ ] Refresh silent path does not flash login page on healthy network
- [ ] Logout clears UI state immediately even before network round-trip completes
- [ ] Consent lists scopes matching PRM
