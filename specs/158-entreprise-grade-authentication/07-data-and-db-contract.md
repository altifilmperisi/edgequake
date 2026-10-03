# 07 — Data and DB contract

Parent: [README](README.md) · Architecture: [04](04-architecture.md) ·
Findings: [04-findings](04-findings.md)

Migration name: **`164_spec158_federation.sql`** (next after `163_spec155_message_feedback.sql`).
Must register in [`migrations/manifest.toml`](../../edgequake/migrations/manifest.toml)
and [`checksums.lock`](../../edgequake/migrations/checksums.lock) per SPEC-150.

## Goals

1. Persist `(issuer, subject) → user_id` (LAW-158-3).
2. Persist multi-provider config (LAW-158-7).
3. Durable OIDC pending + handoff (LAW-158-4, LAW-158-11).
4. Bind refresh sessions to IdP `sid` for back-channel logout (LAW-158-10).
5. Fix membership NULL uniqueness (F-158-12).
6. Keep RLS fail-closed patterns from migration 096.

## ER diagram

```text
  identity_providers 1---* federated_identities *---1 users
           |                                            |
           |                                            *---* memberships *---1 tenants
           |                                            |
           +---- optional tenant_id                     +--- refresh_tokens (+ idp_sid)
                                                        +--- api_keys

  oidc_login_attempts (ephemeral, TTL)
  auth_handoff_codes  (ephemeral, single-use)
```

## Table: `identity_providers`

| Column | Type | Notes |
|--------|------|-------|
| `provider_id` | UUID PK | |
| `tenant_id` | UUID NULL | NULL = global (e.g. Keycloak fleet) |
| `slug` | TEXT | unique per tenant_id |
| `kind` | TEXT | `oidc`, `github_oauth`, `keycloak` |
| `issuer` | TEXT | OIDC issuer URL |
| `client_id` | TEXT | |
| `client_secret_ref` | TEXT | env/secret name — **not** plaintext secret in DB preferred |
| `client_secret_enc` | TEXT NULL | optional sealed secret for UI-managed |
| `redirect_uri` | TEXT | |
| `scopes` | TEXT[] | default `{openid,email,profile,organization}` |
| `trust_email` | BOOLEAN | default FALSE |
| `link_policy` | TEXT | `never` \| `verified_email` \| `always` (always forbidden in prod gates) |
| `jit_enabled` | BOOLEAN | default TRUE |
| `role_claim` | TEXT | e.g. `edgequake_role` |
| `role_map` | JSONB | claim value → membership role |
| `max_role` | TEXT | cap |
| `enabled` | BOOLEAN | |
| `metadata` | JSONB | discovery cache hints, hd allow-list |
| `created_at` / `updated_at` | TIMESTAMPTZ | |

Indexes: `(tenant_id, slug)` UNIQUE; `(issuer)` for lookup.

## Table: `federated_identities`

| Column | Type | Notes |
|--------|------|-------|
| `federated_id` | UUID PK | |
| `user_id` | UUID FK users ON DELETE CASCADE | |
| `provider_id` | UUID FK identity_providers | |
| `issuer` | TEXT | denormalized for UNIQUE |
| `subject` | TEXT | IdP `sub` |
| `email_at_link` | TEXT NULL | audit only |
| `email_verified_at_link` | BOOLEAN | |
| `raw_claims` | JSONB | last snapshot (PII-minimized policy) |
| `linked_at` / `last_login_at` | TIMESTAMPTZ | |

Constraint: **`UNIQUE (issuer, subject)`**.

## Table: `oidc_login_attempts`

| Column | Type | Notes |
|--------|------|-------|
| `state` | TEXT PK | CSRF |
| `provider_id` | UUID | |
| `pkce_verifier` | TEXT | |
| `nonce` | TEXT | |
| `organization_hint` | TEXT NULL | |
| `redirect_after` | TEXT NULL | validated relative path only |
| `expires_at` | TIMESTAMPTZ | ~10 minutes |
| `created_at` | TIMESTAMPTZ | |

Index: `expires_at` for purge job.

## Table: `auth_handoff_codes`

| Column | Type | Notes |
|--------|------|-------|
| `code_hash` | TEXT PK | SHA-256 of one-time code |
| `user_id` | UUID | |
| `access_token_jti` | TEXT | optional pre-mint tracking |
| `expires_at` | TIMESTAMPTZ | ~60–120 seconds |
| `consumed_at` | TIMESTAMPTZ NULL | |

## Alter: refresh tokens / grants

Add nullable:

- `idp_provider_id UUID`
- `idp_sid TEXT` — Keycloak session id from id_token `sid` if present

Index `(idp_sid)` for logout.

(Align with existing `refresh_tokens` / oauth grant tables from migrations 007 / 161 / 162 — implementer picks the SSOT refresh table used by web sessions.)

## Membership uniqueness fix (F-158-12)

```sql
-- Prefer (PG 15+; EdgeQuake supports PG16–18):
ALTER TABLE memberships
  DROP CONSTRAINT IF EXISTS memberships_user_id_tenant_id_workspace_id_key;

ALTER TABLE memberships
  ADD CONSTRAINT memberships_user_tenant_workspace_uidx
  UNIQUE NULLS NOT DISTINCT (user_id, tenant_id, workspace_id);
```

Verify on PG16, PG17, PG18 in CI matrix.

## Email / username uniqueness (F-158-11)

**Decision (locked for W2):** Keep global `users.email` UNIQUE and global
`users.username` UNIQUE. One human federated identity = one user row; multi-tenant
access = multiple `memberships`. On username collision at JIT, suffix
`{base}_{short_sub}`. Do **not** drop UNIQUE in 164 without a separate migration
spec — documents the tradeoff for B2B shared mailboxes (use distinct IdP subjects).

## RLS

New tables with `tenant_id`:

- ENABLE + FORCE ROW LEVEL SECURITY
- Policies using `current_setting('app.tenant_id', true)` like migration 096
- `identity_providers` global rows (`tenant_id IS NULL`): readable by service role
  / platform admin only
- `federated_identities`: via user_id join or service scope during login (login
  path uses elevated connection with audit — document break-glass)

Ephemeral tables (`oidc_login_attempts`, `auth_handoff_codes`): no tenant RLS;
access only via service role; TTL purge.

## Seed from env (compat)

On boot, if `EDGEQUAKE_OIDC_ENABLED` and no row exists for issuer:

```text
INSERT identity_providers (slug=env-oidc, kind=oidc, ...)
```

Idempotent upsert by issuer.

## Purge jobs

| Job | Interval | Action |
|-----|----------|--------|
| pending TTL | 1m | DELETE expired attempts |
| handoff TTL | 1m | DELETE expired/consumed |
| claims snapshot | optional | redact raw_claims older than N days |

## Rollback

164 must be forward-only friendly: new tables DROP IF EXISTS in down script if
SPEC-150 supports downs; otherwise document expand-contract. No destructive
change to `users` PK.
