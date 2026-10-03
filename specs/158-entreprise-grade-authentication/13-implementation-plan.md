# 13 — Implementation plan (waves W0–W7)

Parent: [README](README.md) · Edge cases: [12](12-edge-cases.md) ·
Tests: [14](14-e2e-test-matrix.md)

## Definition of Done (global)

- [x] All LAW-158-* satisfied or explicitly residual in security lens
- [x] Every EC-158 has a green named gate
- [x] SPEC-154 e2e suite still green
- [x] `python3 specs/158-entreprise-grade-authentication/scripts/validate-cross-ref.py` OK
- [x] Operator docs shipped ([16](16-documentation-plan.md))
- [x] No tokens in OIDC redirects; no email-primary federation

---

## W0 — Spec pack + fail-first tests (this delivery)

**Scope:** Documentation only + optional failing test stubs.

| Deliverable | Status |
|-------------|--------|
| Spec pack under `specs/158-entreprise-grade-authentication/` | This wave |
| Cross-ref validator | This wave |
| Fail-first tests (may `#[ignore]` until W1): tenant CRUD authz, email-link takeover | Plan for W1 kickoff |

**DoD:** Pack validates; findings cite real symbols.

**Lenses:** All review.

---

## W1 — Safety net (close S0/S1 without full registry)

**Code:**

1. Tenant CRUD AuthZ (`ApiRequireAdmin` / membership) — F-158-04
2. Replace URL tokens with handoff + `eq_refresh` cookie — F-158-02
3. Stop email auto-link without verified∧trust_email; stop fabricated email match — F-158-01 (minimal: refuse link; still single IdP)
4. Force `strict_tenant_bind` + startup gates when OIDC enabled — F-158-05
5. Extract shared `issue_login_tokens` DRY with password path

**Tests:** `e2e_spec158_tenant_crud_authz`, `handoff_no_token_url`, `federation_key` (partial), `startup_sso_gates`, `open_redirect`

**DoD:** S0 findings closed on single-IdP path; SPEC-154 green.

---

## W2 — Schema & durable pending

**Code:**

1. Migration `164_spec158_federation.sql` + manifest/checksums
2. `federated_identities`, `identity_providers`, `oidc_login_attempts`, `auth_handoff_codes`
3. Membership NULLS NOT DISTINCT
4. Switch pending store to PG
5. Refresh rows: `idp_sid` columns

**Tests:** `pending_pg`, `membership_unique`, migration apply on PG16–18

**DoD:** Multi-replica login works against shared DB.

---

## W3 — Provider registry + tenant resolver + JIT

**Code:**

1. `IdentityProvider` + `OidcProvider` + `ProviderRegistry`
2. Env shim seed
3. TenantResolver from organization claim
4. JIT membership + role mapper + max_users + last-owner guard
5. Discovery/JWKS cache
6. Health `sso_providers`

**Tests:** `org_tenant_bind`, `jit_membership`, `role_resync`, `last_owner_guard`, `jwks_rotation`, `state_replay`, `multi_org_picker`

**DoD:** Keycloak (or wiremock org claim) → correct tenant; unknown org denied.

---

## W4 — Keycloak image & packaging

**Code / ops:**

1. `deploy/keycloak/Containerfile` + realm JSON
2. `docker-compose.keycloak.yml` + Helm values
3. GHCR publish workflow
4. `make dev-sso`
5. Pin ≥ 26.8.0

**Tests:** `e2e_spec158_keycloak_smoke` (compose); import-realm caveats in docs

**DoD:** Fresh volume import; `/health` SSO; login via KC test user.

---

## W5 — WebUI SSO + tenant UX

**Code:**

1. SSO buttons + org hint + `/auth/callback`
2. Locale strings en/fr/zh
3. Membership-scoped tenant list in UI
4. Admin IdP read-only panel (minimum)
5. proxy-guards allowlist

**Tests:** Playwright `@spec158` — handoff, no token URL, login a11y

**DoD:** US-158-02 path works against W4 stack.

---

## W6 — Back-channel logout & deprovision

**Code:**

1. `POST /auth/oidc/backchannel-logout`
2. Wire Keycloak client attribute
3. Revoke by sid/sub; audit
4. Tenant suspended checks on refresh

**Tests:** `backchannel_logout`, `tenant_suspended`

**DoD:** LAW-158-10 proven.

---

## W7 — GitHub T2 + docs + release gates

**Code / docs:**

1. Native `GithubOAuthProvider` (optional feature) **or** document Keycloak-only
2. IdP-specific e2e simulations (GitHub wiremock, hd, entra guest)
3. Ship `docs/security/authentication/*`
4. Update FAQ, best-practices, `.env.example`, `AGENTS.md`, `EXTERNAL_SSO_PATTERN`
5. Release checklist entry in `docs/operations/release-and-cd.md`

