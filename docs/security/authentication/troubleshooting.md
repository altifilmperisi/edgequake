# Troubleshooting SSO

| Symptom | Cause | Fix |
|---------|-------|-----|
| `/auth/oidc/callback` -> 503 | SSO not active | `EDGEQUAKE_OIDC_ENABLED=true` + `AUTH_ENABLED=true` |
| `?error=org_unknown` | Tenant for the org alias does not exist | Create a tenant whose slug equals the org alias |
| `?error=org_missing` | Token has no org claim (user not a member / scope missing) | Add the user to the Organization; keep the `organization` scope |
| `?error=org_ambiguous:a,b` | Several orgs, no hint | Pick one (UI picker) or pass `?org=` |
| `?error=account_exists_unlinked` | Same email already local | Sign in with password, or enable verified-email linking deliberately |
| `?error=jit_disabled` | `EDGEQUAKE_OIDC_JIT=false` | Pre-create the user/membership or enable JIT |
| `invalid_handoff` | Code expired (90 s), reused or tab restored | Start sign-in again |
| Login loops / `state_expired` | Login host != redirect URI host (state cookie) | Use the same origin for login start and `REDIRECT_URI` |
| Issuer mismatch at startup/callback | Issuer string differs from discovery `issuer` | Set Keycloak `KC_HOSTNAME`; use identical string |
| Keycloak form re-rendered after correct password | Brute-force lock (60 s after quick failures) | Wait or unlock in the Admin console |
| Realm edits ignored | Import skips existing realms | Edit in the Admin console, or drop the DB volume (dev) |
| Refresh -> 401 `session_revoked` | Back-channel logout / admin revoke | Sign in again |
| Refresh -> 403 `tenant_suspended` / `membership_revoked` | Tenant/membership changed | Restore access, sign in again |
| Pending migration at boot | SPEC-150 gate | `edgequake migrate` (or `EDGEQUAKE_SCHEMA_GATE=wait`) |

Verify the Keycloak side independently: `python3 scripts/keycloak_smoke.py` (add `--api URL`
to include the EdgeQuake handoff).
