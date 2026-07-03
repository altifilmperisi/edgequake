---
template_id: "doc-mig-001"
version: "1.0.0"
category: "documentation"
tags: ["migration", "postgresql", "database", "schema", "deployment"]
context_scope: "edgequake/migrations/"
status: "stable"
info_type: "guide"
info_text: "Migration documentation for EdgeQuake database schema changes and upgrade procedures"
---

# Migration Documentation

## Overview

This directory contains documentation for EdgeQuake database migrations. Each migration file in `edgequake/migrations/` follows the naming convention `NNN_YYYY_MMDD_HHMMSS_<description>.sql` and is designed to be idempotent and safe to run multiple times.

## Migration Structure

### File Naming Convention

```
NNN_YYYY_MMDD_HHMMSS_<description>.sql
```

- **NNN**: Zero-padded 3-digit migration number (001, 002, ..., 099)
- **YYYY_MMDD_HHMMSS**: Timestamp of creation
- **<description>**: Short description of the migration purpose

### Example

```
041_20260703_212000_document_stats_columns.sql
```

## Migration Types

### 1. Schema Changes

Add, modify, or remove database objects (tables, columns, indexes, constraints).

**Example**: Adding columns to documents table

```sql
ALTER TABLE documents
    ADD COLUMN IF NOT EXISTS entity_count INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS relationship_count INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS chunk_count INTEGER NOT NULL DEFAULT 0;
```

### 2. Data Backfill

Populate existing data with computed values.

**Example**: Backfilling vector table columns from JSONB metadata

```sql
UPDATE public.eq_%_vectors
SET
    document_id = COALESCE(metadata->>'document_id', metadata->>'source_document_id'),
    tenant_id = metadata->>'tenant_id',
    workspace_id = metadata->>'workspace_id'
WHERE document_id IS NULL;
```

### 3. Index Optimization

Create, drop, or modify indexes for query performance.

**Example**: Creating expression indexes on edge properties

```sql
CREATE INDEX IF NOT EXISTS idx_edge_source_id
    ON _ag_label_edge((ag_catalog.agtype_to_json(properties)->>'source_id'));
```

### 4. Constraint Fixes

Repair or add validation constraints.

**Example**: Fixing document status constraints

```sql
ALTER TABLE documents ADD CONSTRAINT documents_valid_status CHECK (
    status IN ('pending', 'processing', 'chunking', 'extracting', 'embedding',
               'indexing', 'completed', 'indexed', 'failed', 'partial_failure', 'cancelled')
);
```

## Migration Safety Features

### Idempotency

All migrations are designed to be safe to run multiple times:

- **CREATE TABLE IF NOT EXISTS**: Prevents duplicate table creation
- **ALTER TABLE ... ADD COLUMN IF NOT EXISTS**: Prevents duplicate column addition
- **CREATE INDEX IF NOT EXISTS**: Prevents duplicate index creation
- **ON CONFLICT ... DO NOTHING**: Prevents duplicate row insertion
- **DROP CONSTRAINT IF EXISTS**: Prevents constraint conflicts

### Error Handling

Migrations include defensive checks:

```sql
DO $$
BEGIN
    ALTER TABLE documents DROP CONSTRAINT IF EXISTS documents_valid_status;
EXCEPTION WHEN undefined_object THEN
    RAISE NOTICE 'Constraint documents_valid_status did not exist, skipping';
END $$;
```

### Batch Processing

Large data operations use batch processing to avoid long locks:

```sql
DECLARE
    batch_size INT := 10000;
    updated INT;
BEGIN
    LOOP
        UPDATE public.%I SET ...
        WHERE ctid IN (
            SELECT ctid FROM public.%I
            WHERE ... LIMIT %s
        )...;
        GET DIAGNOSTICS updated = ROW_COUNT;
        EXIT WHEN updated < batch_size;
        PERFORM pg_sleep(0.05);
    END LOOP;
END $$;
```

## Migration Execution

### Order

Migrations must be executed in numerical order:

1. `001_init_database.sql` - Initial schema
2. `002_add_tasks_table.sql` - Tasks table
3. ... (continue in order)
4. `041_document_stats_columns.sql` - Document statistics

### Skipping Gaps

Migration 018 is intentionally skipped. This is acceptable for:
- Removed features
- Deprecated functionality
- Migrations merged into adjacent migrations

## Common Patterns

### 1. Multi-Phase Migrations

Complex changes use multiple phases within a single migration file:

```sql
-- Phase 1: Add columns
DO $$
DECLARE
    tbl RECORD;
BEGIN
    FOR tbl IN SELECT tablename FROM pg_tables WHERE tablename LIKE 'eq_%_vectors'
    LOOP
        EXECUTE format('ALTER TABLE public.%I ADD COLUMN IF NOT EXISTS ...', tbl.tablename);
    END LOOP;
END $$;

-- Phase 2: Backfill data
DO $$
DECLARE
    tbl RECORD;
BEGIN
    FOR tbl IN SELECT tablename FROM pg_tables WHERE tablename LIKE 'eq_%_vectors'
    LOOP
        -- Update logic here
    END LOOP;
END $$;
```

### 2. Conditional Logic

Use PL/pgSQL blocks to check conditions before executing:

```sql
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age') THEN
        -- AGE-specific operations
    ELSE
        RAISE NOTICE 'Apache AGE not installed — skipping';
    END IF;
END $$;
```

### 3. Comments and Documentation

Each migration file includes:

- **Header**: Description, date, purpose
- **WHY**: Explanation of why this migration exists
- **Safety**: Idempotency guarantees
- **Verification**: Success messages

## Troubleshooting

### Migration Failed

1. **Check PostgreSQL logs** for error details
2. **Verify table existence**: `SELECT tablename FROM pg_tables WHERE schemaname = 'public'`
3. **Check column types**: `SELECT column_name, data_type FROM information_schema.columns WHERE table_name = 'documents'`

### Out of Order Migrations

If a migration was skipped or removed:

1. **Do not re-run** the missing migration (may cause conflicts)
2. **Document the gap** in this file
3. **Plan a new migration** to address the skipped functionality

### Large Datasets

For datasets with millions of rows:

1. **Use batch processing** with `LIMIT` and `ctid`
2. **Add `pg_sleep(0.05)`** between batches to avoid long locks
3. **Monitor progress** with `SELECT COUNT(*) FROM %I WHERE ...`

## References

- [PostgreSQL Documentation](https://www.postgresql.org/docs/)
- [sqlx Documentation](https://docs.rs/sqlx/)
- [EdgeQuake Specifications](../specs/)
