# SPEC-158 — Enterprise-grade authentication, multi-tenant SSO & Keycloak

> **Status:** Implemented (W0–W7) — see [As built](13-implementation-plan.md#as-built)  
> **Product pin:** EdgeQuake v0.29.0+  
> **Scope:** Federated SSO (Keycloak as default broker for Google, Microsoft
> Entra, AWS Cognito / IAM Identity Center, GitHub), Keycloak Organizations as
> the tenant model, a published `edgequake-keycloak` image, and operator docs.  
> **Inherits:** [SPEC-027](../027-api-edgequake-audit/) ·
> [SPEC-154](../154-sec-hardening/) ·
> [SPEC-083](../083-improvements/) S-07…S-10 ·
> [SPEC-087](../087-fix-issues/) ·
> [SPEC-150](../150-reliable-migration-system/) ·
> [SPEC-152 §08](../152-new-mcp-contract/08-security-observability.md)  
> **Peers:** [SPEC-146](../146-rbac-attributes-based-securty/) ·
> [SPEC-157](../157-side-by-side-query/) (pack shape) ·
> [docs/security/best-practices.md](../../docs/security/best-practices.md)

## Start here

1. [00-why.md](00-why.md) — Five WHYs + causal ASCII
2. [01-first-principles.md](01-first-principles.md) — LAW-158-1…14 + DRY/SOLID
3. [02-surfaces.md](02-surfaces.md) — Code map of auth / tenant / OIDC / WebUI
4. [03-product-spec.md](03-product-spec.md) — Stories, personas, acceptance, non-goals
5. [04-architecture.md](04-architecture.md) — Target BFF + Keycloak Organizations
6. [04-findings.md](04-findings.md) — F-158-* with file/symbol citations
7. [05-keycloak-integration.md](05-keycloak-integration.md) — Image, realm, brokers
8. [06-idp-matrix.md](06-idp-matrix.md) — Google / Entra / AWS / GitHub / generic
9. [07-data-and-db-contract.md](07-data-and-db-contract.md) — Migration 164 design
10. [08-api-and-token-contract.md](08-api-and-token-contract.md) — Routes, claims, handoff
11. [09-ux-ui-spec.md](09-ux-ui-spec.md) — Login, org-first, tenant picker, admin IdP
12. [10-frontend-architecture.md](10-frontend-architecture.md) — Stores, routes, SOLID
13. [11-security-threat-model.md](11-security-threat-model.md) — STRIDE + RFC 9700
14. [12-edge-cases.md](12-edge-cases.md) — EC-158 register + mitigations
15. [13-implementation-plan.md](13-implementation-plan.md) — Waves W0–W7 + DoD
16. [14-e2e-test-matrix.md](14-e2e-test-matrix.md) — One gate per EC
17. [15-cross-ref.md](15-cross-ref.md) — Law ↔ WHY ↔ F ↔ EC ↔ wave ↔ test ↔ lens
18. [16-documentation-plan.md](16-documentation-plan.md) — Operator docs to ship
19. Lenses → [`lenses/`](lenses/)
    - [Product Owner](lenses/LENS-product-owner.md)
    - [Full Stack](lenses/LENS-full-stack.md)
    - [Database](lenses/LENS-database.md)
    - [UX / UI](lenses/LENS-ux-ui.md)
    - [Front](lenses/LENS-front.md)
    - [Security](lenses/LENS-security.md)
    - [Keycloak Expert](lenses/LENS-keycloak.md)

## Locked decisions (Wave 0)

1. **Keycloak Organizations = EdgeQuake tenants** — One Keycloak realm
   (`edgequake`); one Organization per tenant; Organization alias maps to
   `tenants.slug`. Realm-per-tenant is documented as an advanced ops path, not
   the product default.
2. **Published `edgequake-keycloak` image** — Official Keycloak base, optimized
   multi-stage build, EdgeQuake realm + theme baked; compose/Helm overlay
   alongside the stock stack. Not an all-in-one JVM+Rust binary.
3. **BFF, one signer** — Browser SSO ends with EdgeQuake minting its own session
   JWT (same `CredentialDecision`, jti denylist, refresh rotation as SPEC-154).
   Keycloak RS256 tokens are verified only at the OIDC callback (and optional
   M2M bearer mode via the same verifier profile).
4. **Identity key is `(issuer, subject)`** — Never email alone. Email may link
   accounts only when `email_verified` **and** the IdP is flagged `trust_email`
   (default policy: `never`).
5. **Tenant from IdP binding, not headers** — Keycloak `organization` claim
   resolves tenant; unknown org means deny, not default tenant. Headers remain
   advisory under `strict_tenant_bind`.
6. **JIT then bind** — Federated first login creates user + membership
   atomically; roles re-sync from claim maps on each login, capped by IdP
   config.
7. **Strategy + registry (SOLID/DRY)** — `IdentityProvider` trait; generic
   `OidcProvider` for Keycloak/Google/Entra/Cognito; native GitHub OAuth as T2;
   DB-backed registry with cached discovery/JWKS; legacy `EDGEQUAKE_OIDC_*`
   env becomes a shim that seeds one provider row.
8. **Fail closed when SSO is active** — Auth enabled, `strict_tenant_bind` on,
   secure cookies, https redirect URIs unless `EDGEQUAKE_DEV_MODE`.
9. **No secrets in URLs** — OIDC success handoff uses single-use code +
   HttpOnly `eq_refresh` cookie (closes F-158-02; upholds LAW-154-9).
10. **Keycloak brokers hyperscalers** — Google, Microsoft, Cognito/Identity
    Center, GitHub are configured as Keycloak IdPs in the shipped realm; EdgeQuake
    talks OIDC to Keycloak.
11. **CI is proof** — Every EC-158 maps to a named gate (LAW-158-14).
12. **Docs replace oauth2-proxy as the enterprise answer** — oauth2-proxy remains
    a documented alternative; Keycloak + builtin OIDC is the supported path.

## Job in one screen

```text
  Analyst / Operator
         |
         v
  +------------------+     PKCE + organization scope     +------------------+
  | EdgeQuake WebUI  | --------------------------------> | Keycloak 26.8.x  |
  | /login SSO btns | <-------------------------------- | Organizations    |
  +--------+---------+     code (no tokens in URL)       | Google/Entra/... |
           |                                             +--------+---------+
           | handoff code + Set-Cookie eq_refresh                 |
           v                                                      v
  +------------------+                                   Federated brokers
  | edgequake-api    |                                   (hyperscaler IdPs)
  | verify (iss,sub) |
  | JIT membership   |
  | mint EQ JWT      |
  +--------+---------+
           |
           v
  PostgreSQL: users + federated_identities + memberships + RLS
```

## Non-goals (explicit)

- Embedding Keycloak inside the EdgeQuake Rust binary.
- Replacing EdgeQuake HS256 session tokens with Keycloak access tokens for the
  WebUI (optional M2M bearer mode is a separate profile).
- Building a second user store for SSO users (LAW-154-12 / LAW-158-2).
- Native SAML SP in EdgeQuake (SAML terminates at Keycloak).
- SCIM provisioning in Wave 1–6 (deferred; Keycloak SCIM is tracked as T3).
- Replacing SPEC-154 session/cookie/MCP audience work.

## Success narrative

When Waves W0–W7 land, an enterprise operator can:

1. `docker compose -f docker-compose.quickstart.yml -f docker-compose.keycloak.yml up`
   and get EdgeQuake + Keycloak with Organizations enabled.
2. Configure Google / Entra / Cognito / GitHub once in Keycloak (or via the
   shipped realm templates) and map Organizations to EdgeQuake tenants.
3. Sign in from the WebUI via SSO; land in the correct tenant with correct role;
   never see tokens in the browser URL or localStorage.
4. Deprovision a user in Keycloak and have EdgeQuake revoke sessions via
   back-channel logout.
5. Trust CI: Rust `e2e_spec158_*` and Playwright `@spec158` green.

## Validate locally

```bash
python3 specs/158-entreprise-grade-authentication/scripts/validate-cross-ref.py
```

Cross-ref: [00-why](00-why.md) · [15-cross-ref](15-cross-ref.md).
