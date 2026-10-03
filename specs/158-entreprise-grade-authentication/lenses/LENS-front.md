# Lens — Front Designer / Web Engineer

Parent: [README](../README.md) · Arch: [10](../10-frontend-architecture.md)

## Non-negotiables

- Access token never in `localStorage` (keep SPEC-154)
- Refresh only via HttpOnly cookie
- Handoff `code` redeemed once then stripped from history (`replace`)
- `safeRedirectPath` shared — no open redirects

## Implementation notes

| Concern | Approach |
|---------|----------|
| Cross-origin API | Full page navigate to API start URL |
| CORS | Already constrained; SSO redirects are top-level |
| Middleware | Allow `/auth/callback` |
| State | Zustand auth store unchanged contract |

## Component API sketch

```tsx
<SsoProviderButtons
  providers={providers}
  orgHint={org}
  redirect={postLoginPath}
  disabled={isLoading}
/>
```

## Tests

- vitest for URL builders and redirect sanitizer
- Playwright `@spec158` for handoff
- Keep `auth-storage.spec.ts` green

## Anti-patterns

- `localStorage.setItem("refresh_token")`
- Parsing `access_token` from query "for convenience"
- Silently swallowing handoff errors
