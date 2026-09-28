# 01 — First Principles (LAW-143)

## Axioms

| ID | Law | Operational meaning |
|----|-----|---------------------|
| **LAW-143-1** | Page attribution is storage SSOT | Active page from `<!-- edgequake-page:N -->` and/or chunk `page_start` — never invented in UI |
| **LAW-143-2** | One controller, two views | `PageSyncController` owns `activePage`; PDF stack and MD anchors are views |
| **LAW-143-3** | Sync lock | Driver pane (`pdf` \| `md` \| `external`) owns updates until settle (150–250ms) |
| **LAW-143-4** | Wheel scrolls the stack | Continuous multi-page stack; native overflow scroll — not edge-detect on a single page |
| **LAW-143-5** | Overlay binds to active page | SPEC-128 layout overlay renders on the active sheet only |
| **LAW-143-6** | Degrade honestly | Missing markers → PDF nav works; MD sync no-ops; toggle disabled or inert |
| **LAW-143-7** | Unfakable contracts | E2E asserts `data-page`, `data-eq-page`, and URL `?page=` — not screenshots alone |
| **LAW-143-8** | Deeplink = `page_start` | Cross-page spans (`page_end > page_start`) navigate to `page_start` (SPEC-135) |
| **LAW-143-9** | Sync is one explicit direction | Segmented control: `none` \| `pdf-to-md` \| `md-to-pdf`. First visit defaults to PDF→MD; last choice remembered. Bidirectional sync is forbidden. |
| **LAW-143-10** | Marker grammar frozen | Do not change `<!-- edgequake-page:N -->` (SPEC-083 X-13) |

## Anti-patterns

| Anti-pattern | Violates |
|--------------|----------|
| Invent page from scroll fraction without markers | LAW-143-1 |
| Two independent `pageNumber` sources of truth | LAW-143-2 |
| Bidirectional scroll without lock (feedback loop) | LAW-143-3 |
| Wheel-at-edge page turn on single `<Page>` | LAW-143-4 |
| Overlay on every sheet in the stack | LAW-143-5 |
| Fake sync when no anchors exist | LAW-143-6 |
| E2E that only checks “viewer visible” | LAW-143-7 |
| Deeplink to `page_end` for multi-page chunks | LAW-143-8 |

## Cross-refs

- WHY: [00-why.md](00-why.md)
- Architecture: [04-target-architecture.md](04-target-architecture.md)
