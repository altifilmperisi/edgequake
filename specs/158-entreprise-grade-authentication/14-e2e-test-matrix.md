# 14 — E2E test matrix

Parent: [README](README.md) · Edge cases: [12](12-edge-cases.md) ·
Plan: [13](13-implementation-plan.md)

One automated gate per EC. Prefer Rust integration tests with wiremock /
Testcontainers Keycloak; Playwright for WebUI.

## Commands (verified)

The gates are five Rust binaries plus the live Keycloak smoke and one Playwright file.
`--features postgres` is required. A fresh database that creates users must have been
through `edgequake migrate` (`EDGEQUAKE_MIGRATE_CLI=1`) so support SQL 048 adds the
lockout columns the write path uses.

```bash
cd edgequake
cargo test -p edgequake-api --features postgres \
  --test e2e_spec158_federation \
  --test e2e_spec158_tenancy \
  --test e2e_spec158_session \
  --test e2e_spec158_store_contract \
  --test e2e_spec158_pg_membership \
  --test e2e_spec158_surfaces

# NULL-workspace uniqueness on PostgreSQL 16 and 17 (18 is the dev server):
EDGEQUAKE_SPEC158_PG_URL=postgres://edgequake:edgequake_secret@127.0.0.1:5433/eq158 \
  cargo test -p edgequake-api --features postgres \
  --test e2e_spec158_store_contract -- migration_164_dedupes

# SPEC-154 retained. `e2e_spec154_jti_durable` and `e2e_spec154_web_refresh_rotation`
# set `EDGEQUAKE_MIGRATE_CLI=1` themselves before `run_postgres_migrations`.
cargo test -p edgequake-api --features postgres \
  --test e2e_spec154_audience_parity \
  --test e2e_spec154_api_key_scopes \
  --test e2e_spec154_ws_no_query_token \
  --test e2e_spec154_web_refresh_rotation \
  --test e2e_spec154_jti_durable \
  --test e2e_spec154_scope_predicate \
  --test e2e_spec154_membership_bind_mcp \
  --test e2e_spec154_anonymous_vs_mcp \
  --test e2e_spec154_startup_auth_off_fatal

# Live Keycloak (isolated proof stack — does not bind :8080/:8081):
make spec158-proof-e2e
# equivalent:
python3 scripts/keycloak_smoke.py --kc http://localhost:18081 \
  --api http://localhost:18080 --web-callback http://localhost:13010/auth/callback \
  --redirect-uri http://localhost:13010/api/v1/auth/oidc/callback \
  --admin-password "$EDGEQUAKE_BOOTSTRAP_ADMIN_PASSWORD" --deep --broker --realm-restart

# WebUI against that stack:
cd edgequake_webui
E2E_SSO=1 PLAYWRIGHT_BASE_URL=http://127.0.0.1:13010 PLAYWRIGHT_SKIP_STACK_CHECK=1 \
  bunx playwright test e2e/spec158-sso.spec.ts --project=chromium
```

## Matrix

