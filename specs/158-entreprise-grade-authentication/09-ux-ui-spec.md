# 09 — UX / UI specification

Parent: [README](README.md) · Product: [03](03-product-spec.md) ·
Front arch: [10](10-frontend-architecture.md)

## Principles

1. SSO is the primary path when providers exist; password is secondary (or hidden
   when `disable_password_login` is set).
2. Never show raw tokens; never put tokens in the address bar.
3. Tenant context is visible after login; switching is intentional.
4. Errors are honest and non-enumerating where security requires.
5. a11y: keyboard, focus return, en/fr/zh strings.

## Login screen (`/(auth)/login`)

```text
+------------------------------------------+
|              EdgeQuake                   |
|  Sign in to the Knowledge Graph platform |
|                                          |
|  [ Continue with SSO                 ]   |
|  [ Continue with Google              ]   |  (if exposed)
|  [ Continue with Microsoft           ]   |
|  [ Continue with GitHub              ]   |
|                                          |
|  -------- or sign in with password ----- |
|  Username [......................]       |
|  Password [......................]  (eye)|
|  [ Sign in ]                             |
|                                          |
|  Organization (optional) [ acme____ ]    |
|  error region (aria-live)                |
+------------------------------------------+
```

### Behaviors

| State | UI |
|-------|-----|
| `GET /auth/sso/providers` empty | Hide SSO buttons; password only |
| Providers present | Show buttons; org field if Organizations enabled |
| `authEnabled=false` + demo | Keep SPEC-155 demo skip |
| First-run wizard | Unchanged when `needs_setup` |
| SSO click | Navigate to API start URL (full page) |
| Org filled | Pass `org=` to start |

## Callback route (`/auth/callback`)

New page:

1. Read `code` query (handoff) — reject if tokens present (log + strip).
2. `POST /api/v1/auth/handoff`.
3. `useAuthStore.login(response)`.
4. `router.replace(safeRedirect || '/')`.
5. Loading + error states; no flash of dashboard.

## Post-login tenant chrome

Header / workspace switcher:

- Show current tenant name + slug.
- If multiple memberships: dropdown "Switch organization" → re-issue session
  via `POST /auth/switch-tenant` (W5) or force re-SSO with `org=`.
- Never trust client-only header changes without server re-bind.

## Admin: Identity providers (settings)

Settings → Security → Identity providers (platform admin):

| Column | Content |
|--------|---------|
| Slug / kind | keycloak, google, … |
| Issuer | truncated |
| Trust email | badge |
| Enabled | toggle |
| Tenants | bound org aliases |

W5 minimum: read-only + link to docs; W5+ mutate via API if secrets via vault.

## Error copy (user-facing)

| Code | Message (en) |
|------|----------------|
| `org_unknown` | Your organization is not enrolled in EdgeQuake. Contact your admin. |
| `email_unverified` | Your identity provider has not verified your email. |
| `state_expired` | Sign-in expired. Please try again. |
| `oidc_not_configured` | Single sign-on is not available on this server. |
| generic | Sign-in failed. Try again or use password if enabled. |

Avoid "user does not exist" on SSO (enumeration).

## Responsive / a11y

- Buttons ≥ 44px touch target; focus rings.
- SSO buttons have accessible names ("Continue with Google").
- Error in `role="alert"`.
- Dark theme parity (SPEC-155 tokens).

## Non-goals

- Embedding Keycloak theme customization in WebUI.
- In-app IdP secret clipboard UX beyond admin form.
