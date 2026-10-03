# 15 — Cross-reference matrix

Parent: [README](README.md)

## Law ↔ Finding ↔ EC ↔ Wave ↔ Test ↔ Lens

| Law | Finding | ECs | Wave | Primary test | Primary lens |
|-----|---------|-----|------|--------------|--------------|
| LAW-158-1 | F-158-08, F-158-10 | 28 | W4 | `keycloak_smoke` | Keycloak / Product |
| LAW-158-2 | F-158-11 | 05, 18 | W2–W3 | `jit_membership` | Database / Full stack |
| LAW-158-3 | F-158-01 | 01–04, 25 | W1–W3 | `federation_key` | Security |
| LAW-158-4 | F-158-02 | 10, 29, 30 | W1 | `handoff_no_token_url` | Front / Security |
| LAW-158-5 | F-158-03, F-158-05 | 11–13, 34 | W1–W3 | `org_tenant_bind` | Full stack / Security |
| LAW-158-6 | F-158-04 | 14 | W1 | `tenant_crud_authz` | Full stack / Product |
| LAW-158-7 | F-158-07 | 15–17, 31 | W3 | `jwks_rotation`, `state_replay` | Full stack |
| LAW-158-8 | F-158-03, F-158-12 | 05, 18–20, 32–33 | W2–W3 | `jit_membership`, `role_resync` | Database |
| LAW-158-9 | F-158-05 | 16, 21 | W1 | `startup_sso_gates` | Security / Product |
| LAW-158-10 | F-158-09 | 22–23 | W6 | `backchannel_logout` | Security / Keycloak |
| LAW-158-11 | F-158-06 | 24 | W2 | `pending_pg` | Full stack / Database |
| LAW-158-12 | F-158-08 | 25–27 | W4–W7 | `github_broker`, `google_hd`, `entra_guest` | Keycloak |
| LAW-158-13 | F-158-10 | 28 | W7 | docs + smoke | Product / Keycloak |
| LAW-158-14 | all | all | all | [14-e2e-test-matrix](14-e2e-test-matrix.md) | Full stack |

## WHY chain ↔ Law

| WHY | Laws |
|-----|------|
| A email takeover | LAW-158-3 |
| B tokens in URL | LAW-158-4 |
| C default tenant | LAW-158-5, LAW-158-8 |
| D tenant CRUD | LAW-158-6 |
| E oauth2-proxy-only story | LAW-158-1, LAW-158-12, LAW-158-13 |

## Prior specs

| Prior | Topic | SPEC-158 link |
|-------|-------|---------------|
| [SPEC-027](../027-api-edgequake-audit/) | Builtin OIDC RP | Surfaces; extend not replace |
| [SPEC-154](../154-sec-hardening/) | Session / MCP / cookies | Inheritance table in [01](01-first-principles.md) |
| [SPEC-083](../083-improvements/) | jti / CORS / JWT | Startup gates |
| [SPEC-087](../087-fix-issues/) | Anonymous / guest chat | Unchanged; SSO mode disables guest by default via auth_enabled |
| [SPEC-150](../150-reliable-migration-system/) | Migrations | Migration 164 |
| [SPEC-152 §08](../152-new-mcp-contract/08-security-observability.md) | MCP scopes | EC-158-35/36 |
| [docs/security/best-practices.md](../../docs/security/best-practices.md) | Ops | Update per [16](16-documentation-plan.md) |

## Code symbols

| Symbol | Path | Findings |
|--------|------|----------|
| `resolve_or_create_oidc_user` | `handlers/auth/oidc.rs` | F-158-01 |
| `OidcFlowService::complete_login` | `services/oidc_flow.rs` | F-158-01 |
| `oidc_callback` redirect query | `handlers/auth/oidc.rs` | F-158-02 |
| `access_token_claims` | `services/identity_storage.rs` | F-158-03 |
| `create_tenant` / `list_tenants` / `delete_tenant` | `handlers/workspaces/tenants.rs` | F-158-04 |
| `ApiSecurityConfig::strict_tenant_bind` | `state/security_config.rs` | F-158-05 |
| `store_oidc_pending` | `services/oidc_pending.rs` | F-158-06 |
| `OidcConfig::from_env` | `edgequake-auth/oidc_config.rs` | F-158-07 |
| `EXTERNAL_SSO_PATTERN` | `edgequake-auth/config.rs` | F-158-10 |
| `users` UNIQUE email | `migrations/007_*.sql` | F-158-11 |
| `memberships` UNIQUE | `migrations/008_*.sql` | F-158-12 |
| `CredentialDecision` | `services/auth_validation.rs` | inherit |
| `set_refresh_cookie_header` | `handlers/auth/refresh_cookie.rs` | W1 reuse |
| `useAuthStore` | `edgequake_webui/.../use-auth-store.ts` | W5 |

## Document map

| Doc | Role |
|-----|------|
| [README](README.md) | Entry, locked decisions |
| [00-why](00-why.md) | 5-WHY + causal ASCII |
| [01-first-principles](01-first-principles.md) | LAW-158-* |
| [02-surfaces](02-surfaces.md) | Code map + finding index |
| [03-product-spec](03-product-spec.md) | US-158-* |
| [04-architecture](04-architecture.md) | BFF + modules |
| [04-findings](04-findings.md) | F-158-* |
| [05-keycloak-integration](05-keycloak-integration.md) | Image + realm |
| [06-idp-matrix](06-idp-matrix.md) | Hyperscalers |
| [07-data-and-db-contract](07-data-and-db-contract.md) | Migration 164 |
| [08-api-and-token-contract](08-api-and-token-contract.md) | Routes/claims |
| [09-ux-ui-spec](09-ux-ui-spec.md) | UX |
| [10-frontend-architecture](10-frontend-architecture.md) | WebUI modules |
| [11-security-threat-model](11-security-threat-model.md) | STRIDE |
| [12-edge-cases](12-edge-cases.md) | EC-158-* |
| [13-implementation-plan](13-implementation-plan.md) | Waves |
| [14-e2e-test-matrix](14-e2e-test-matrix.md) | Gates |
| [16-documentation-plan](16-documentation-plan.md) | Operator docs |
| [lenses/](lenses/) | Role lenses |

## Wave status tracker

| Wave | Status | Sign-off lenses |
|------|--------|-----------------|
| 0 Spec pack | **Done (this delivery)** | All |
| 1 Safety net | Planned | Security, Full stack |
| 2 Schema | Planned | Database |
| 3 Registry/JIT | Planned | Full stack, Security |
| 4 Keycloak image | Planned | Keycloak, Product |
| 5 WebUI | Planned | UX, Front |
| 6 Logout | Planned | Security, Keycloak |
| 7 Docs/GitHub/release | Planned | Product, Keycloak |
