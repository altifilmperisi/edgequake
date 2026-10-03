-- ============================================================================
-- Migration 165: SPEC-158 — map SSO access-token jti to the refresh family
-- ============================================================================
-- Back-channel logout (LAW-158-10, EC-158-22) has no bearer token to present.
-- Every access JWT minted for a federated family is recorded here so logout
-- can put those jti values on the SPEC-154 denylist. Rows expire with the JWT.

SET search_path = public;

CREATE TABLE IF NOT EXISTS federated_access_jti (
    jti        TEXT PRIMARY KEY,
    family_id  UUID NOT NULL REFERENCES federated_sessions(family_id) ON DELETE CASCADE,
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_federated_access_jti_family
    ON federated_access_jti (family_id);
