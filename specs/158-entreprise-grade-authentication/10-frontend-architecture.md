# 10 — Frontend architecture

Parent: [README](README.md) · UX: [09](09-ux-ui-spec.md) · API: [08](08-api-and-token-contract.md)

## Modules (DRY / SOLID)

```text
src/
  app/(auth)/
    login/page.tsx              # SSO buttons + password
    callback/page.tsx           # NEW handoff redeem
  lib/api/edgequake/
    auth.ts                     # login + ssoProviders + handoff
    sso.ts                      # NEW thin client helpers
  lib/auth/
    safe-redirect.ts            # EXTRACT from login page (DRY)
    strip-legacy-token-params.ts
  stores/
    use-auth-store.ts           # unchanged storage rules (LAW-154-9)
    use-tenant-store.ts         # membership-scoped lists
  providers/
    tenant-provider.tsx         # multi-membership awareness
  components/auth/
    sso-provider-buttons.tsx    # NEW
    org-hint-field.tsx          # NEW
  locales/{en,fr,zh}.json       # SSO strings
```

## Auth store invariants (keep)

From [`use-auth-store.ts`](../../edgequake_webui/src/stores/use-auth-store.ts):

- Access token: memory / client module only
- Refresh: HttpOnly cookie via API — store keeps `refreshToken: null`
- Persist: `isAuthenticated`, `user`, `expiresAt` only

Handoff path calls the same `login(LoginResponse)`.

## SSO start URL construction

```ts
// sso.ts
export function buildSsoStartUrl(opts: {
  apiBase: string;
  provider: string;
  org?: string;
  redirect?: string;
}): string {
  const u = new URL("/api/v1/auth/sso/start", opts.apiBase);
  u.searchParams.set("provider", opts.provider);
  if (opts.org) u.searchParams.set("org", opts.org);
  if (opts.redirect) u.searchParams.set("redirect", opts.redirect);
  return u.toString();
}
```

Full navigation (`window.location.assign`) — not XHR — so cookies/Set-Cookie on
callback work across API host.

## Proxy / middleware

[`proxy.ts`](../../edgequake_webui/src/proxy.ts) +
[`proxy-guards.ts`](../../edgequake_webui/src/lib/server/proxy-guards.ts):

- Allow unauthenticated access to `/login`, `/auth/callback`
- Keep session cookie mirror for dashboard routes
- Do not treat handoff `code` as session proof

## Runtime config

Extend `getRuntimeConfig()` / health-derived flags:

| Flag | Source |
|------|--------|
| `ssoProviders` | `/health` or `/auth/sso/providers` |
| `authEnabled` | existing |
| `disableDemoLogin` | existing |

## Tenant provider changes

After login, load memberships (`GET /tenants` scoped). If JWT `tenant_id`
mismatches selected UI tenant, prefer JWT (server wins). Switch-tenant API
updates cookies/tokens before mutating Zustand.

## Testing

| Layer | What |
|-------|------|
| vitest | `safe-redirect`, `buildSsoStartUrl`, strip legacy token params |
| Playwright `@spec158` | SSO button → mock handoff → dashboard; no token in URL |
| Existing | `auth-storage.spec.ts` remains green |

## i18n keys (minimum)

- `auth.sso.continue`
- `auth.sso.google` / `.microsoft` / `.github` / `.keycloak`
- `auth.sso.orgHint`
- `auth.callback.loading` / `.error`
- `auth.errors.org_unknown` / `.state_expired` / …

## Non-goals

- Embedding Keycloak JS adapter in the browser (BFF only)
- Storing IdP tokens in Zustand
