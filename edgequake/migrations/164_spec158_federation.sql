-- ============================================================================
-- Migration 164: SPEC-158 — enterprise federation (SSO) auth plane
-- ============================================================================
-- * identity_providers      multi-provider registry (LAW-158-7)
-- * federated_identities    (issuer, subject) -> user (LAW-158-3)
-- * federated_sessions      refresh family -> tenant scope + IdP sid (LAW-158-5/10)
-- * oidc_login_attempts     durable pending state, multi-replica safe (LAW-158-11)
-- * auth_handoff_codes      single-use SPA handoff, no tokens in URLs (LAW-158-4)
-- * oidc_logout_jti         back-channel logout replay guard (LAW-158-10)
-- * memberships             NULLS NOT DISTINCT uniqueness (F-158-12; PostgreSQL 15+)
--
-- Auth-plane tables are service-scoped (no tenant RLS): they are read during login,
-- before a tenant context exists — same posture as refresh_tokens / oauth_refresh_grants.

SET search_path = public;

CREATE TABLE IF NOT EXISTS identity_providers (
    provider_id        UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug               TEXT NOT NULL UNIQUE,
    kind               TEXT NOT NULL DEFAULT 'generic',
    display_name       TEXT NOT NULL DEFAULT 'Single sign-on',
    issuer             TEXT NOT NULL,
    client_id          TEXT NOT NULL,
    -- Name of the environment variable holding the secret; never the secret itself.
    client_secret_ref  TEXT,
    redirect_uri       TEXT NOT NULL,
    scopes             TEXT[] NOT NULL DEFAULT '{}',
    trust_email        BOOLEAN NOT NULL DEFAULT FALSE,
    link_policy        TEXT NOT NULL DEFAULT 'never'
                       CHECK (link_policy IN ('never', 'verified_email')),
    jit_enabled        BOOLEAN NOT NULL DEFAULT TRUE,
    role_claim         TEXT NOT NULL DEFAULT '',
    role_map           JSONB NOT NULL DEFAULT '{}'::jsonb,
    max_role           TEXT NOT NULL DEFAULT 'admin'
                       CHECK (max_role IN ('readonly', 'member', 'admin', 'owner')),
    enabled            BOOLEAN NOT NULL DEFAULT TRUE,
    -- require_org, tenant_slug, allowed_hd, allowed_tid, success_redirect_url, ...
    metadata           JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_identity_providers_issuer ON identity_providers (issuer);

CREATE TABLE IF NOT EXISTS federated_identities (
    federated_id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id                 UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    provider_slug           TEXT NOT NULL,
    issuer                  TEXT NOT NULL,
    subject                 TEXT NOT NULL,
    email_at_link           TEXT,
    email_verified_at_link  BOOLEAN NOT NULL DEFAULT FALSE,
    linked_at               TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_login_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT federated_identities_issuer_subject_key UNIQUE (issuer, subject)
);

CREATE INDEX IF NOT EXISTS idx_federated_identities_user ON federated_identities (user_id);

CREATE TABLE IF NOT EXISTS federated_sessions (
    family_id     UUID PRIMARY KEY,
    user_id       UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    provider_slug TEXT NOT NULL,
    issuer        TEXT NOT NULL,
    subject       TEXT NOT NULL,
    idp_sid       TEXT,
    tenant_id     UUID NOT NULL,
    workspace_id  UUID,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    revoked_at    TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_federated_sessions_sid ON federated_sessions (issuer, idp_sid)
    WHERE idp_sid IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_federated_sessions_subject ON federated_sessions (issuer, subject);
CREATE INDEX IF NOT EXISTS idx_federated_sessions_user ON federated_sessions (user_id);

CREATE TABLE IF NOT EXISTS oidc_login_attempts (
    state             TEXT PRIMARY KEY,
    provider_slug     TEXT NOT NULL,
    pkce_verifier     TEXT NOT NULL,
    nonce             TEXT NOT NULL,
    organization_hint TEXT,
    redirect_after    TEXT,
    expires_at        TIMESTAMPTZ NOT NULL,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_oidc_login_attempts_expires ON oidc_login_attempts (expires_at);

CREATE TABLE IF NOT EXISTS auth_handoff_codes (
    code_hash     TEXT PRIMARY KEY,
    user_id       UUID NOT NULL,
    family_id     UUID NOT NULL,
    provider_slug TEXT NOT NULL,
    tenant_id     UUID NOT NULL,
    workspace_id  UUID,
    redirect_after TEXT,
    expires_at    TIMESTAMPTZ NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_auth_handoff_codes_expires ON auth_handoff_codes (expires_at);

CREATE TABLE IF NOT EXISTS oidc_logout_jti (
    issuer     TEXT NOT NULL,
    jti        TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (issuer, jti)
);

CREATE INDEX IF NOT EXISTS idx_oidc_logout_jti_expires ON oidc_logout_jti (expires_at);

-- F-158-12: UNIQUE(user_id, tenant_id, workspace_id) treats NULL workspaces as distinct, so
-- concurrent tenant-wide JIT inserts could duplicate. Collapse duplicates, then tighten.
DELETE FROM memberships a
USING memberships b
WHERE a.workspace_id IS NULL
  AND b.workspace_id IS NULL
  AND a.user_id = b.user_id
  AND a.tenant_id = b.tenant_id
  AND (a.joined_at, a.membership_id) > (b.joined_at, b.membership_id);

-- 001 names it `memberships_unique`; 008 (when it created the table first) auto-named it.
ALTER TABLE memberships DROP CONSTRAINT IF EXISTS memberships_unique;
ALTER TABLE memberships
    DROP CONSTRAINT IF EXISTS memberships_user_id_tenant_id_workspace_id_key;

DO $$
BEGIN
    IF NOT EXISTS (
        -- Scope to this table: pg_constraint is database-wide, so a same-named
        -- constraint in another schema must not suppress the ADD here.
        SELECT 1 FROM pg_constraint
        WHERE conname = 'memberships_user_tenant_workspace_uidx'
          AND conrelid = 'memberships'::regclass
    ) THEN
        ALTER TABLE memberships
            ADD CONSTRAINT memberships_user_tenant_workspace_uidx
            UNIQUE NULLS NOT DISTINCT (user_id, tenant_id, workspace_id);
    END IF;
END $$;
