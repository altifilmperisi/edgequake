# Google

Two supported routes; prefer the **Keycloak Google identity provider** (one place for org mapping).

## Direct OIDC

1. Google Cloud console -> APIs & Services -> Credentials -> OAuth client (Web). Authorized
   redirect URI: `https://<api>/api/v1/auth/oidc/callback`.
2. Configure:

```bash
EDGEQUAKE_OIDC_ENABLED=true
EDGEQUAKE_OIDC_KIND=google
EDGEQUAKE_OIDC_SLUG=google
EDGEQUAKE_OIDC_ISSUER_URL=https://accounts.google.com
EDGEQUAKE_OIDC_CLIENT_ID=...apps.googleusercontent.com
EDGEQUAKE_OIDC_CLIENT_SECRET=...
EDGEQUAKE_OIDC_REDIRECT_URI=https://<api>/api/v1/auth/oidc/callback
EDGEQUAKE_OIDC_SUCCESS_REDIRECT_URL=https://<app>/auth/callback
EDGEQUAKE_OIDC_ALLOWED_HD=example.com        # Workspace hosted domain(s)
EDGEQUAKE_OIDC_TENANT_SLUG=example           # Google has no org claim: pin one tenant
```

Notes: identity is keyed on `sub` (never email). `hd` must match `ALLOWED_HD` or the login is
denied (`hd_mismatch`); consumer `@gmail.com` accounts have no `hd`.

## Via Keycloak

Add Google as an identity provider in the realm and attach the user to an Organization (domain
matching). EdgeQuake sees a normal Keycloak login with the `organization` claim.
