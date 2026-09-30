# Lens — Database Expert

Parent: [README](../README.md) · Laws: LAW-154-7, LAW-154-8 · Plan Wave 4: [06](../06-implementation-plan.md)

## Current persistence (auth-related)

| Store | Table / KV | Notes |
|-------|------------|-------|
| Users + lockout | `users` (+ support 048) | `failed_login_attempts`, `locked_until` |
| Web refresh | `refresh_tokens` | `token_hash`, revoked flag |
| Stored API keys | session/identity stores | Argon2 `key_hash`, prefix lookup |
| MCP refresh | `oauth_refresh_grants` | family_id, status, token_hash |
| OAuth clients / codes | oauth_* tables | DCR/CIMD registrations |
| jti denylist | **in-process HashSet only** | Not durable — F-154-05 |

## Required schema direction (Wave 4)

```text
  jwt_jti_denylist
  ----------------
  jti          TEXT PK
  expires_at   TIMESTAMPTZ NOT NULL  -- = access token exp
  revoked_at   TIMESTAMPTZ NOT NULL
  reason       TEXT  -- logout | admin | reuse

  Index: expires_at for TTL cleanup job
```

Web refresh should share **family semantics** with MCP:

```text
  refresh present
       |
       v
  hash lookup --> row status
       |
       +-- active --> rotate (new hash), old revoked
       +-- revoked --> revoke_family(family_id)
```

Prefer one `RefreshFamilyStore` port over duplicate SQL in
`session_storage` and `oauth/store.rs` (DRY / LAW-154-7).

## API key scopes (Wave 3)

Ensure `scopes` column (TEXT[] or JSONB) is the SSOT for least privilege.
Migration: backfill missing scopes to `{edgequake:read, edgequake:query}` —
**not** admin, **not** write.

## Isolation

- Membership bind queries must run under existing tenant isolation helpers
  (`verify_membership_active`).
- RLS / `PgIsolationScope` unchanged; auth tables remain identity SSOT
  (SPEC-027), not workspace documents.

## Operational concerns

| Concern | Mitigation |
|---------|------------|
| Denylist growth | TTL = token exp; periodic DELETE WHERE expires_at < now() |
| Hash collision | SHA-256 of high-entropy tokens; accept |
| Replica lag | Prefer read-your-writes on revoke path (primary) |
| Migration safety | Additive tables first; dual-read HashSet+PG then drop memory SSOT |

## Tests

- Contract: insert jti, verify reject across new JwtService instance
- Contract: refresh reuse revokes siblings in family
- Migration checksum script remains green
