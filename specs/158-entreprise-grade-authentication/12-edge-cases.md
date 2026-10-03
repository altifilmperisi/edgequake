# 12 — Edge cases (EC-158)

Parent: [README](README.md) · Findings: [04](04-findings.md) ·
Tests: [14](14-e2e-test-matrix.md)

Each EC has: scenario, expected, mitigation law, primary test id.

---

## Identity & linking

### EC-158-01 — Unverified email tries to link existing user
- **Scenario:** IdP returns email of existing user with `email_verified=false`.
- **Expected:** No link; JIT new user on `(iss,sub)` or deny per policy; victim session untouched. Unverified email that matches nobody still creates a new user (that is the JIT path, not an accident).
- **Law:** LAW-158-3 · F-158-01
- **Test:** `e2e_spec158_federation::unverified_email_never_links_local_account`

### EC-158-02 — Verified email + trust_email links intentionally
- **Scenario:** Entra/Workspace IdP with trust_email; verified email matches existing password user.
- **Expected:** Link federated_identities; single user_id; audit `account_linked`.
- **Law:** LAW-158-3
- **Test:** `e2e_spec158_federation::verified_email_links_only_when_provider_trusted_and_verified`

### EC-158-03 — Fabricated local email must not match
- **Scenario:** IdP omits email; legacy code would invent `oidc-{sub}@edgequake.local`.
- **Expected:** No email field match; store null/empty email or unique non-colliding value outside UNIQUE conflict with real users.
- **Law:** LAW-158-3 · F-158-01
- **Test:** `e2e_spec158_federation::omitted_email_does_not_match_local_account`

### EC-158-04 — Same subject different issuers
- **Scenario:** Two IdPs issue identical `sub` strings.
- **Expected:** Distinct federated rows; no cross-link.
- **Law:** LAW-158-3
- **Test:** `e2e_spec158_federation::federation_key_is_issuer_and_subject`

### EC-158-05 — Username collision on JIT
- **Scenario:** preferred_username already taken.
- **Expected:** Deterministic suffix; login succeeds; no 500.
- **Law:** LAW-158-8 · F-158-11
- **Test:** `e2e_spec158_federation::username_collision_gets_suffix`

---

## Session handoff & cookies

### EC-158-10 — Success redirect must not contain tokens
- **Scenario:** `EDGEQUAKE_OIDC_SUCCESS_REDIRECT_URL` set.
- **Expected:** Only `code=` handoff; `Set-Cookie: eq_refresh`; Playwright URL assertion.
- **Law:** LAW-158-4 · F-158-02
- **Test:** `e2e_spec158_session::handoff_redirect_carries_only_an_opaque_code` + Playwright

### EC-158-29 — Handoff code replay
- **Scenario:** Redeem same code twice.
- **Expected:** Second → 401; first session intact.
- **Law:** LAW-158-4
- **Test:** `e2e_spec158_session::handoff_code_is_single_use_and_garbage_rejected`

### EC-158-30 — Open redirect via redirect param
- **Scenario:** `redirect=//evil.test` or absolute URL.
- **Expected:** Ignored; default `/`.
- **Law:** LAW-158-4
- **Test:** `e2e_spec158_federation::open_redirect_targets_are_neutralised`

---

## Tenant binding

### EC-158-11 — Unknown organization alias
- **Scenario:** Token org `unknown` not in `tenants.slug`.
- **Expected:** 403 `org_unknown`; no default tenant session.
- **Law:** LAW-158-5 · F-158-03
- **Test:** `e2e_spec158_tenancy::unknown_org_is_denied`

### EC-158-12 — Header spoof with SSO session
- **Scenario:** Valid JWT for tenant A; `X-Tenant-ID: B`.
- **Expected:** 403 when strict bind on (SSO forces on).
- **Law:** LAW-158-5 · F-158-05
- **Test:** `e2e_spec158_tenancy::sso_jwt_rejects_foreign_tenant_header`

