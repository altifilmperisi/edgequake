# 01 — First principles (LAW-158)

Parent: [README](README.md) · Prev: [00-why](00-why.md) · Next: [02-surfaces](02-surfaces.md)

These laws are **non-negotiable** for SPEC-158. Implementation waves may defer
features, but must not ship code that violates a law.

## DRY / SOLID application

| Principle | Application in SPEC-158 |
|-----------|-------------------------|
| **DRY** | One login token issuer shared by password + OIDC; one `CredentialDecision` (LAW-154-1); one claim→role mapper; one discovery/JWKS cache. |
| **SRP** | Pure mapping in `edgequake-auth`; HTTP/DB in `edgequake-api` services; Keycloak packaging under `deploy/keycloak/`. |
| **OCP** | New IdPs = new `IdentityProvider` impl or Keycloak broker config — not forks of `oidc_callback`. |
| **LSP** | Every provider returns the same `FederatedIdentity` DTO; handlers never special-case Google vs Keycloak. |
| **ISP** | Split begin/complete/logout traits; GitHub OAuth need not implement OIDC id_token verify. |
| **DIP** | Handlers depend on `ProviderRegistry` + `TenantResolver` traits, not env globals. |

---

## LAW-158-1 — Keycloak is the enterprise identity plane

EdgeQuake ships a supported Keycloak integration: published `edgequake-keycloak`
image (optimized, realm import), compose overlay, Helm values, and operator
docs. Hyperscaler IdPs (Google, Microsoft Entra, AWS Cognito / Identity Center,
GitHub) are **brokered through Keycloak** by default.

- **Closes:** WHY chain E · F-158-10
- **Tests:** W4 image smoke, compose e2e, realm import gate

## LAW-158-2 — Single identity store (extends LAW-154-12)

SSO users live in the same `users` / `memberships` / refresh / jti tables as
password users. No parallel "OIDC-only" user table. Federation adds
`federated_identities` as a **link table**, not a second principal store.

- **Closes:** FP-AUTH-01 continuity
- **Tests:** JIT user appears in `/api/v1/users` and membership bind

## LAW-158-3 — Identity key is `(issuer, subject)`

Federation lookup is `UNIQUE(issuer, subject)`. Email is never a primary key.
Account linking by email requires **both** IdP `email_verified` (or Keycloak
Trust Email with Sync FORCE) **and** provider flag `trust_email=true`. Default
policy is `never`. Fabricated emails (`oidc-{sub}@…`) must not match existing
users.

- **Closes:** WHY chain A · F-158-01 · EC-158-01…04
- **Tests:** `e2e_spec158_federation_key`, takeover fail-first

## LAW-158-4 — No secrets in redirects (upholds LAW-154-9)

OIDC success must not put `access_token` or `refresh_token` in query strings.
Completion uses a single-use `auth_handoff` code + `Set-Cookie: eq_refresh`
(HttpOnly, Secure when HTTPS, SameSite=Lax, Path=`/api/v1/auth`).

- **Closes:** WHY chain B · F-158-02 · EC-158-10
- **Tests:** `e2e_spec158_handoff_no_token_url`, Playwright storage checks

## LAW-158-5 — Tenant from IdP binding, not spoofable headers

For SSO sessions, `Claims.tenant_id` / membership derive from:

1. Keycloak `organization` claim alias → `tenants.slug`, or
2. Explicit IdP→tenant binding in `identity_providers`, or
3. Documented native hints (`tid`, `hd`) only when configured.

Unknown / missing org → **deny** (403/401), never silent default tenant.
`strict_tenant_bind` is forced **on** when any SSO provider is enabled
(unless `EDGEQUAKE_DEV_MODE`).

- **Closes:** WHY chain C · F-158-03 · F-158-05 · EC-158-11…13
- **Tests:** `e2e_spec158_org_tenant_bind`, cross-tenant deny

## LAW-158-6 — Tenant lifecycle APIs require authorization

`POST/GET/PUT/DELETE /api/v1/tenants*` require authenticated principal **and**
platform admin or tenant owner/admin membership as appropriate. List is scoped
to memberships unless platform admin. Fail-first e2e proves today's gap.

- **Closes:** WHY chain D · F-158-04 · EC-158-14
- **Tests:** `e2e_spec158_tenant_crud_authz` (fail-first then green)

## LAW-158-7 — Provider registry + Strategy pattern

Multiple IdPs are first-class: DB-backed `identity_providers` +
`ProviderRegistry`. Generic OIDC covers Keycloak and hyperscaler OIDC issuers.
Discovery metadata and JWKS are cached with TTL and key rotation. Legacy
`EDGEQUAKE_OIDC_*` seeds at most one shim row for backward compatibility.

- **Closes:** F-158-07 · F-158-08 · EC-158-15…17
- **Tests:** unit registry; wiremock multi-issuer

## LAW-158-8 — JIT provision then bind; roles re-sync

First successful federation creates `users` + `memberships` (+ optional default
workspace membership) in one transaction. Role comes from a deterministic
claim→`MembershipRole` mapper capped by IdP config. Each login re-applies the
mapper (groups removed at IdP → privilege drop). Honour `tenants.max_users`.

- **Closes:** F-158-03 · EC-158-18…20
- **Tests:** `e2e_spec158_jit_membership`, role downgrade

## LAW-158-9 — SSO mode fail-closed at startup

When any SSO provider is runtime-active:

