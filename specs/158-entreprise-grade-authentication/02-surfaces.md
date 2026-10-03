# 02 — Surfaces (code map)

Parent: [README](README.md) · Prev: [01-first-principles](01-first-principles.md) ·
Next: [03-product-spec](03-product-spec.md)

Findings referenced here are expanded in [04-findings](04-findings.md).

## Surface map

```text
  +---------------------------+     +---------------------------+
  | edgequake_webui           |     | edgequake-api             |
  | login page / auth store   |     | handlers/auth/*           |
  | tenant provider / proxy   |     | services/oidc_*           |
  +-------------+-------------+     | middleware + auth_valid.  |
                |                   | oauth/* (MCP thin AS)     |
                |                   +-------------+-------------+
                |                                 |
                v                                 v
  +---------------------------+     +---------------------------+
  | edgequake-auth            |     | PostgreSQL                |
  | jwt / oidc_config / rbac  |     | users, memberships, RLS   |
  | password / types          |     | refresh, jti denylist     |
  +---------------------------+     +---------------------------+
                |
                v  (W4+)
  +---------------------------+
  | edgequake-keycloak image  |
  | realm + Organizations      |
  +---------------------------+
```

## File and symbol inventory

### `edgequake-auth`

| Path | Symbols | Role today | SPEC-158 delta |
|------|---------|------------|----------------|
| [`crates/edgequake-auth/src/lib.rs`](../../edgequake/crates/edgequake-auth/src/lib.rs) | crate root, FEAT0501–0504 | Public API | Export federation types |
| [`…/oidc_config.rs`](../../edgequake/crates/edgequake-auth/src/oidc_config.rs) | `OidcConfig`, `MECHANISM_*` | Single global OIDC env | Shim → registry seed |
| [`…/config.rs`](../../edgequake/crates/edgequake-auth/src/config.rs) | `AuthConfig`, `EXTERNAL_SSO_PATTERN`, `OAUTH2_OIDC_BUILTIN` | Docs still say oauth2-proxy | Update constants/docs [F-158-10](04-findings.md) |
| [`…/jwt.rs`](../../edgequake/crates/edgequake-auth/src/jwt.rs) | `Claims`, `JwtService` | HS256 session JWT; `tenant_id`/`workspace_id` optional | Claims populated from resolver |
| [`…/rbac.rs`](../../edgequake/crates/edgequake-auth/src/rbac.rs) | `Permission`, `RbacService` | Global role permissions | Keep; membership role separate |
| [`…/types.rs`](../../edgequake/crates/edgequake-auth/src/types.rs) | `Role` | admin/user/readonly | Keep platform roles |
| [`…/tenant.rs`](../../edgequake/crates/edgequake-auth/src/tenant.rs) | `TenantContext` (feature `multi-tenant`) | Optional | Align with API TenantContext |

### `edgequake-api` — auth handlers & services