### EC-158-13 — Multi-organization membership
- **Scenario:** User in org acme + globex; requests `organization:*` or picker.
- **Expected:** Explicit selection; no silent wrong tenant.
- **Law:** LAW-158-5
- **Test:** `e2e_spec158_tenancy::multi_org_requires_explicit_selection` + Playwright

### EC-158-14 — Non-admin tenant delete
- **Scenario:** Member JWT DELETE /tenants/{other}.
- **Expected:** 403; fail-first proves today's gap then fix.
- **Law:** LAW-158-6 · F-158-04
- **Test:** `e2e_spec158_tenancy::tenant_crud_requires_authorization`

---

## Providers & reliability

### EC-158-15 — JWKS key rotation mid-flight
- **Scenario:** IdP rotates signing key between authorize and callback.
- **Expected:** Refresh JWKS cache; verify succeeds or clear error; no panic.
- **Law:** LAW-158-7
- **Test:** `e2e_spec158_session::jwks_rotation_between_authorize_and_callback_is_survived`

### EC-158-16 — Keycloak down; break-glass password
- **Scenario:** SSO start fails; local admin password works.
- **Expected:** Password login 200; SSO button shows degraded error; audit.
- **Law:** LAW-158-9 · US-158-09
- **Test:** `e2e_spec158_federation::unreachable_idp_redirects_and_password_still_works` + Playwright `sso_unavailable`

### EC-158-17 — Clock skew within leeway
- **Scenario:** id_token iat/exp skew ≤ 30s (JwtService leeway).
- **Expected:** Accept; beyond leeway reject.
- **Law:** LAW-158-7
- **Test:** `e2e_spec158_federation::id_token_clock_skew_within_30s_is_accepted` and `id_token_clock_skew_beyond_30s_is_rejected`

### EC-158-24 — Multi-replica pending state
- **Scenario:** begin on replica A, callback on B.
- **Expected:** Success with PG pending; fail closed if memory-only.
- **Law:** LAW-158-11 · F-158-06
- **Test:** `e2e_spec158_session::pending_login_survives_replica_handoff`

### EC-158-31 — state / nonce replay
- **Scenario:** Reuse authorization code or state.
- **Expected:** Reject.
- **Law:** LAW-158-7
- **Test:** `e2e_spec158_federation::state_is_single_use`

---

## JIT & roles

### EC-158-18 — First login creates membership
- **Scenario:** New federated user with org acme.
- **Expected:** users + memberships + claims.tenant_id = acme.
- **Law:** LAW-158-8
- **Test:** `e2e_spec158_federation::jit_provisions_user_and_single_sso_membership`

### EC-158-19 — max_users exceeded
- **Scenario:** Tenant at cap.
- **Expected:** 403 `max_users`; no partial user row.
- **Law:** LAW-158-8
- **Test:** `e2e_spec158_tenancy::tenant_capacity_blocks_new_members`

### EC-158-20 — Duplicate NULL workspace membership
- **Scenario:** Concurrent JIT tenant-wide membership inserts.
- **Expected:** One row (NULLS NOT DISTINCT).
- **Law:** LAW-158-8 · F-158-12
- **Test:** `e2e_spec158_store_contract::migration_164_dedupes_and_enforces_null_workspace_uniqueness` (PostgreSQL 16, 17, 18)

### EC-158-32 — Role downgrade on re-login
- **Scenario:** IdP removes admin group.
- **Expected:** Membership role becomes member/readonly; admin APIs 403.
- **Law:** LAW-158-8
- **Test:** `e2e_spec158_federation::role_downgrade_resyncs_on_next_login`

### EC-158-33 — Last owner lockout
- **Scenario:** Role map would demote sole owner.
- **Expected:** Keep owner **or** explicit deny with ops runbook — product choice: **deny demotion of last owner** with audit.
- **Law:** LAW-158-8
- **Test:** `e2e_spec158_federation::last_owner_is_not_demoted_by_resync`

---

## Logout & lifecycle

