-- ============================================================================
-- Migration 161: MCP OAuth AS refresh grants (SPEC-028 / MCP Authorization 2026-07-28)
-- ============================================================================
-- Hashed, durable refresh tokens with rotation family tracking.
-- Reuse of a rotated token revokes the entire family (RFC 9700 §4.14).

SET search_path = public;

CREATE TABLE IF NOT EXISTS oauth_refresh_grants (
    token_hash   TEXT PRIMARY KEY,
    family_id    UUID NOT NULL,
    client_id    TEXT NOT NULL,
    resource     TEXT NOT NULL,
    scope        TEXT NOT NULL,
    user_id      TEXT NOT NULL,
    role         TEXT NOT NULL,
    tenant_id    TEXT,
    workspace_id TEXT,
    -- active | rotated | revoked
    status       TEXT NOT NULL DEFAULT 'active'
                 CHECK (status IN ('active', 'rotated', 'revoked')),
    expires_at   TIMESTAMPTZ NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_oauth_refresh_grants_family
    ON oauth_refresh_grants (family_id);

CREATE INDEX IF NOT EXISTS idx_oauth_refresh_grants_expires
    ON oauth_refresh_grants (expires_at)
    WHERE status = 'active';
