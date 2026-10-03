# 00 — WHY (5-WHY)

Parent: [README](README.md) · Next: [01-first-principles](01-first-principles.md)

## The job to be done

An enterprise customer must connect EdgeQuake to their existing identity
plane (Google Workspace, Microsoft Entra ID, AWS Cognito / IAM Identity Center,
GitHub Enterprise) and get:

1. **Single sign-on** for humans (WebUI) without EdgeQuake becoming a password
   vault for those users.
2. **Tenant isolation** that matches their org structure — not a shared default
   tenant for every federated login.
3. **Least privilege** that survives IdP group/role changes and deprovisioning.
4. **A shippable Keycloak path** (image + compose/Helm + docs) so SSO is not an
   "assemble oauth2-proxy yourself" exercise.

Today's code has a thin OIDC RP (SPEC-027 phase 54) and solid session hygiene
(SPEC-154), but federation is single-IdP, email-linked, default-tenant scoped,
and still leaks tokens into redirect URLs. That is why this pack exists.

---

## 5-WHY chain A — Federated login = account takeover by email

| # | Question | Answer |
|---|----------|--------|
| 1 | Why can an attacker take over an existing EdgeQuake user via OIDC? | `resolve_or_create_oidc_user` calls `find_user_by_login(..., &identity.email)` and links on email match. |
| 2 | Why is email treated as identity? | `OidcIdentity` only carries `subject`, `email`, `username`; there is no `(issuer, subject)` table and no `email_verified` check. |
| 3 | Why does missing email still succeed? | `complete_login` fabricates `oidc-{subject}@edgequake.local` when the IdP omits email. |
| 4 | Why is that fatal for GitHub / social IdPs? | Unverified profile emails are common; Keycloak docs warn Trust Email + auto-link is a takeover vector for GitHub. Google/Entra docs say never key on email. |
| 5 | **Root cause** | **Email is used as a primary key for federation instead of `(issuer, subject)` with an explicit trust policy.** |

```text
  Attacker IdP account email=victim@corp.com (unverified)
           |
           v
  GET /api/v1/auth/oidc/callback
           |
           +--> find_user_by_login(email) --> existing victim UserRecord
           +--> issue EQ JWT as victim
           |
           X  no email_verified check
           X  no (iss, sub) store
```

Cross-ref: [LAW-158-3](01-first-principles.md) · [F-158-01](04-findings.md) ·
[EC-158-01](12-edge-cases.md).

---

## 5-WHY chain B — Tokens in the browser URL

| # | Question | Answer |
|---|----------|--------|
| 1 | Why do access/refresh tokens appear in browser history and Referer? | `oidc_callback` appends them to `EDGEQUAKE_OIDC_SUCCESS_REDIRECT_URL` query params. |
| 2 | Why wasn't the HttpOnly cookie path used? | Cookie helpers exist (`refresh_cookie.rs`, SPEC-154 Wave 5) but OIDC never calls `set_refresh_cookie_header`. |
| 3 | Why does that violate the product bar? | LAW-154-9 forbids secrets in URLs/localStorage; SSO must not be weaker than password login. |
| 4 | Why do operators still enable the redirect? | It is the only documented WebUI completion path when `EDGEQUAKE_OIDC_SUCCESS_REDIRECT_URL` is set. |
| 5 | **Root cause** | **OIDC success UX was never upgraded to the SPEC-154 handoff/cookie model.** |

```text
  Password login (SPEC-154)              OIDC callback (today)
  -------------------------              ---------------------
  Set-Cookie: eq_refresh (HttpOnly)      ?access_token=...&refresh_token=...
  JSON omits refresh for SPA             tokens in history / logs / Referer
```

Cross-ref: [LAW-158-4](01-first-principles.md) · [F-158-02](04-findings.md) ·
[LAW-154-9](../154-sec-hardening/01-first-principles.md).

---

## 5-WHY chain C — Every SSO user is the default tenant

| # | Question | Answer |
|---|----------|--------|
| 1 | Why do federated users see the wrong corpus? | `access_token_claims` always stamps `default_identity_scope()`. |
| 2 | Why isn't membership created at login? | OIDC path never calls `sync_default_membership` / JIT membership for the IdP org. |
| 3 | Why can headers not fix it safely? | `strict_tenant_bind` defaults **false**; spoofable `X-Tenant-ID` is still advisory. |
| 4 | Why will multi-tenant SSO amplify this? | Without org→tenant mapping, all customers share one blast radius. |
| 5 | **Root cause** | **Tenant is a JWT default constant, not a claim derived from IdP organization binding.** |

