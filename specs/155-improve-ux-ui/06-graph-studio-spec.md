# 06 — Graph Studio specification (flagship)

Parent: [README](README.md) · Findings G-* [04](04-findings.md) · Laws: 3,4,5,6,8,10,11,12 · Next: [07-screens-spec](07-screens-spec.md)

## WHY

The graph is EdgeQuake’s differentiator. Today it rebuilds on every filter,
drops parallel edges, mis-reports truncation, exports a blank/edges-only PNG,
and never lights answer entities. Graph Studio is the highest-bar surface.

## Product name & job

**Graph Studio** — explore, filter, explain, and export the workspace knowledge
graph with a stable mental map.

Primary jobs:

1. Orient (overview → community → ego).
2. Find (search, type, document scope, time — **wired**).
3. Explain (answer-on-graph, evidence in details).
4. Edit (merge/delete — existing, keep behind permission).
5. Export / share (PNG, SVG, saved view JSON).

## Architecture (SOLID)

```text
  +------------------+       +------------------------+
  | GraphStudioPage  |       | useGraphStore (data)   |
  |  (thin React)    |------>|  nodes, edges, filters |
  +--------+---------+       |  selection ids only    |
           |                 +-----------+------------+
           | applyDelta / setFocus       |
           v                             v
  +------------------+       +------------------------+
  | GraphEngine      |<------| layout worker (FA2)    |
  |  MultiGraph      |       +------------------------+
  |  Sigma (long)    |
  |  theme tokens    |
  |  export-image    |
  +------------------+

  Drop: @react-sigma/*  |  Forbid: Sigma.kill on filter/theme
```

### GraphEngine API (normative)

```text
  create(container, options) -> Engine
  applyDelta({ upsertNodes, upsertEdges, removeNodeIds, removeEdgeIds })
  setFocus({ mode: 'none'|'hover'|'select'|'ego'|'path'|'answer', ids, depth? })
  setFilters({ types, relTypes, query, timeRange, documentIds })  // dim layer
  setLod(tier: 0|1|2)
  setLayoutMode(mode) / startLayout() / stopLayout() / pin(nodeId)
  setTheme(resolvedTokens)
  getCamera() / setCamera()
  exportImage({ format: 'png'|'svg', pixelRatio })
  destroy()
```

Store holds **serialisable** graph data + UI prefs. Engine holds **imperative**
GPU state. React never calls `new Sigma` outside Engine.

### File split target (W4)

| Today | Target modules (≤300 lines each) |
|-------|----------------------------------|
| graph-viewer 1041 | `GraphStudioPage`, `useGraphDataSource`, `GraphToolbar`, `GraphSidePanels` |
| graph-renderer 889 | `GraphEngine`, `reducers`, `programs`, `interactions` |
| use-graph-store 1068 | `graph-data-slice`, `graph-filter-slice`, `graph-view-slice`, `graph-stream-slice` |

## Rendering & data correctness (W4)

| Requirement | Detail |
|-------------|--------|
| Multigraph | `new Graph({ multi: true, type: 'directed' })`; curvature spread by parallel index |
| Truncation | Honour server `is_truncated` + `total_nodes/edges` (workspace-scoped after W3); fix inverted client logic |
| Time filter | Apply in single filter pipeline (DRY with store selectors) |
| include_orphans / depth | Pass through to REST & stream |
| Export | `@sigma/export-image`; background from `--graph-canvas-bg` |
| Camera / minimap | Use Sigma display data / official camera API; delete broken normaliser or fix to max(w,h) |
| Keyboard | Roving tabindex on canvas; arrow moves selection among **visible** nodes; Enter opens details; Shift+F10 menu; never preventDefault Tab at window |
| WebGL fail | Friendly ErrorState + “Open as table”; no raw GL messages; no `window.location` |

## Interaction model (W5)

```text
  Focus+context (default)
       |
       +-- hover: dim non-neighbours (opacity), keep positions
       +-- select: emphasize node + neighbours; details panel
       +-- search/type filter: dim non-matches (LAW-155-4)

  Modes
       +-- Ego: depth 1–3 slider; per-hop cap from API
       +-- Path: pick A then B; graphology-shortest-path; highlight
       +-- Lasso: multi-select → selectedNodes (wire dead state)
       +-- Answer: highlight subgraph from query message
```

## Scale & progressive disclosure (LAW-155-5)

| LOD | Camera / size | Show |
|-----|---------------|------|
| 0 Overview | zoomed out | Community hulls + supernode counts; hide edge labels |
| 1 Mid | default | Top-degree labels; curved edges; dim filters |
| 2 Detail | zoomed in | All labels in view; edge labels if &lt; N edges |

Caps: soft 2k interactive; hard 5k with supernode collapse; “Load more”
appends via delta (no full restart). Worker FA2 with pin-on-drag.

Community hulls: convex hull (or padded contour) per `community_id` from
**server** (W3). Legend lists communities + entity types.

## Colour & a11y

- Entity colours from CSS tokens / workspace overrides (SPEC-102 retained).
- Community palette: fixed Okabe–Ito-like token set + shape markers (circle /
  square / diamond program variants where needed).
- Announcer: keep `GraphAccessibilityAnnouncer`; announce focus mode changes.
- **Graph as table:** virtualised table of visible nodes/edges; same selection
  sync; required for WCAG when canvas is `role="application"`.

## Saved views & bookmarks

```text
  SavedView {
    workspaceId, name,
    camera, layoutMode, lod,
    filters, documentScope,
    nodePositions?: Map,   // optional snapshot
    focusMode?
  }
  Storage: per-workspace key (not global "graph-bookmarks")
```

Restore must not require full rebuild if Engine alive — `setCamera` +
`setFilters` + position attributes.

## Answer-on-graph (W6, LAW-155-11)

```text
  Query message
    done/context.subgraph.entities[]  --map-->  graph node ids
    relationships[]                   --map-->  edge keys
         |
         v
  Engine.setFocus({ mode: 'answer', ids })
  Deep link: /graph?answerMessage=<id>|&highlight=...
  UI: button "Show on graph" on chat-message / citations
```

Id mapping: prefer API `node_id` (W3); fallback normalised name within
workspace. Persist enough context on `messages.context` to reopen.

## Performance budgets

| Action              | Budget                                       |
| ---------------------| ----------------------------------------------|
| Toggle entity type  | &lt; 50ms; 0 Sigma.kill                      |
| Stream batch upsert | RAF-batched applyDelta; layout optional soft |
| Pan/zoom 2k nodes   | 60fps                                        |
| Layout worker       | UI thread free; progress indicator           |
| Export PNG 2k       | &lt; 3s                                      |

## Dependency upgrades

- `sigma` → 3.0.3
- add `@sigma/export-image`
- use FA2 worker supervisor from `graphology-layout-forceatlas2`
- add `graphology-shortest-path` for path mode
- remove `@react-sigma/*`

## E2E anchors (see [11](11-e2e-test-matrix.md))

- Filter does not remount `[data-graph-engine]` instance id
- Truncation banner matches fixture totals
- Export download non-empty PNG
- Parallel edges both visible
- Keyboard: Tab leaves canvas; arrows move selection
- Answer deep link highlights ≥1 node
- axe on Graph Studio chrome (not WebGL pixels)

Cross-ref: [08-data-contract](08-data-contract.md) · [LENS-ai-engineer](lenses/LENS-ai-engineer.md).