| Path | Symbols | Role today | SPEC-158 delta |
|------|---------|------------|----------------|
| [`handlers/auth/oidc.rs`](../../edgequake/crates/edgequake-api/src/handlers/auth/oidc.rs) | `oidc_login`, `oidc_callback`, `resolve_or_create_oidc_user`, `issue_login_tokens` | Single IdP RP | Split; handoff; federation key [F-158-01](04-findings.md) [F-158-02](04-findings.md) |
| [`services/oidc_flow.rs`](../../edgequake/crates/edgequake-api/src/services/oidc_flow.rs) | `OidcFlowService`, `OidcIdentity`, `begin_login`, `complete_login` | openidconnect 4.x PKCE | Generic provider; verify email; no fabricated match |
| `services/oidc_pending.rs` (removed in W1; see [`federation/store.rs`](../../edgequake/crates/edgequake-api/src/services/federation/store.rs)) | `store_oidc_pending`, `take_oidc_pending` | In-memory | PG `oidc_login_attempts` [F-158-06](04-findings.md) |
| [`handlers/auth/refresh_cookie.rs`](../../edgequake/crates/edgequake-api/src/handlers/auth/refresh_cookie.rs) | `set_refresh_cookie_header`, `REFRESH_COOKIE_NAME` | Password path | Must be used by OIDC |
| [`handlers/auth/session.rs`](../../edgequake/crates/edgequake-api/src/handlers/auth/session.rs) | `login`, `refresh_token`, `logout` | Password session | Share token issuer with OIDC |
| [`services/identity_storage.rs`](../../edgequake/crates/edgequake-api/src/services/identity_storage.rs) | `access_token_claims`, `default_identity_scope`, `verify_membership_active`, `sync_default_membership_to_postgres` | Always default tenant | Claim-aware claims builder [F-158-03](04-findings.md) |
| [`services/auth_validation.rs`](../../edgequake/crates/edgequake-api/src/services/auth_validation.rs) | `CredentialDecision`, `validate_presented_token`, `decide` | SPEC-154 SSOT | Optional Keycloak bearer profile |
| [`state/auth_runtime.rs`](../../edgequake/crates/edgequake-api/src/state/auth_runtime.rs) | `AuthRuntime` | One `OidcFlowService` | `ProviderRegistry` |
| [`state/security_config.rs`](../../edgequake/crates/edgequake-api/src/state/security_config.rs) | `strict_tenant_bind` default false | Opt-in bind | Force on with SSO [F-158-05](04-findings.md) |
| [`middleware.rs`](../../edgequake/crates/edgequake-api/src/middleware.rs) | `protected_api_auth`, `apply_authenticated_context`, `membership_bind_decision`, public OIDC paths | Auth + bind | Keep; add logout + handoff public paths |
| [`routes.rs`](../../edgequake/crates/edgequake-api/src/routes.rs) | `/auth/oidc/*`, `/tenants*` | Route table | New SSO/handoff/logout routes |
| [`handlers/workspaces/tenants.rs`](../../edgequake/crates/edgequake-api/src/handlers/workspaces/tenants.rs) | `create_tenant`, `list_tenants`, `update_tenant`, `delete_tenant` | No admin extractor | AuthZ [F-158-04](04-findings.md) |
| [`handlers/auth/extractors.rs`](../../edgequake/crates/edgequake-api/src/handlers/auth/extractors.rs) | `ApiRequireAdmin` | Exists; unused by tenants | Wire into tenant CRUD |
| [`handlers/health.rs`](../../edgequake/crates/edgequake-api/src/handlers/health.rs) | `auth_mechanisms`, `external_sso_pattern` | Reports builtin OIDC | Report registry providers |
| [`oauth/*`](../../edgequake/crates/edgequake-api/src/oauth/) | MCP thin AS | Separate AS for agents | Unchanged session issuer |

### Multi-tenancy types & DB

| Path | Role |
|------|------|
| [`edgequake-core/.../multitenancy/tenant.rs`](../../edgequake/crates/edgequake-core/src/types/multitenancy/tenant.rs) | `Tenant`, plans, model defaults |
| [`…/membership.rs`](../../edgequake/crates/edgequake-core/src/types/multitenancy/membership.rs) | `Membership`, `MembershipRole` |
| [`migrations/007_add_auth_tables.sql`](../../edgequake/migrations/007_add_auth_tables.sql) | `users`, `api_keys`, `refresh_tokens` — global UNIQUE email/username [F-158-11](04-findings.md) |
| [`migrations/008_add_multi_tenancy_tables.sql`](../../edgequake/migrations/008_add_multi_tenancy_tables.sql) | `tenants`, `workspaces`, `memberships` — NULL workspace uniqueness [F-158-12](04-findings.md) |
| [`migrations/096_rls_fail_closed_force.sql`](../../edgequake/migrations/096_rls_fail_closed_force.sql) | RLS FORCE pattern to copy |
| [`migrations/161_oauth_refresh_grants.sql`](../../edgequake/migrations/161_oauth_refresh_grants.sql) / [`162_spec154_jti_denylist_refresh_family.sql`](../../edgequake/migrations/162_spec154_jti_denylist_refresh_family.sql) | Refresh family + jti |

