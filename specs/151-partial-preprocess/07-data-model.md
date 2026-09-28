# 07 — Data model (Database Expert)

Parent: [README](README.md) · Laws: [01](01-first-principles.md) · Backend: [06](06-backend-architecture.md)

## WHY

Without a durable per-page row, OCR salvage and health UI are impossible. Migration **160** is expand-only (SPEC-150 LAW-150-3).

## Migration 160 — `document_page_states`

```text
  document_page_states
  ├── page_state_id UUID PK
  ├── document_id UUID FK → documents(id) ON DELETE CASCADE
  ├── workspace_id UUID NOT NULL FK → workspaces
  ├── page_number INT CHECK (>= 1)
  ├── UNIQUE (document_id, page_number)
  │
  ├── parse_status TEXT NOT NULL DEFAULT 'pending'
  ├── parse_error TEXT
  ├── parse_attempts INT NOT NULL DEFAULT 0
  ├── parse_method TEXT
  ├── parse_model TEXT
  ├── raw_markdown TEXT
  ├── raw_sha256 TEXT
  │
  ├── figures_status TEXT NOT NULL DEFAULT 'pending'
  ├── figures_count INT NOT NULL DEFAULT 0
  ├── figures_error TEXT
  │
  ├── entities_status TEXT NOT NULL DEFAULT 'pending'
  ├── chunk_count INT NOT NULL DEFAULT 0
  ├── failed_chunk_count INT NOT NULL DEFAULT 0
  ├── entities_error TEXT
  │
  ├── last_track_id TEXT
  ├── created_at / updated_at TIMESTAMPTZ
```

Status vocabulary: `pending` | `running` | `ok` | `failed` | `skipped`.

## RLS

Copy workspace-scoped pattern from migration **148** (`document_pages`):

- `ENABLE ROW LEVEL SECURITY`
- Policies: SELECT/INSERT/UPDATE/DELETE where `workspace_id::text = current_setting('app.current_workspace_id', true)` OR setting null/empty
- Grants to `edgequake` role
- Index `(workspace_id, document_id)`

## Related subset upserts

| Table | Change |
|-------|--------|
| `document_pages` | Add `upsert_pages_subset(document_id, pages[])` — do not wipe other pages |
| `document_mm_assets` | Delete+insert only rows for `page_num IN (…)`; leave other pages |

## Manifest / checksums

- File: `edgequake/migrations/160_spec151_document_page_states.sql`
- `manifest.toml`: version 160, phase `expand`, lock_class `ddl_access_exclusive`
- Bump `compat_serve_max = 160`
- Append SHA-384 to `checksums.lock`

## Legacy derivation (no rows)

When health GET finds zero rows:

```text
  for each page marker in markdown:
    parse = placeholder? failed : ok
    figures = mm_assets count for page (ok if >0 else pending)
    entities = chunks overlapping page; failed if any in failed_chunks
```

Cross-refs: implementation WP-2 in [10](10-implementation-plan.md).
