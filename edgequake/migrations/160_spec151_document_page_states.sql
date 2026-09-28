-- ============================================================================
-- Migration 160: document_page_states (SPEC-151 partial page reprocess)
-- ============================================================================
-- Durable per-page parse / figures / entities status so partial OCR and
-- staged reprocess never lose healthy page work (LAW-151-6).
-- RLS mirrors document_pages (migration 148).

SET search_path = public;

CREATE TABLE IF NOT EXISTS document_page_states (
    page_state_id       UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    document_id         UUID NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    workspace_id        UUID NOT NULL REFERENCES workspaces(workspace_id) ON DELETE CASCADE,
    page_number         INT NOT NULL CHECK (page_number >= 1),

    parse_status        TEXT NOT NULL DEFAULT 'pending',
    parse_error         TEXT,
    parse_attempts      INT NOT NULL DEFAULT 0,
    parse_method        TEXT,
    parse_model         TEXT,
    raw_markdown        TEXT,
    raw_sha256          TEXT,

    figures_status      TEXT NOT NULL DEFAULT 'pending',
    figures_count       INT NOT NULL DEFAULT 0,
    figures_error       TEXT,

    entities_status     TEXT NOT NULL DEFAULT 'pending',
    chunk_count         INT NOT NULL DEFAULT 0,
    failed_chunk_count  INT NOT NULL DEFAULT 0,
    entities_error      TEXT,

    last_track_id       TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE (document_id, page_number),
    CONSTRAINT document_page_states_parse_status_chk
        CHECK (parse_status IN ('pending', 'running', 'ok', 'failed', 'skipped')),
    CONSTRAINT document_page_states_figures_status_chk
        CHECK (figures_status IN ('pending', 'running', 'ok', 'failed', 'skipped')),
    CONSTRAINT document_page_states_entities_status_chk
        CHECK (entities_status IN ('pending', 'running', 'ok', 'failed', 'skipped'))
);

CREATE INDEX IF NOT EXISTS idx_document_page_states_workspace_doc
    ON document_page_states (workspace_id, document_id);

CREATE INDEX IF NOT EXISTS idx_document_page_states_doc_parse
    ON document_page_states (document_id, parse_status);

COMMENT ON TABLE document_page_states IS
    'SPEC-151 per-page parse/figures/entities health. Cascade-deletes with documents.';

ALTER TABLE document_page_states ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS document_page_states_workspace_select ON document_page_states;
DROP POLICY IF EXISTS document_page_states_workspace_insert ON document_page_states;
DROP POLICY IF EXISTS document_page_states_workspace_update ON document_page_states;
DROP POLICY IF EXISTS document_page_states_workspace_delete ON document_page_states;

CREATE POLICY document_page_states_workspace_select ON document_page_states
    FOR SELECT
    USING (
        workspace_id::text = COALESCE(current_setting('app.current_workspace_id', true), '')
        OR current_setting('app.current_workspace_id', true) IS NULL
        OR current_setting('app.current_workspace_id', true) = ''
    );

CREATE POLICY document_page_states_workspace_insert ON document_page_states
    FOR INSERT
    WITH CHECK (
        workspace_id::text = COALESCE(current_setting('app.current_workspace_id', true), '')
        OR current_setting('app.current_workspace_id', true) IS NULL
        OR current_setting('app.current_workspace_id', true) = ''
    );

CREATE POLICY document_page_states_workspace_update ON document_page_states
    FOR UPDATE
    USING (
        workspace_id::text = COALESCE(current_setting('app.current_workspace_id', true), '')
        OR current_setting('app.current_workspace_id', true) IS NULL
        OR current_setting('app.current_workspace_id', true) = ''
    )
    WITH CHECK (
        workspace_id::text = COALESCE(current_setting('app.current_workspace_id', true), '')
        OR current_setting('app.current_workspace_id', true) IS NULL
        OR current_setting('app.current_workspace_id', true) = ''
    );

CREATE POLICY document_page_states_workspace_delete ON document_page_states
    FOR DELETE
    USING (
        workspace_id::text = COALESCE(current_setting('app.current_workspace_id', true), '')
        OR current_setting('app.current_workspace_id', true) IS NULL
        OR current_setting('app.current_workspace_id', true) = ''
    );

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'edgequake') THEN
        GRANT SELECT, INSERT, UPDATE, DELETE ON document_page_states TO edgequake;
    END IF;
END $$;