### EC-158-22 — Back-channel logout revokes refresh
- **Scenario:** Keycloak POSTs logout_token with sid.
- **Expected:** Refresh reuse fails; access jti denylisted if mapped.
- **Law:** LAW-158-10 · F-158-09
- **Test:** `e2e_spec158_session::backchannel_logout_denylists_handoff_access_token`, `access_jti_denylist_is_durable_across_processes` + `scripts/keycloak_smoke.py --deep`

### EC-158-23 — Logout token replay / missing events
- **Scenario:** Replayed jti or token without events claim.
- **Expected:** 400/401; no revoke side effect on replay.
- **Law:** LAW-158-10
- **Test:** `e2e_spec158_session::backchannel_logout_rejects_replay_and_malformed_tokens`

### EC-158-34 — Tenant suspended
- **Scenario:** `tenants.is_active=false` after prior session.
- **Expected:** Refresh/API 403; SSO re-login denied.
- **Law:** LAW-158-5
- **Test:** `e2e_spec158_tenancy::suspended_tenant_blocks_login_and_refresh`

---

## IdP-specific

### EC-158-25 — GitHub unverified email
- **Scenario:** GitHub profile email unverified; Trust Email off.
- **Expected:** No auto-link to corp account.
- **Law:** LAW-158-12 · F-158-08
- **Test:** `e2e_spec158_federation::github_broker_unverified_email_does_not_link` + `scripts/keycloak_smoke.py --broker`

### EC-158-26 — Entra guest user
- **Scenario:** Guest from foreign tid.
- **Expected:** Deny (`idp_tenant_not_allowed`); no user row.
- **Law:** LAW-158-12
- **Test:** `e2e_spec158_federation::entra_tenant_allow_list`

### EC-158-27 — Google hd mismatch
- **Scenario:** Restriction example.com; token without hd or other hd.
- **Expected:** Deny (`hd_mismatch`).
- **Law:** LAW-158-12
- **Test:** `e2e_spec158_federation::google_hosted_domain_allow_list`

---

## Packaging & docs

### EC-158-21 — SSO without auth_enabled
- **Scenario:** OIDC enabled, auth disabled, non-local DB.
- **Expected:** Startup fatal (combine LAW-154-10 + LAW-158-9).
- **Law:** LAW-158-9 · F-158-05
- **Test:** `e2e_spec158_session::startup_gates_are_fail_closed_for_sso`

### EC-158-28 — Realm import skip on restart
- **Scenario:** Operator edits realm JSON and restarts container.
- **Expected:** Docs warn skip behavior; reset script documented; e2e uses fresh volume.
- **Law:** LAW-158-1 · F-158-10
- **Test:** `scripts/keycloak_smoke.py --realm-restart` against `edgequake-keycloak:spec158`

---

## Cross-surface (SPEC-154 interplay)

### EC-158-35 — SSO user MCP token audience
- **Scenario:** SSO user obtains MCP token via thin AS.
- **Expected:** MCP aud enforced; cannot call REST as MCP token.
- **Law:** LAW-154-3
- **Test:** `e2e_spec158_surfaces::sso_user_mcp_token_is_audience_bound`

### EC-158-36 — SSO user API key scopes
- **Scenario:** Federated user creates API key without write scope.
- **Expected:** Write tools denied (LAW-154-5).
- **Test:** `e2e_spec158_surfaces::sso_user_read_query_api_key_cannot_write`

### EC-158-37 — WebSocket auth after SSO
- **Scenario:** WS upgrade with SSO access JWT; no `?token=`.
- **Expected:** Success via header/subprotocol; membership bind applies.
- **Law:** LAW-154-9 · LAW-158-5
- **Test:** `e2e_spec158_surfaces::sso_access_token_upgrades_websocket_header_only`

### EC-158-38 — Cookie Secure behind reverse proxy
- **Scenario:** `x-forwarded-proto: https`.
- **Expected:** Secure flag on eq_refresh (existing helper).
- **Test:** `e2e_spec158_session::handoff_redirect_carries_only_an_opaque_code`

---

## Count

EC-158-01…05, 10–14, 15–23, 24–28, 29–38 → **32 edge cases** registered for
matrix coverage (gaps in numbering reserved / intentional).
