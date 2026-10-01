# Lens — Full Stack Developer

Parent: [README](../README.md) · Surfaces: [02](../02-surfaces.md) · Plan: [10](../10-implementation-plan.md)

## Responsibilities

- Own end-to-end wiring: OpenAPI ↔ WebUI types ↔ GraphEngine ↔ cargo e2e.
- Enforce DRY/SOLID file splits and store selector discipline.
- Keep SPEC-099/100/101/102/154 suites green while landing 155.

## Key code anchors

| Concern | Path |
|---------|------|
| Graph orchestration | `components/graph/graph-viewer.tsx` → split W4 |
| Renderer | `graph-renderer.tsx` → `lib/graph/engine/` |
| Store | `stores/use-graph-store.ts` → slices |
| Stream | `hooks/use-graph-stream.ts` |
| API | `lib/api/edgequake/graph.ts` + chat stream |
| Shell | `app/(dashboard)/layout.tsx`, `w/[slug]/layout.tsx` → shared |
| Backend graph | `edgequake-api/.../handlers/graph/*` |
| Communities | `edgequake-storage/.../community*.rs` |

## Implementation principles

```text
  1. Feature flag GraphEngine until W4 DoD
  2. applyDelta only — never replace graph reference for filters
  3. One filter pipeline module used by viewer + store selectors
  4. MSW/route.fulfill fixtures for PW — don't depend on make dev
  5. Prefer generating types from OpenAPI after W3
```

## Anti-patterns to delete

- Whole-store `useGraphStore()` in hot paths
- `window.location.href` on graph fatal
- Duplicate layout lists / zoom handlers / formatCost
- Client Louvain as SSOT after W3 communities land

## Test ownership

- W3 cargo tests
- W4 graph_filter_no_rebuild, truncation, export
- Contract test after OpenAPI refresh

## Sequencing tip

Land W3 DTOs before W5 hulls (need `community_id`). Land W3 node ids before
W6 answer highlight.