| Gate | Required |
|------|----------|
| `auth_enabled` | true (else fatal, LAW-154-10) |
| `strict_tenant_bind` | true (unless DEV_MODE) |
| Redirect URIs | `https` (unless DEV_MODE) |
| Cookie Secure | true behind HTTPS / `EDGEQUAKE_COOKIE_SECURE` |
| JWT secret | not default (existing gate) |

- **Closes:** F-158-05 · EC-158-21
- **Tests:** `e2e_spec158_startup_sso_gates`

## LAW-158-10 — Back-channel logout ends EdgeQuake sessions

Keycloak client configures `backchannel.logout.url` to EdgeQuake. Handler
validates logout token (iss, aud, events, no nonce, JWKS), revokes by `sid`
and/or `sub` (refresh family), and denylists every access-token `jti` recorded
for that family. A token that was never recorded still expires with its TTL.
Refresh rows carry `idp_sid` for precise revoke.

- **Closes:** F-158-09 · EC-158-22…23
- **Tests:** `e2e_spec158_session::backchannel_logout_denylists_handoff_access_token`

## LAW-158-11 — Durable OIDC pending state

PKCE verifier, nonce, and CSRF state live in PostgreSQL
(`oidc_login_attempts`) with short TTL, not process-local memory. Required for
multi-replica API.

- **Closes:** F-158-06 · EC-158-24
- **Tests:** multi-instance or shared-store contract test

## LAW-158-12 — Hyperscaler matrix via brokers (+ GitHub T2)

| Provider | Path | Notes |
|----------|------|-------|
| Google | Keycloak Google IdP (or native OIDC) | Key on `sub`; check `hd` when restricted |
| Microsoft Entra | Keycloak Microsoft / OIDC | Key on `oid`+`tid`; email mutable |
| AWS Cognito / Identity Center | Keycloak OIDC/SAML | Cognito issuer formats documented |
| GitHub | Keycloak GitHub broker (primary); native OAuth T2 | No id_token; Trust Email off |

- **Closes:** WHY E · F-158-08 · [06-idp-matrix](06-idp-matrix.md)
- **Tests:** broker e2e with simulated upstream realms

## LAW-158-13 — Documentation is a deliverable

Operator docs under `docs/security/authentication/` ship with the feature:
quickstart, per-IdP guides, tenant/role mapping, troubleshooting, env
reference. FAQ/best-practices stop claiming oauth2-proxy is the only enterprise
path.

- **Closes:** F-158-10 · [16-documentation-plan](16-documentation-plan.md)
- **Tests:** link checker / validate-cross-ref; doc presence in release gate

## LAW-158-14 — CI is proof

Every EC-158 has a named automated gate in [14-e2e-test-matrix](14-e2e-test-matrix.md).
No "manual only" for security-critical ECs without an explicit residual-risk
row in the security lens.

- **Closes:** pack completeness
- **Tests:** `scripts/validate-cross-ref.py`

---

## Inheritance from SPEC-154 (must remain green)

| LAW-154 | Relationship |
|---------|--------------|
| LAW-154-1 One verifier | SSO sessions use `CredentialDecision` |
| LAW-154-3 Audience capability | Web session ≠ MCP resource |
| LAW-154-7 One refresh algorithm | OIDC issued refresh uses same rotation |
| LAW-154-8 Durable jti | Logout / back-channel share denylist |
| LAW-154-9 No URL/localStorage secrets | Strengthened by LAW-158-4 |
| LAW-154-12 Single identity store | Strengthened by LAW-158-2 |

## Official standards grounding

| Standard / doc | Use |
|----------------|-----|
| [OpenID Connect Core 1.0](https://openid.net/specs/openid-connect-core-1_0.html) | Code + PKCE, nonce, id_token verify |
| [RFC 9700](https://www.rfc-editor.org/rfc/rfc9700.html) (OAuth 2.0 Security BCP) | Exact redirect URIs, PKCE, no tokens in URLs |
| [OIDC Back-Channel Logout](https://openid.net/specs/openid-connect-backchannel-1_0.html) | LAW-158-10 |
| [Keycloak Organizations](https://docs.redhat.com/en/documentation/red_hat_build_of_keycloak/26.4/html/server_administration_guide/managing_organizations) | Tenant model |
| [Keycloak containers](https://www.keycloak.org/server/containers) | Image build |
| [Keycloak importExport](https://www.keycloak.org/server/importExport) | Realm import caveats |
| [Google OIDC](https://developers.google.com/identity/openid-connect/openid-connect) | `sub`, `hd`, `email_verified` |
| [Entra ID token claims](https://learn.microsoft.com/en-us/entra/identity-platform/id-token-claims-reference) | `oid`, `tid`, mutable email |
| [Cognito federation endpoints](https://docs.aws.amazon.com/cognito/latest/developerguide/rest-api-federation.html) | Issuer / JWKS |
| [GitHub OAuth apps](https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/authorizing-oauth-apps) | PKCE, no OIDC |
| MCP Authorization 2026-07-28 | Do not weaken MCP RS |

## Residual risks (accepted until trigger)

| Risk | Why deferred | Revisit when |
|------|--------------|--------------|
| Native SAML SP | Keycloak terminates SAML | Customer forbids Keycloak |
| SCIM inbound | Keycloak SCIM / Admin API | Large enterprise RFP |
| EdDSA / asymmetric session JWT | Single binary verifier | Second service verifies EQ tokens |
| DPoP | Bearer theft window = access TTL | Regulated deploy |