```text
  Keycloak Organization "acme"  -->  should map to tenants.slug=acme
                |
                v  (today)
  Claims.tenant_id = 00000000-...-0001   (always)
  memberships row   = missing
  strict_tenant_bind = off
```

Cross-ref: [LAW-158-5](01-first-principles.md) · [F-158-03](04-findings.md) ·
[F-158-05](04-findings.md).

---

## 5-WHY chain D — Tenant CRUD is an open door once SSO lands

| # | Question | Answer |
|---|----------|--------|
| 1 | Why can any authenticated principal list/delete tenants? | `handlers/workspaces/tenants.rs` CRUD takes no `ApiRequireAdmin` and no membership filter. |
| 2 | Why was that tolerable before? | Single-tenant / open-auth quickstart; admin was "whoever has a token". |
| 3 | Why does JIT make it urgent? | Every federated user becomes an authenticated principal with a JWT. |
| 4 | Why isn't middleware enough? | Auth middleware proves *who*; it does not prove *tenant admin*. |
| 5 | **Root cause** | **Authorization on tenant lifecycle APIs was never bound to membership/RBAC.** |

Cross-ref: [LAW-158-6](01-first-principles.md) · [F-158-04](04-findings.md).

---

## 5-WHY chain E — Enterprise SSO is still "bring your own proxy"

| # | Question | Answer |
|---|----------|--------|
| 1 | Why do docs still push oauth2-proxy? | `EXTERNAL_SSO_PATTERN = "oauth2-proxy"`; FAQ and best-practices predate builtin OIDC. |
| 2 | Why isn't Keycloak the product answer? | No realm, no image, no Organizations mapping, no compose overlay. |
| 3 | Why can't operators use hyperscalers directly today? | One global `EDGEQUAKE_OIDC_*` issuer; GitHub has no OIDC `id_token`; no domain routing. |
| 4 | Why does that lose RFPs? | Enterprises expect SSO + MFA + IdP lifecycle, not a second password store. |
| 5 | **Root cause** | **Product packaging never made Keycloak (Organizations + brokers) a first-class EdgeQuake deliverable.** |

```text
  Today                                      Target (SPEC-158)
  -----                                      -----------------
  oauth2-proxy (external, undocumented map)  edgequake-keycloak image
  one OIDC env                               Organizations + IdP brokers
  WebUI password form only                   SSO buttons + org-first login
```

Cross-ref: [LAW-158-1](01-first-principles.md) · [F-158-10](04-findings.md) ·
[05-keycloak-integration](05-keycloak-integration.md).

---

## Causal stack (what must be true)

```text
  LAW-158-1  Keycloak is the enterprise IdP plane (image + realm)
       |
       +--> LAW-158-2  Single identity store (extends LAW-154-12)
       +--> LAW-158-3  (iss, sub) primary key + trust_email policy   --> closes A
       +--> LAW-158-4  Handoff code + cookie, never tokens in URL    --> closes B
       +--> LAW-158-5  Tenant from organization claim / IdP bind     --> closes C
       +--> LAW-158-6  Tenant APIs require membership/RBAC           --> closes D
       |
  LAW-158-7  Provider registry (Strategy) + cached discovery
  LAW-158-8  JIT membership + role re-sync
  LAW-158-9  SSO mode fail-closed startup
  LAW-158-10 Back-channel logout / deprovision path
  LAW-158-11 Durable OIDC pending state (multi-replica)
  LAW-158-12 Hyperscaler matrix via Keycloak brokers (+ GitHub T2)
       |
  LAW-158-13 Operator docs are the product surface
  LAW-158-14 CI is proof (one gate per EC)
```

## What this pack does **not** claim

- Replacing EdgeQuake as the session AS for the WebUI (Keycloak remains the
  *identity* AS; EdgeQuake remains the *session* AS for browsers).
- Immediate native SAML SP or SCIM in EdgeQuake.
- Weakening SPEC-154 audience / refresh / jti controls.
- Making oauth2-proxy unsupported — it stays as a documented alternative.
