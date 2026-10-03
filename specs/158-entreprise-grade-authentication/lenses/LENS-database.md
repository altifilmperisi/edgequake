# Lens — Database Expert

Parent: [README](../README.md) · Contract: [07](../07-data-and-db-contract.md)

## Invariants

1. `(issuer, subject)` unique — federation primary key.
2. One `users` row per human federated identity; tenancy via `memberships`.
3. RLS FORCE on tenant-scoped tables; login paths documented for service role.
4. Ephemeral tables TTL-purged; no unbounded growth.
5. SPEC-150: manifest + checksums for migration 164.

## Critical fixes

| Issue | Fix |
|-------|-----|
| F-158-12 NULL membership dupes | `UNIQUE NULLS NOT DISTINCT` |
| F-158-11 email UNIQUE | Keep; document; suffix usernames |
| In-memory pending | `oidc_login_attempts` |
| Logout precision | `idp_sid` on refresh |

## Index checklist

- `federated_identities (issuer, subject)` UNIQUE
- `identity_providers (tenant_id, slug)` UNIQUE
- `oidc_login_attempts (expires_at)`
- `auth_handoff_codes (expires_at)`
- `refresh_* (idp_sid)` WHERE NOT NULL

## PG matrix

Run 164 on **PG16, PG17, PG18** (product images). `NULLS NOT DISTINCT` requires
PG15+.

## Data classification

| Column | Class |
|--------|-------|
| `raw_claims` | PII — minimize / redact |
| `client_secret_enc` | Secret — prefer secret_ref |
| `pkce_verifier` | Secret — TTL short |

## Review checklist

- [ ] No plaintext secrets required in DB for env-shim mode
- [ ] Down/expand strategy documented
- [ ] Purge job or SQL retention noted
- [ ] RLS tests for new tables
