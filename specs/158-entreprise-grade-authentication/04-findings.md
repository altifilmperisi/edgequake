# 04 — Findings register (F-158)

Parent: [README](README.md) · Surfaces: [02](02-surfaces.md) · Laws: [01](01-first-principles.md)

Severity: **S0** exploit-now · **S1** high with SSO · **S2** ops/reliability · **S3** docs/debt.

---

## F-158-01 — Account takeover via email federation link

| Field | Value |
|-------|-------|
| Severity | **S0** |
| Law | [LAW-158-3](01-first-principles.md) |
| Symbols | `resolve_or_create_oidc_user`, `find_user_by_login`, `OidcFlowService::complete_login` |
| Files | [`handlers/auth/oidc.rs`](../../edgequake/crates/edgequake-api/src/handlers/auth/oidc.rs) · [`services/oidc_flow.rs`](../../edgequake/crates/edgequake-api/src/services/oidc_flow.rs) |

**Evidence:** Callback resolves users by `identity.email`. `complete_login` never reads `email_verified` and fabricates `oidc-{subject}@edgequake.local` when email is absent. Google OIDC docs: use `sub`, not email. Entra: email mutable. Keycloak: Trust Email + auto-link unsafe for GitHub.

**Mitigation:** `federated_identities UNIQUE(issuer, subject)`; link-by-email only if verified ∧ `trust_email`; never match fabricated emails. Wave W1 fail-first + W2/W3 fix.

**ECs:** EC-158-01…04

---

## F-158-02 — Access/refresh tokens in success redirect URL

| Field | Value |
|-------|-------|
| Severity | **S0** |
| Law | [LAW-158-4](01-first-principles.md) (LAW-154-9) |
| Symbols | `oidc_callback` query_pairs `access_token` / `refresh_token` |
| Files | [`handlers/auth/oidc.rs`](../../edgequake/crates/edgequake-api/src/handlers/auth/oidc.rs) |

**Evidence:** When `EDGEQUAKE_OIDC_SUCCESS_REDIRECT_URL` is set, tokens are appended to the redirect. Cookie helpers in `refresh_cookie.rs` are unused on this path.

**Mitigation:** Single-use handoff code + `Set-Cookie: eq_refresh`. Wave W1.

**ECs:** EC-158-10

---

## F-158-03 — Federated sessions always get default tenant claims

| Field | Value |
|-------|-------|
| Severity | **S1** |
| Law | [LAW-158-5](01-first-principles.md) · [LAW-158-8](01-first-principles.md) |
| Symbols | `access_token_claims`, `default_identity_scope` |
| Files | [`services/identity_storage.rs`](../../edgequake/crates/edgequake-api/src/services/identity_storage.rs) |

**Evidence:** All logins stamp default tenant/workspace UUIDs. OIDC path does not create org-scoped memberships.

**Mitigation:** TenantResolver from organization claim / IdP bind; JIT membership; claims from resolved scope. Waves W2–W3.

**ECs:** EC-158-11…13, EC-158-18

---

## F-158-04 — Tenant CRUD lacks membership/admin authorization

| Field | Value |
|-------|-------|
| Severity | **S1** (S0 once SSO JIT is live) |
| Law | [LAW-158-6](01-first-principles.md) |
| Symbols | `create_tenant`, `list_tenants`, `update_tenant`, `delete_tenant` |
| Files | [`handlers/workspaces/tenants.rs`](../../edgequake/crates/edgequake-api/src/handlers/workspaces/tenants.rs) · [`routes.rs`](../../edgequake/crates/edgequake-api/src/routes.rs) |

**Evidence:** Handlers take `State` + JSON/Path only — no `ApiRequireAdmin` / membership check. `ApiRequireAdmin` exists in `handlers/auth/extractors.rs` but is unused here.

**Mitigation:** Fail-first e2e (W0); wire AuthZ (W1). Platform admin vs tenant-scoped list.

**ECs:** EC-158-14

---

## F-158-05 — `strict_tenant_bind` defaults false

| Field | Value |
|-------|-------|
| Severity | **S1** |
| Law | [LAW-158-5](01-first-principles.md) · [LAW-158-9](01-first-principles.md) |
| Symbols | `ApiSecurityConfig::strict_tenant_bind` |
| Files | [`state/security_config.rs`](../../edgequake/crates/edgequake-api/src/state/security_config.rs) |

**Evidence:** `parse_bool_env("EDGEQUAKE_STRICT_TENANT_BIND", false)`. Membership bind skipped unless operators opt in.

**Mitigation:** When any SSO provider is runtime-active, force bind on (unless `EDGEQUAKE_DEV_MODE`). Startup fatal if SSO ∧ !auth. Wave W1.

**ECs:** EC-158-12, EC-158-21

---

## F-158-06 — OIDC pending state is process-local

| Field | Value |
|-------|-------|
| Severity | **S2** |
| Law | [LAW-158-11](01-first-principles.md) |
| Symbols | `store_oidc_pending`, `take_oidc_pending`, auth_memory store |
| Files | `services/oidc_pending.rs` (removed in W1; now [`services/federation/pg_store.rs`](../../edgequake/crates/edgequake-api/src/services/federation/pg_store.rs)) |

