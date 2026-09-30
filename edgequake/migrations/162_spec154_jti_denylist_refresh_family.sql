-- ============================================================================
-- Migration 162: SPEC-154 Wave 4 — durable jti denylist + web refresh rotation family
-- ============================================================================

SET search_path = public;

CREATE TABLE IF NOT EXISTS jwt_jti_denylist (
    jti          TEXT PRIMARY KEY,
    expires_at   TIMESTAMPTZ NOT NULL,
    revoked_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    reason       TEXT NOT NULL DEFAULT 'logout'
);

CREATE INDEX IF NOT EXISTS idx_jwt_jti_denylist_expires
    ON jwt_jti_denylist (expires_at);

ALTER TABLE refresh_tokens
    ADD COLUMN IF NOT EXISTS family_id UUID,
    ADD COLUMN IF NOT EXISTS status TEXT NOT NULL DEFAULT 'active';

-- Backfill family_id for legacy rows (each token is its own family).
UPDATE refresh_tokens
SET family_id = token_id
WHERE family_id IS NULL;

ALTER TABLE refresh_tokens
    ALTER COLUMN family_id SET DEFAULT gen_random_uuid();

-- Align legacy revoked boolean into status.
UPDATE refresh_tokens
SET status = 'revoked'
WHERE revoked = TRUE AND status = 'active';

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'refresh_tokens_status_check'
    ) THEN
        ALTER TABLE refresh_tokens
            ADD CONSTRAINT refresh_tokens_status_check
            CHECK (status IN ('active', 'rotated', 'revoked'));
    END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_refresh_tokens_family
    ON refresh_tokens (family_id);
