# Lens — Keycloak Expert

Parent: [README](../README.md) · Integration: [05](../05-keycloak-integration.md) ·
IdP matrix: [06](../06-idp-matrix.md)

## Recommended topology

**One realm (`edgequake`) + Organizations** for B2B multi-tenancy — matches
Keycloak's CIAM/Organizations design (alias, domains, per-org IdPs, organization
scope/claim).

Realm-per-tenant remains valid for hard isolation but is ops-heavy; document
only as advanced.

## Version pin

- **Minimum:** Keycloak **26.8.0** (Organizations enumeration CVE-2026-4633;
  current feature set for org groups / brokers).
- Base image: `quay.io/keycloak/keycloak:26.8` (or digest-pinned).

## Realm must-haves

| Setting | Value |
|---------|-------|
| `organizationsEnabled` | true |
| Registration | false (JIT via brokers) |
| Client `edgequake-web` | confidential, standard flow, PKCE S256 |
| Front-channel logout | **false** when back-channel configured |
| `backchannel.logout.url` | EdgeQuake logout endpoint |
| `backchannel.logout.session.required` | true |
| Organization Membership mapper | add id + attributes |
| GitHub IdP Trust Email | **OFF** |
| First broker login | no automatic link for social IdPs |

## Import / config management

- `--import-realm` skips existing realms — first-boot only.
- For GitOps: prefer Admin API or `keycloak-config-cli` with careful
  `import.managed.*` so org **members** are not wiped (known footgun).
- Use `${ENV}` placeholders for secrets in realm JSON.

## Broker wiring pattern

```text
Organization alias=acme
  domains: acme.com
  IdP: google-acme (Redirect when email domain matches = ON)
→ EdgeQuake tenants.slug=acme
```

One IdP can link to only one Organization (Keycloak rule) — plan aliases
accordingly.

## Container production checklist

- [ ] `kc.sh build` in image; run `--optimized`
- [ ] Dedicated Postgres (not EdgeQuake app DB)
- [ ] Memory limit set
- [ ] Health on :9000
- [ ] Hostname / HTTPS at ingress
- [ ] Bootstrap admin from secrets, rotate after install
- [ ] No `start-dev` in prod

## EdgeQuake coupling

EdgeQuake is an OIDC **relying party** + session AS. Do **not** replace EdgeQuake
refresh cookies with Keycloak JS adapters in the SPA (BFF pattern).

Optional later: Standard Token Exchange V2 for M2M — not required for WebUI.

## Validation

- Import realm on empty DB
- Login test user → organization claim present
- Back-channel logout hits EdgeQuake (tcpdump / audit)
- Upgrade test 26.8.x → next minor
