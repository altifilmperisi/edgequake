# Lens — Database Expert

Parent: [README](../README.md) · Contract: [06](../06-data-and-db-contract.md)

## Verdict

**No DDL / migrations required** for SPEC-157 default delivery. Companion state
is URL + `localStorage`. Document and message context JSON already carry the
fields needed for Source pane.

## Read paths (existing)

| Path | Notes |
|------|-------|
| Document by id | Workspace-scoped; enforce 403/404 cross-tenant |
| PDF content / binary | Auth; 404 when missing |
| Conversation messages | Context JSON with sources + entities |
| Optional neighbourhood | Only if expand fetches server-side |

## Write paths

None for open/close companion. Chat submit may already accept document scope
ids (W4 uses existing API).

## Gaps to watch

| Gap | Impact | Action |
|-----|--------|--------|
| Non-stream `subgraph: None` | Weaker Graph pane after non-SSE | Optional W5 API parity — no schema |
| Entity node id not durable | Name fallback | Client map; optional persist field |
| Large context JSON | Payload size | Cap display N; full Studio escape |

## Indexing / performance

No new indexes. Avoid N+1: Source pane uses existing React Query keys
(`['document', id]`, `['pdfContent', …]`). Do not refetch full workspace graph
to show answer subgraph.

## Migration policy

If OpenAPI / message JSON gains `graph_node_id`:

1. Additive field only.
2. Update contract tests.
3. No backfill required (fallback remains).

Cross-ref: F-157-08, F-157-09 in [02](../02-surfaces.md) · EC-157-18,27.
