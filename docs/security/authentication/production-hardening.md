# Production hardening

## Startup gates (enforced when SSO is active)

The server **refuses to start** outside `EDGEQUAKE_DEV_MODE` unless: `EDGEQUAKE_AUTH_ENABLED=true`,
`EDGEQUAKE_STRICT_TENANT_BIND` is on (forced), and every OIDC redirect URI is `https://`
(`http://localhost` is tolerated). In dev mode violations become warnings.

## Checklist

- Keycloak >= 26.8.0 (fixes CVE-2026-4633, organization enumeration); pin the image digest.
- TLS everywhere; `sslRequired=external` in the realm (the dev overlay sets `none` via
  `EQ_KC_SSL_REQUIRED`; never ship that).
- Replace demo users/passwords and `EQ_KC_CLIENT_SECRET`; store the client secret in a secret
  manager (Helm: `values-sso-keycloak.yaml.example`, `api.extraSecretEnv`).
- Issuer URL = Keycloak's frontend URL exactly; run Keycloak with `KC_HOSTNAME` set.
- Allow `POST /api/v1/auth/oidc/backchannel-logout` from Keycloak through ingress/NetworkPolicy.
- Keep `TRUST_EMAIL=false` unless the IdP verifies emails; keep `LINK_POLICY=never` by default.
- Set `MAX_ROLE` so the IdP can never create owners; keep a break-glass local admin.
- Back up Keycloak's database; realm import is create-only on an existing realm.
- Multi-replica: SSO state, handoff codes, sessions and `jti` replay protection live in PostgreSQL
  (migration 164), so any replica can complete a login started on another.
