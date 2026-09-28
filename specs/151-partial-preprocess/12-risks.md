# 12 — Risks (honest)

Parent: [README](README.md) · Plan: [10](10-implementation-plan.md)

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Rebuild briefly removes doc from query | High | Medium | Progress UI; short window; document status=processing |
| Content-hash reuse misses near-identical chunks (whitespace) | Medium | Low | Normalize whitespace before hash; warn on low reuse ratio |
| Vision refactor changes Full convert bytes | Medium | High | Golden markdown fixtures; byte-identical tests on convert() |
| `pdf_processing.rs` still too large | Medium | Medium | Keep page_reprocess modules isolated; do not inline |
| Frontend monotonic progress rejects updates | Medium | Medium | New phase `page_reprocess` |
| M160 RLS diverge from 148 | Low | High | Copy 148 policies verbatim |
| Pass B offset invalidation | High | Medium | Scope Pass B to selected sub-doc only |
| No snapshot on old docs | High | Medium | Warning in dry_run; full extract for all chunks |

## Deferred

- Stable page-scoped chunk IDs (`{doc}-pN-cK`) — larger migration; reuse-by-hash is enough for v1.
- Surgical graph removal — rejected by product choice (rebuild+reuse).
- Object storage for page PNGs.

## Open questions (non-blocking)

- Should selecting all pages auto-redirect to Full reprocess dialog? (v1: warn only)