**Tests:** `github_broker`, `google_hd`, `entra_guest`, doc link check

**DoD:** Product narrative complete; residual risks listed.

---

## Dependency graph

```text
W0 docs
  -> W1 safety
       -> W2 schema
            -> W3 registry/JIT
                 -> W4 Keycloak image
                 -> W5 WebUI
                      -> W6 logout
                           -> W7 docs/GitHub/release
```

W4 can parallelize with late W3 once org claim shape is stable.

## Risk register

| Risk | Mitigation |
|------|------------|
| Breaking clients that scrape tokens from URL | Changelog + migration note in W1 |
| Realm import skip surprises | Reset script + docs EC-158-28 |
| AuthZ breaks open quickstart demos | DEV_MODE exceptions documented |
| Migration on large `memberships` | Concurrent index; expand-contract |

## Effort (indicative)

| Wave | Eng-days (order of magnitude) |
|------|-------------------------------|
| W0 | 2–3 (docs) |
| W1 | 3–5 |
| W2 | 3–4 |
| W3 | 5–8 |
| W4 | 3–5 |
| W5 | 4–6 |
| W6 | 2–3 |
| W7 | 3–5 |

## As built

All waves landed. Verified end to end against **real** Keycloak 26.8.0 (image `edgequake-keycloak:spec158`) + PostgreSQL 16, 17, and 18 + the Next.js UI
(`scripts/keycloak_smoke.py --api ... --deep`, Playwright `e2e/spec158-sso.spec.ts` — 5 passed, including the acme-only tenant switcher), plus Rust e2e suites
`e2e_spec158_{federation,tenancy,session,store_contract,pg_membership}.rs`. SPEC-154's nine binaries passed on a database that had been through `edgequake migrate`.

| Wave | Delivered |
|------|-----------|
| W1 | Migration 164 (`federated_identities`, `login_attempts`, `sso_handoffs`, `federated_sessions`, `oidc_jti`, `identity_providers`, `memberships` NULL-workspace uniqueness) and 165 (`federated_access_jti`); handoff + `eq_refresh` cookie; no token in URL |
| W2 | `LinkPolicy` (verified-email only), `(issuer, sub)` identity, tenant CRUD authorization (`tenant_access.rs`), startup gates (`startup_sso.rs`) |
| W3 | Provider registry, JIT + role map/max role/resync, org resolution, capacity, suspended-tenant denial |
| W4 | `deploy/keycloak` image + realm, `docker-compose.keycloak.yml`, `make dev-sso`, Helm values example, `release-keycloak.yml`, `keycloak_smoke.py` |
| W5 | Login SSO buttons + org hint, `/auth/callback`, org picker, i18n en/fr/zh, read-only admin IdP card, proxy allowlist |
| W6 | `POST /auth/oidc/backchannel-logout` (HS*/JWKS, `jti` replay, `sid`/`sub`), refresh-time `revalidate_scope`, and denylist of every mapped access-token `jti` |
| W7 | `docs/security/authentication/*`, FAQ/best-practices/deployment/release updates, `.env.example`, AGENTS.md env row |

### Deviations from the plan (decisions)

- **`federated_sessions`** (extra table) maps refresh families to `(issuer, sid, sub)` so back-channel logout can revoke by `sid` or `sub`.
- **id_token clock skew is 30 seconds.** The verifier's clock is `now - 30s`, and `iat` more than 30s in the future is rejected. JWKS rotation is still one forced refresh + retry on `kid` miss.
- **Back-channel logout revokes the refresh family and denylists every access `jti` recorded for that family** (callback token, handoff token, and later refresh rotations). A token that was never recorded still lives until its TTL.
- **Last-owner rule:** an IdP claim that would demote the last owner is skipped (logged), never applied.
- **Tenant authorization compat mode:** membership scoping applies when `strict_tenant_bind || sso_active`; platform admins always pass; open quickstart installs keep global reads.
- **Tenants are never auto-created from an org claim** (`org_unknown`); a requested org the token does not assert yields `org_missing`.
- **GitHub = Keycloak broker only** (OAuth2, no id_token); no native adapter shipped.
- **`compat_serve_max = 165`** in the migration manifest (SPEC-150 gate).
- **Realm `sslRequired`** is `${EQ_KC_SSL_REQUIRED:external}`; the dev overlay sets `none` (plain http on a docker network).
- **Keycloak 26 login is two-step** (username, then password); clients/tests must answer both pages.
- **Bug found only against real PostgreSQL:** the PG membership insert ignored `metadata`, so `source="sso"` was never persisted and role resync never ran. Fixed in `edgequake-core` (`MEMBERSHIP_COLUMNS`, JSONB bind) and guarded by `e2e_spec158_pg_membership.rs`.