**Evidence:** Comment: "in-memory, not KV". Multi-replica login: start on A, callback on B → state_expired.

**Mitigation:** `oidc_login_attempts` PG table with TTL. Wave W2.

**ECs:** EC-158-24

---

## F-158-07 — Single global OIDC provider only

| Field | Value |
|-------|-------|
| Severity | **S2** |
| Law | [LAW-158-7](01-first-principles.md) |
| Symbols | `OidcConfig::from_env`, `AuthRuntime::new` |
| Files | [`oidc_config.rs`](../../edgequake/crates/edgequake-auth/src/oidc_config.rs) · [`auth_runtime.rs`](../../edgequake/crates/edgequake-api/src/state/auth_runtime.rs) |

**Evidence:** One issuer/client/redirect. No per-tenant IdP, no domain routing, discovery re-fetched every begin/complete with no JWKS cache.

**Mitigation:** Provider registry + cache. Env becomes shim. Wave W3.

**ECs:** EC-158-15…17

---

## F-158-08 — No hyperscaler / GitHub product path

| Field | Value |
|-------|-------|
| Severity | **S2** |
| Law | [LAW-158-1](01-first-principles.md) · [LAW-158-12](01-first-principles.md) |
| Symbols | n/a (missing packaging) |
| Files | compose / deploy / WebUI login |

**Evidence:** GitHub OAuth has no `id_token` — current `CoreClient` path cannot serve it. No Keycloak image/realm. No SSO buttons in WebUI.

**Mitigation:** `edgequake-keycloak` + brokers; WebUI SSO; native GitHub T2. Waves W4–W5, W7.

**ECs:** EC-158-25…27

---

## F-158-09 — No OIDC back-channel logout

| Field | Value |
|-------|-------|
| Severity | **S1** |
| Law | [LAW-158-10](01-first-principles.md) |
| Symbols | missing handler |
| Files | routes / Keycloak client config |

**Evidence:** Deprovisioned IdP users keep refresh tokens up to 30 days. No `backchannel.logout.url` consumer.

**Mitigation:** Logout token endpoint + `idp_sid` on refresh rows. Wave W6.

**ECs:** EC-158-22…23

---

## F-158-10 — Documentation still positions oauth2-proxy as the enterprise answer

| Field | Value |
|-------|-------|
| Severity | **S3** |
| Law | [LAW-158-13](01-first-principles.md) |
| Symbols | `EXTERNAL_SSO_PATTERN`, `OAUTH2_OIDC_BUILTIN` |
| Files | [`config.rs`](../../edgequake/crates/edgequake-auth/src/config.rs) · [`docs/faq.md`](../../docs/faq.md) · [`docs/security/best-practices.md`](../../docs/security/best-practices.md) |

**Evidence:** FAQ: "recommended over in-process OIDC for enterprise". Constant still names oauth2-proxy.

**Mitigation:** New auth doc set; update FAQ/best-practices; keep oauth2-proxy as alternative. Wave W7 (+ W0 outline in [16](16-documentation-plan.md)).

**ECs:** EC-158-28

---

## F-158-11 — Global UNIQUE on `users.email` / `username`

| Field | Value |
|-------|-------|
| Severity | **S2** |
| Law | [LAW-158-2](01-first-principles.md) · [LAW-158-8](01-first-principles.md) |
| Files | [`migrations/007_add_auth_tables.sql`](../../edgequake/migrations/007_add_auth_tables.sql) |

**Evidence:** Same person cannot exist as distinct principals across tenants if email collides. Multi-tenant B2B often needs shared emails with separate memberships (one user row + many memberships is OK) — collision on JIT username also breaks.

**Mitigation:** Keep one user row per `(iss,sub)`; username uniqueness strategy: suffix on conflict; document email as global contact identity. Optional later: drop global unique only with migration plan. Wave W2 decision record.

**ECs:** EC-158-05, EC-158-19

---

## F-158-12 — Membership UNIQUE allows duplicate NULL workspace rows

| Field | Value |
|-------|-------|
| Severity | **S2** |
| Law | [LAW-158-8](01-first-principles.md) |
| Files | [`migrations/008_add_multi_tenancy_tables.sql`](../../edgequake/migrations/008_add_multi_tenancy_tables.sql) |

**Evidence:** `UNIQUE(user_id, tenant_id, workspace_id)` — in PostgreSQL, NULL ≠ NULL, so multiple tenant-wide memberships can insert.

**Mitigation:** `UNIQUE NULLS NOT DISTINCT` (PG15+) or partial unique index for `workspace_id IS NULL`. Verify PG16–18 matrix. Wave W2.

**ECs:** EC-158-20

---

## Wave mapping

| Finding | Primary wave |
|---------|--------------|
| F-158-01, F-158-02, F-158-04, F-158-05 | W0–W1 |
| F-158-03, F-158-06, F-158-11, F-158-12 | W2–W3 |
| F-158-07, F-158-08 | W3–W5 |
| F-158-09 | W6 |
| F-158-10 | W7 |
