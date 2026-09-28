# 10 — Implementation plan

Parent: [README](README.md) · Architecture: [06](06-backend-architecture.md) · Edge cases: [09](09-edge-cases.md)

## Work packages (order)

```text
  WP-1 domain SSOT ──► WP-2 M160 ──► WP-3 vision split
         │                              │
         └──────────► WP-4 reuse ◄──────┘
                          │
                          ▼
                       WP-5 record in full pipeline
                          │
                          ▼
                       WP-6 API + executor
                          │
                          ▼
                       WP-7 frontend
                          │
                          ▼
                       WP-8 e2e + screenshots
                          │
                          ▼
                       WP-9 gates
```

| WP | Deliverable | DRY/SOLID note |
|----|-------------|----------------|
| WP-1 | `page_sections.rs`, `ReprocessStage`, `PageScope` | Eliminate duplicate splitters |
| WP-2 | M160 + `PageStateStorage` | Expand-only; RLS from 148 |
| WP-3 | `vision_assets.rs` + sink hook | SRP; convert() byte-identical |
| WP-4 | `ChunkReuseIndex` | OCP over resume map |
| WP-5 | Record states in full PDF/Insert | One writer path |
| WP-6 | Health + reprocess API + executor modules | Planner pure |
| WP-7 | Strip, dialog, i18n, hooks | Pure libs first |
| WP-8 | Playwright + ANALYSIS | Screenshot inspect |
| WP-9 | fmt/clippy/tests | Release hygiene |

## Definition of done

- All REQ-151-01..12 implemented or explicitly deferred in [12](12-risks.md).
- EC-151-01..20 have a named test.
- Screenshots inspected and green in ANALYSIS.md.
- `cargo clippy -D warnings` and Playwright mocked suite pass.

Cross-refs: tests [11](11-test-plan.md).
