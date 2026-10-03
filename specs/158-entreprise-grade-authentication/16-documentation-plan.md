# 16 — Operator documentation plan

Parent: [README](README.md) · Law: [LAW-158-13](01-first-principles.md) ·
Finding: [F-158-10](04-findings.md)

Ship under `docs/security/authentication/` in Wave W7 (outline locked here).

## New pages

| Path | Audience | Content |
|------|----------|---------|
| `docs/security/authentication/index.md` | All | Decision tree: password vs Keycloak SSO vs oauth2-proxy vs DEV_MODE |
| `docs/security/authentication/keycloak-quickstart.md` | Operator | `make dev-sso`, ports, first admin, realm import caveats |
| `docs/security/authentication/google.md` | Tenant admin | Google Cloud OAuth client → Keycloak Google IdP → org domain |
| `docs/security/authentication/microsoft-entra.md` | Tenant admin | App registration v2, tid allow-list, guest rules |
| `docs/security/authentication/aws-cognito.md` | Tenant admin | User pool issuer/JWKS, Identity Center via SAML/OIDC |
| `docs/security/authentication/github.md` | Tenant admin | GitHub OAuth app, Trust Email OFF, verified email |
| `docs/security/authentication/tenant-and-roles.md` | Admin | org alias ↔ slug, role map, max_users, last owner |
| `docs/security/authentication/api-keys-and-mcp.md` | Integrator | SSO users mint keys; SPEC-154 scopes/aud |
| `docs/security/authentication/production-hardening.md` | Security | Startup gates, TLS, secrets, KC pin, backups |
| `docs/security/authentication/troubleshooting.md` | Operator | state_expired, org_unknown, import skip, replica pending |
| `docs/security/authentication/env-reference.md` | Operator | Full env table including new SSO vars |

## Updates to existing docs

| Path | Change |
|------|--------|
| [`docs/faq.md`](../../docs/faq.md) | Replace “oauth2-proxy recommended over in-process OIDC” with Keycloak primary + oauth2-proxy alternative |
| [`docs/security/best-practices.md`](../../docs/security/best-practices.md) | Add Keycloak section; keep oauth2-proxy sample labeled alternative |
| [`docs/security/index.md`](../../docs/security/index.md) | Link new authentication hub |
| [`docs/operations/deployment.md`](../../docs/operations/deployment.md) | Compose overlay pointer |
| [`docs/operations/release-and-cd.md`](../../docs/operations/release-and-cd.md) | Keycloak image publish + SPEC-158 gates |
| [`.env.example`](../../.env.example) | Expand OIDC block; note handoff; Keycloak sample |
| [`AGENTS.md`](../../AGENTS.md) | Env table rows for SSO |
| [`edgequake-auth/src/config.rs`](../../edgequake/crates/edgequake-auth/src/config.rs) | `EXTERNAL_SSO_PATTERN` docs → keycloak primary |

## Decision tree (index.md sketch)

```text
Need SSO for humans?
  No → password / API keys / DEV_MODE
  Yes → Deploy edgequake-keycloak overlay?
         Yes → Configure Organizations + brokers (recommended)
         No  → Point EDGEQUAKE_OIDC_* at existing OIDC issuer
               OR oauth2-proxy in front (legacy)
```

## Official references to cite in user docs

- https://www.keycloak.org/
- https://www.keycloak.org/server/containers
- https://www.keycloak.org/server/importExport
- https://docs.redhat.com/en/documentation/red_hat_build_of_keycloak/26.4/html/server_administration_guide/managing_organizations
- https://developers.google.com/identity/openid-connect/openid-connect
- https://learn.microsoft.com/en-us/entra/identity-platform/id-token-claims-reference
- https://docs.aws.amazon.com/cognito/latest/developerguide/federation-endpoints.html
- https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/authorizing-oauth-apps
- https://www.rfc-editor.org/rfc/rfc9700.html

## DoD for docs wave

- [ ] All pages above exist and link from `docs/security/index.md`
- [ ] FAQ/best-practices updated
- [ ] Screenshot of login SSO (optional) in `specs/158.../e2e/`
- [ ] No contradictory “OIDC not builtin” statements left
