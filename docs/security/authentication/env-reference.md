# SSO environment reference

Defaults are the safest values. `.env.example` mirrors this table.

| Variable | Default | Purpose |
|----------|---------|---------|
| `EDGEQUAKE_OIDC_ENABLED` | `false` | Master switch |
| `EDGEQUAKE_OIDC_ISSUER_URL` | - | Issuer (must equal discovery `issuer`) |
| `EDGEQUAKE_OIDC_CLIENT_ID` / `_CLIENT_SECRET` | - | Confidential client |
| `EDGEQUAKE_OIDC_REDIRECT_URI` | - | `https://<api>/api/v1/auth/oidc/callback` |
| `EDGEQUAKE_OIDC_SUCCESS_REDIRECT_URL` | - | SPA landing, e.g. `https://<app>/auth/callback` |
| `EDGEQUAKE_OIDC_KIND` | `generic` | `generic` / `keycloak` / `google` / `entra` / `cognito` (sets defaults only) |
| `EDGEQUAKE_OIDC_SLUG` | kind name (`oidc` for generic) | Provider id (`?provider=`) |
| `EDGEQUAKE_OIDC_DISPLAY_NAME` | `Single sign-on` | Login button label |
| `EDGEQUAKE_OIDC_TRUST_EMAIL` | `false` | Trust `email_verified` |
| `EDGEQUAKE_OIDC_LINK_POLICY` | `never` | `never` / `verified_email` |
| `EDGEQUAKE_OIDC_JIT` | `true` | Auto-provision users |
| `EDGEQUAKE_OIDC_REQUIRE_ORG` | `false` (`true` for keycloak) | Deny logins without org |
| `EDGEQUAKE_OIDC_TENANT_SLUG` | - | Pin the provider to one tenant |
| `EDGEQUAKE_OIDC_SCOPES` | - | Extra scopes |
| `EDGEQUAKE_OIDC_ROLE_CLAIM` / `_ROLE_MAP` | keycloak `realm_access.roles`, entra `roles`, cognito `cognito:groups` / - | IdP roles -> membership role |
| `EDGEQUAKE_OIDC_DEFAULT_ROLE` / `_MAX_ROLE` | `member` / `admin` | Default and ceiling |
| `EDGEQUAKE_OIDC_ALLOWED_HD` / `_ALLOWED_TID` | - | Google domains / Entra directories |
| `EDGEQUAKE_STRICT_TENANT_BIND` | forced on with SSO (non-dev) | Tenant-bound tokens |

More providers at runtime (admin only): `GET/PUT/DELETE /api/v1/admin/identity-providers[/{slug}]`.
Public, secret-free list for the UI: `GET /api/v1/auth/sso/providers`.

Compose overlay (`docker-compose.keycloak.yml`): `KC_PUBLIC_URL`, `EQ_API_PUBLIC_URL`,
`EQ_WEB_PUBLIC_URL`, `EQ_KC_CLIENT_SECRET`, `EQ_KC_DEMO_PASSWORD`, `EQ_KC_SSL_REQUIRED`,
`EDGEQUAKE_KEYCLOAK_IMAGE`.