| EC | Wave | Primary gate | Type |
|----|------|--------------|------|
| EC-158-01 | W1 | `e2e_spec158_federation::unverified_email_never_links_local_account` | API |
| EC-158-02 | W3 | `e2e_spec158_federation::verified_email_links_only_when_provider_trusted_and_verified` | API |
| EC-158-03 | W1 | `e2e_spec158_federation::omitted_email_does_not_match_local_account` | API |
| EC-158-04 | W3 | `e2e_spec158_federation::federation_key_is_issuer_and_subject` | API |
| EC-158-05 | W3 | `e2e_spec158_federation::username_collision_gets_suffix` | API |
| EC-158-10 | W1 | `e2e_spec158_session::handoff_redirect_carries_only_an_opaque_code` | API + PW |
| EC-158-11 | W3 | `e2e_spec158_tenancy::unknown_org_is_denied` | API |
| EC-158-12 | W1/W3 | `e2e_spec158_tenancy::sso_jwt_rejects_foreign_tenant_header` | API |
| EC-158-13 | W3/W5 | `e2e_spec158_tenancy::multi_org_requires_explicit_selection` | API + PW |
| EC-158-14 | W0/W1 | `e2e_spec158_tenancy::tenant_crud_requires_authorization` | API |
| EC-158-15 | W3 | `e2e_spec158_session::jwks_rotation_between_authorize_and_callback_is_survived` | API |
| EC-158-16 | W1 | `e2e_spec158_federation::unreachable_idp_redirects_and_password_still_works` | API + PW |
| EC-158-17 | W3 | `e2e_spec158_federation::id_token_clock_skew_within_30s_is_accepted` | API |
| EC-158-18 | W3 | `e2e_spec158_federation::jit_provisions_user_and_single_sso_membership` | API |
| EC-158-19 | W3 | `e2e_spec158_tenancy::tenant_capacity_blocks_new_members` | API |
| EC-158-20 | W2 | `e2e_spec158_store_contract::migration_164_dedupes_and_enforces_null_workspace_uniqueness` | API PG16–18 |
| EC-158-21 | W1 | `e2e_spec158_session::startup_gates_are_fail_closed_for_sso` | API |
| EC-158-22 | W6 | `e2e_spec158_session::access_jti_denylist_is_durable_across_processes` | API + smoke `--deep` |
| EC-158-23 | W6 | `e2e_spec158_session::backchannel_logout_rejects_replay_and_malformed_tokens` | API |
| EC-158-24 | W2 | `e2e_spec158_session::pending_login_survives_replica_handoff` | API |
| EC-158-25 | W7 | `e2e_spec158_federation::github_broker_unverified_email_does_not_link` + smoke `--broker` | API + KC |
| EC-158-26 | W7 | `e2e_spec158_federation::entra_tenant_allow_list` | API |
| EC-158-27 | W7 | `e2e_spec158_federation::google_hosted_domain_allow_list` | API |
| EC-158-28 | W4 | `scripts/keycloak_smoke.py --realm-restart` | image |
| EC-158-29 | W1 | `e2e_spec158_session::handoff_code_is_single_use_and_garbage_rejected` | API |
| EC-158-30 | W1 | `e2e_spec158_federation::open_redirect_targets_are_neutralised` | API |
| EC-158-31 | W3 | `e2e_spec158_federation::state_is_single_use` | API |
| EC-158-32 | W3 | `e2e_spec158_federation::role_downgrade_resyncs_on_next_login` | API |
| EC-158-33 | W3 | `e2e_spec158_federation::last_owner_is_not_demoted_by_resync` | API |
| EC-158-34 | W6 | `e2e_spec158_tenancy::suspended_tenant_blocks_login_and_refresh` | API |
| EC-158-35 | W3 | `e2e_spec158_surfaces::sso_user_mcp_token_is_audience_bound` | API |
| EC-158-36 | W3 | `e2e_spec158_surfaces::sso_user_read_query_api_key_cannot_write` | API |
| EC-158-37 | W5 | `e2e_spec158_surfaces::sso_access_token_upgrades_websocket_header_only` | API |
| EC-158-38 | W1 | `e2e_spec158_session::handoff_redirect_carries_only_an_opaque_code` | API |

## Playwright `@spec158` (minimum)

| Spec file | Covers |
|-----------|--------|
| `e2e/spec158-sso.spec.ts` | SSO button, alice→acme with no globex in the switcher, carol org picker, handoff `code` only, opaque errors, `sso_unavailable` |

Use mock-api project where possible; one live Keycloak job in CI nightly.

## Fixtures

| Fixture | Purpose |
|---------|---------|
| wiremock OIDC (extend spec027) | Claims matrix |
| Keycloak testcontainer / compose | Org claim + brokers |
| Second KC realm | Simulated Google/Entra |
| wiremock GitHub | user + emails |

## Status

The gates above are implemented and green. They are not `#[ignore]`.
Playwright callback-error tests run without Keycloak. Live login stays opt-in (`E2E_SSO=1`).
`make spec158-proof-e2e` proves Keycloak PKCE + API handoff + **back-channel logout (`--deep`)** on
ports 18080/18081/13010 without touching `:8080`. `--broker` still stops at Keycloak's first-broker
login when Organizations scopes are on the client; EC-158-25's named Rust test remains the link-policy gate.
