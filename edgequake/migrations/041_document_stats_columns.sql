-- Migration: 041_document_stats_columns
-- Description: Add document-level statistics columns to documents table
-- Date: 2026-07-03
--
-- Context: Migration 032 added status constraints and migration 034 fixed
-- extraction_method constraints. Migration 041 adds per-document statistics
-- columns (entity_count, relationship_count, chunk_count) to enable fast
-- queries on document-level metrics without aggregating from child tables.
--
-- These columns mirror the workspace_metrics_history table (migration 016)
-- but at the document level, enabling per-document trend analysis and
-- capacity planning.

SET search_path = public;

-- Add statistics columns to documents table
ALTER TABLE documents
    ADD COLUMN IF NOT EXISTS entity_count INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS relationship_count INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS chunk_count INTEGER NOT NULL DEFAULT 0;

-- Add index for document-level metrics queries
CREATE INDEX IF NOT EXISTS idx_documents_entity_count
    ON documents(entity_count DESC);

CREATE INDEX IF NOT EXISTS idx_documents_relationship_count
    ON documents(relationship_count DESC);

CREATE INDEX IF NOT EXISTS idx_documents_chunk_count
    ON documents(chunk_count DESC);

-- Add comment explaining the table purpose
COMMENT ON COLUMN documents.entity_count IS
    'Number of entities extracted from this document';

COMMENT ON COLUMN documents.relationship_count IS
    'Number of relationships extracted from this document';

COMMENT ON COLUMN documents.chunk_count IS
    'Number of chunks this document was split into during processing';

DO $$ BEGIN
    RAISE NOTICE 'Migration 041 completed: document stats columns (entity_count, relationship_count, chunk_count) added to documents table';
END $$;
