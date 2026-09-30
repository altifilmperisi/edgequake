# Lens — Front-end Designer / WebUI Engineer

Parent: [README](../README.md) · F-154-06 · Wave 5: [06](../06-implementation-plan.md)

## Current defect

`use-auth-store.ts` persists `accessToken` and `refreshToken` through Zustand
`persist` into `localStorage`. Comment claims “secure”; XSS can read both.

Authenticated PDF/markdown helpers attach `Authorization` from that store —
keep that pattern for **in-memory** access tokens; stop persisting refresh.

## Target storage model

```text
  Browser
  -------
  Memory (Zustand, no persist of secrets)
     accessToken  -- short lived
     user profile -- OK to persist non-secrets

  Cookie (HttpOnly Secure SameSite)
     refresh_token  -- path=/api/v1/auth  (API-owned)

  Never
     localStorage refresh
     localStorage access
     ?token= on WebSocket URL
```

## Code touchpoints

| File | Change |
|------|--------|
| `stores/use-auth-store.ts` | Drop token fields from persist; fix BR0505 comment |
| `lib/api/client.ts` | Attach Bearer from memory; refresh via credentialed cookie fetch |
| WebSocket client | Use header or ticket; remove query token |
| PDF/auth image helpers | Continue Authorization from memory token |

## CORS / cookie notes

- API must set `Access-Control-Allow-Credentials` for WebUI origin
- `EDGEQUAKE_CORS_ORIGINS` already required in production (SPEC-083 S-10)
- Cookie `Secure` requires HTTPS (demo/prod); localhost exception documented

## Playwright gates

1. After login, `localStorage` has no `access`/`refresh` secret keys.
2. Reload page: session restores via cookie refresh → new access in memory.
3. Logout: cookie cleared; subsequent API calls 401.
4. WS connects without query string token.

## Visual / layout

No new chrome required for Wave 5 beyond existing login. Optional: session
expiry toast if refresh fails (offline).
