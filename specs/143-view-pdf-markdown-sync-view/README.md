# SPEC-143 — PDF / Markdown sync view

> **Mission:** In the document side-by-side viewer, the PDF page and the
> Markdown pane stay aligned using existing `<!-- edgequake-page:N -->`
> markers. Sync is one direction at a time: `none`, `pdf-to-md` (default),
> or `md-to-pdf` (persisted as `eq-page-sync-mode`). Mouse wheel (and keyboard)
> navigate PDF pages via a continuous scroll stack — not toolbar-only
> single-page flips. Shipped on `main` after v0.27.0.
>
> **Method:** One `PageSyncController`; PDF continuous stack + IntersectionObserver;
> markdown marker → DOM `data-eq-page` anchors; sync lock prevents feedback loops.
>
> **Shipped:** on `main` after v0.27.0 (not in the v0.27.0 tag).

## One-screen verdict

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│  LAW: Page attribution comes from markdown markers + chunk page_start.       │
│                                                                              │
│  Product path:                                                               │
│    <!-- edgequake-page:N -->  →  inject DOM anchors                          │
│    PDF continuous stack       →  wheel/keyboard page nav                     │
│    Page sync mode             →  none | pdf-to-md | md-to-pdf                  │
│                                                                              │
│  none = independent scroll. Missing markers = PDF works; MD follow no-ops.   │
│  No DB migration. SPEC-128 overlay stays on active page only.                │
└──────────────────────────────────────────────────────────────────────────────┘
```

## Document map

```ascii
  README
    → 00-why (5 WHY)
    → 01-first-principles (LAW-143-1..7)
    → 02-cross-ref-matrix
    → 03-code-as-is
    → 04-target-architecture
    → 05-lenses/ (PO, fullstack, DB, UX, front, PDF viewer, AI)
    → 06-ux-ui-spec
    → 07-implementation-plan
    → 08-e2e-test-matrix
    → 09-edge-cases
    → 10-acceptance
    → 11-honest-assessment
```

## Status board

| ID | Item | Status |
|----|------|--------|
| D1 | Doc pack | Done |
| I1 | `page-markers.ts` + sanitize allowlist | Done |
| I2 | `usePageSyncController` | Done |
| I3 | PDF continuous stack + `onPageChange` | Done |
| I4 | MD anchors + observer | Done |
| I5 | FEAT0733 sync toggle + surface wire | Done |
| T1 | Unfakable Playwright + unit | Done |
| A1 | Acceptance | Done (checklist in 10-acceptance) |

## Locked decisions

| Decision | Choice |
|----------|--------|
| PDF scroll model | Continuous multi-page stack; native wheel |
| Large PDFs | Windowed render when `numPages > 20` (±2) |
| Sync driver | `<!-- edgequake-page:N -->` → `data-eq-page` |
| Sync directions | Explicit `none` \| `pdf-to-md` (default) \| `md-to-pdf` |
| Sync control | Real FEAT0733 toggle (default ON) |
| URL | Debounced `?page=N` via `onPageChange` |
| Keyboard | PageUp/Down + ArrowUp/Down (PDF focused) |
| Backend | No marker grammar / DB change |

## Cross-spec anchors

| Spec | Relevance |
|------|-----------|
| [SPEC-033](../033-page-lineage/) | Controlled PDF page + deeplink |
| [SPEC-083 X-13](../083-improvements/) | Marker grammar SSOT |
| [SPEC-128](../128-layout-overlay/) | Overlay on active page |
| [SPEC-135](../135-chunking/) | `page_start` / `page_end`; deeplink start |
| [SPEC-142](../142-precise-links-on-query/) | Citation → `?page=` |
| FEAT0733 | Panel synchronization controls (claim → real) |
