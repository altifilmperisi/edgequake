# 05 — Keycloak integration

Parent: [README](README.md) · Architecture: [04](04-architecture.md) ·
IdP matrix: [06](06-idp-matrix.md)

Grounded in:

- [Running Keycloak in a container](https://www.keycloak.org/server/containers)
- [Importing and exporting realms](https://www.keycloak.org/server/importExport)
- [Managing organizations (RHBK 26.4)](https://docs.redhat.com/en/documentation/red_hat_build_of_keycloak/26.4/html/server_administration_guide/managing_organizations)
- [Keycloak 26.8.0 release](https://www.keycloak.org/2026/10/keycloak-2680-released) (pin ≥ 26.8.0; CVE-2026-4633 org enumeration fix)
- [Protocol mappers — organization membership](https://www.keycloak.org/admin-api/protocol-mappers)

## Deliverables (Wave W4)

| Artifact | Path (planned) |
|----------|----------------|
| Optimized Containerfile | `deploy/keycloak/Containerfile` |
| Realm JSON | `deploy/keycloak/realm/edgequake-realm.json` |
| Compose overlay | `docker-compose.keycloak.yml` |
| Helm values | `deploy/kubernetes/helm/.../keycloak-values.yaml` |
| CI publish | `.github/workflows/release-keycloak.yml` → `ghcr.io/.../edgequake-keycloak` |
| Make target | `make dev-sso` |

## Image design

Follow official multi-stage pattern:

```dockerfile
FROM quay.io/keycloak/keycloak:26.8 AS builder
ENV KC_HEALTH_ENABLED=true
ENV KC_METRICS_ENABLED=true
ENV KC_DB=postgres
# COPY providers/ themes/ if any (before build)
WORKDIR /opt/keycloak
RUN /opt/keycloak/bin/kc.sh build

FROM quay.io/keycloak/keycloak:26.8
COPY --from=builder /opt/keycloak/ /opt/keycloak/
COPY realm/ /opt/keycloak/data/import/
# runtime: KC_DB_URL, KC_DB_USERNAME, KC_DB_PASSWORD, KC_HOSTNAME
ENTRYPOINT ["/opt/keycloak/bin/kc.sh"]
```

**Run (prod):** `start --optimized --import-realm` with memory limit (≥2Gi
recommended; heap via `JAVA_OPTS_KC_HEAP` / container limit per docs).

**Run (dev):** `start-dev --import-realm` acceptable for local only.

**Bootstrap admin:** `KC_BOOTSTRAP_ADMIN_USERNAME` / `KC_BOOTSTRAP_ADMIN_PASSWORD`
(required in containers — not creatable from non-local network).

**Health:** management port **9000** — `/health/ready`, `/health/live`, `/metrics`.

## Realm contract

```json
{
  "realm": "edgequake",
  "enabled": true,
  "organizationsEnabled": true,
  "registrationAllowed": false,
  "loginWithEmailAllowed": true,
  "organizations": [
    {
      "name": "Demo Acme",
      "alias": "acme",
      "domains": ["acme.example"],
      "identityProviders": [{ "alias": "google-acme" }]
    }
  ],
  "identityProviders": [ "... templates ..." ],
  "clients": [
    {
      "clientId": "edgequake-web",
      "publicClient": false,
      "secret": "${EQ_KC_CLIENT_SECRET}",
      "redirectUris": [
        "http://localhost:8080/api/v1/auth/oidc/callback",
        "https://*/api/v1/auth/oidc/callback"
      ],
      "attributes": {
        "pkce.code.challenge.method": "S256",
        "backchannel.logout.url": "https://api.example/api/v1/auth/oidc/backchannel-logout",
        "backchannel.logout.session.required": "true"
      },
      "frontchannelLogout": false,
      "standardFlowEnabled": true,
      "directAccessGrantsEnabled": false,
      "optionalClientScopes": ["organization", "organization:*"]
    }
  ]
}
```

### Organization claim

Clients request `organization` or `organization:{alias}` or `organization:*`.
Enable Organization Membership mapper options:

- Add organization id = ON
- Add organization attributes = ON (store `edgequake_tenant_slug` if needed)

Example claim shape (RHBK docs):

```json
"organization": {
  "acme": {
    "id": "42c3e46f-2477-44d7-a85b-d3b43f6b31fa"
  }
}
```

EdgeQuake maps **alias** (`acme`) → `tenants.slug` (LAW-158-5).

### Broker templates (placeholders)

| Alias | Provider | Trust Email | First login flow |
|-------|----------|-------------|------------------|
| `google-*` | google | ON only for controlled Workspace | default first broker login |
| `microsoft-*` | microsoft / oidc | ON for Entra work accounts | default |
| `cognito-*` | oidc | per pool | default |
| `github` | github | **OFF** | confirm link / re-auth |
| `saml-*` | saml | per IdP | default |

### Import caveats (official)

- `--import-realm` **skips** realms that already exist — editing JSON and
  restarting does nothing. Document `reset` / Admin API / `keycloak-config-cli`
  with `import.managed.*=no-delete` for org members.
- Env placeholders in realm JSON are supported (`${VAR}`).
- Do not treat Admin Console partial export as backup.

## Compose overlay sketch

```yaml
services:
  keycloak-db:
    image: postgres:16
    # dedicated DB — do not share EQ app schema
  keycloak:
    image: ghcr.io/raphaelmansuy/edgequake-keycloak:${EDGEQUAKE_VERSION:-latest}
    command: ["start-dev", "--import-realm"]
    environment:
      KC_DB: postgres
      KC_DB_URL: jdbc:postgresql://keycloak-db:5432/keycloak
      KC_BOOTSTRAP_ADMIN_USERNAME: admin
      KC_BOOTSTRAP_ADMIN_PASSWORD: ${KC_ADMIN_PASSWORD}
      EQ_KC_CLIENT_SECRET: ${EQ_KC_CLIENT_SECRET}
    ports:
      - "8081:8080"
      - "9000:9000"
  api:
    environment:
      EDGEQUAKE_OIDC_ENABLED: "true"
      EDGEQUAKE_OIDC_ISSUER_URL: http://keycloak:8080/realms/edgequake
      EDGEQUAKE_OIDC_CLIENT_ID: edgequake-web
      EDGEQUAKE_OIDC_CLIENT_SECRET: ${EQ_KC_CLIENT_SECRET}
      EDGEQUAKE_OIDC_REDIRECT_URI: http://localhost:8080/api/v1/auth/oidc/callback
      EDGEQUAKE_OIDC_SUCCESS_REDIRECT_URL: http://localhost:3000/auth/callback
      EDGEQUAKE_STRICT_TENANT_BIND: "true"
      EDGEQUAKE_AUTH_ENABLED: "true"
      EDGEQUAKE_DEV_MODE: "false"
```

## EdgeQuake client expectations

| Item | Value |
|------|-------|
| Grant | Authorization Code + PKCE S256 |
| Scopes | `openid email profile organization` |
| Client auth | Client secret (confidential) |
| Logout | Back-channel preferred; front-channel OFF |
| Token exchange | Not required for WebUI BFF; optional later for M2M |

## Security hardening checklist

- [ ] Pin image digest in prod Helm
- [ ] Separate Postgres for Keycloak
- [ ] No `start-dev` in prod
- [ ] Hostname v2 / HTTPS at edge
- [ ] Disable registration on realm
- [ ] GitHub Trust Email off
- [ ] Rotate `EQ_KC_CLIENT_SECRET` via secret manager
- [ ] Monitor CVE feed; 26.8.0+ for Organizations enumeration fix

## Advanced: realm-per-tenant

Documented alternative for strongest isolation (separate realm keys, themes,
IdPs). Not default — higher ops cost. Mapping table: `identity_providers.issuer`
→ tenant, no Organizations feature required.