### WebUI

| Path | Role today | SPEC-158 delta |
|------|------------|----------------|
| [`edgequake_webui/src/app/(auth)/login/page.tsx`](../../edgequake_webui/src/app/%28auth%29/login/page.tsx) | Password + demo skip | SSO buttons, org hint |
| [`…/stores/use-auth-store.ts`](../../edgequake_webui/src/stores/use-auth-store.ts) | Memory access + cookie refresh | Handoff redeem; no URL tokens |
| [`…/providers/tenant-provider.tsx`](../../edgequake_webui/src/providers/tenant-provider.tsx) | Tenant context | Multi-membership picker |
| [`…/stores/use-tenant-store.ts`](../../edgequake_webui/src/stores/use-tenant-store.ts) | Tenant/workspace state | Membership-scoped lists |
| [`…/proxy.ts`](../../edgequake_webui/src/proxy.ts) + [`lib/server/proxy-guards.ts`](../../edgequake_webui/src/lib/server/proxy-guards.ts) | Cookie mirror guard | Allow `/auth/callback` |
| [`…/lib/api/edgequake/auth.ts`](../../edgequake_webui/src/lib/api/edgequake/auth.ts) | login API | SSO start + handoff |

### Packaging & docs (gaps)

| Path | Gap |
|------|-----|
| [`docker-compose.quickstart.yml`](../../docker-compose.quickstart.yml) | No Keycloak service |
| [`edgequake/docker/Dockerfile`](../../edgequake/docker/Dockerfile) | API only |
| [`docs/security/best-practices.md`](../../docs/security/best-practices.md) | oauth2-proxy sample |
| [`docs/faq.md`](../../docs/faq.md) | External SSO = oauth2-proxy |
| [`.env.example`](../../.env.example) | Single OIDC block |

### Existing tests to extend

| Test | Use |
|------|-----|
| [`tests/spec027_oidc_e2e.rs`](../../edgequake/crates/edgequake-api/tests/spec027_oidc_e2e.rs) | wiremock IdP pattern |
| [`tests/spec027_pg_auth_e2e.rs`](../../edgequake/crates/edgequake-api/tests/spec027_pg_auth_e2e.rs) | PG identity |
| [`tests/e2e_spec154_*`](../../edgequake/crates/edgequake-api/tests/) | Session/cookie/bind gates |
| [`tests/e2e_tenant_isolation.rs`](../../edgequake/crates/edgequake-api/tests/e2e_tenant_isolation.rs) | Isolation patterns |
| WebUI `auth-storage.spec.ts`, `multi-tenant-isolation.spec.ts` | Playwright baselines |

## Finding index (quick)

| ID | One-line | Primary surface |
|----|----------|-----------------|
| [F-158-01](04-findings.md) | Email-link federation takeover | `oidc.rs` / `oidc_flow.rs` |
| [F-158-02](04-findings.md) | Tokens in success redirect URL | `oidc_callback` |
| [F-158-03](04-findings.md) | Default-tenant claims for SSO | `access_token_claims` |
| [F-158-04](04-findings.md) | Unguarded tenant CRUD | `tenants.rs` |
| [F-158-05](04-findings.md) | `strict_tenant_bind` default off | `security_config.rs` |
| [F-158-06](04-findings.md) | In-memory OIDC pending | `oidc_pending` / auth_memory |
| [F-158-07](04-findings.md) | Single global IdP only | `OidcConfig` |
| [F-158-08](04-findings.md) | No GitHub / hyperscaler matrix | packaging + providers |
| [F-158-09](04-findings.md) | No back-channel logout | missing route |
| [F-158-10](04-findings.md) | Docs still push oauth2-proxy | FAQ / constants |
| [F-158-11](04-findings.md) | Global UNIQUE email/username | migration 007 |
| [F-158-12](04-findings.md) | NULL workspace membership uniqueness | migration 008 |
