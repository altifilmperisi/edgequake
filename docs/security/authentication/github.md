# GitHub

GitHub is **OAuth2-only** (no OIDC id_token), so EdgeQuake does not accept it directly: use the
**Keycloak GitHub identity provider** (decision recorded in SPEC-158).

1. GitHub -> Settings -> Developer settings -> OAuth Apps -> New. Callback:
   `https://<keycloak>/realms/edgequake/broker/github/endpoint`.
2. Keycloak realm -> Identity providers -> GitHub with the client id/secret.
3. Keep **Trust Email OFF** on the broker and leave `EDGEQUAKE_OIDC_TRUST_EMAIL=false`: GitHub
   emails are user-controlled and unverified for linking purposes.
4. Put users in an Organization so the tenant claim is present (or invite them explicitly).
