# 02 — Surfaces (code map)

Parent: [README](README.md) · Laws: [01](01-first-principles.md) · Next: [03-standards-crosswalk](03-standards-crosswalk.md)

## App routes (`edgequake_webui/src/app`)

| Route | Page file | Primary components | Lines (approx) |
|-------|-----------|--------------------|----------------|
| `/` Dashboard | `(dashboard)/page.tsx` (~259) | `dashboard/*` | stats-card 172, recent-activity 127 |
| `/documents` | `(dashboard)/documents/page.tsx` | `documents/document-manager.tsx` | **1251** |
| `/documents/[id]` | `(dashboard)/documents/[id]/page.tsx` | side-by-side, pdf-viewer, hierarchy | **1180** page; pdf-viewer **902** |
| `/query` | `(dashboard)/query/page.tsx` | `query-interface`, citations, history | citations **1079**, history-v2 **990** |
| `/graph` | `(dashboard)/graph/page.tsx` (~88) | `graph-viewer`, `graph-renderer` | viewer **1041**, renderer **889** |
| `/pipeline` | `(dashboard)/pipeline/page.tsx` | `pipeline-monitor` + cards | dialog twin **999** |
| `/costs` | `(dashboard)/costs/page.tsx` (~264) | `cost/*` | period wired cosmetically |
| `/workspace` | `(dashboard)/workspace/page.tsx` (~398) | `workspace/*` | rebuild button **492** |
| `/settings` | `(dashboard)/settings/page.tsx` | settings cards | page **645** |
| `/knowledge` | `(dashboard)/knowledge/page.tsx` | injections UI | **428** |
| `/knowledge/[id]` | `…/knowledge/[id]/page.tsx` | detail | **356** |
| `/api-explorer` | `(dashboard)/api-explorer/page.tsx` | Scalar view | ~102 |
| `/login` | `(auth)/login/page.tsx` | form | ~191 |
| `/w/[slug]/*` | parallel routes | **incomplete shell** | layout ~49; redirects via `push` |

Shell: `(dashboard)/layout.tsx` — AuthGuard, FirstRunWizard, Sidebar, Header,
BackendStatusBanner, DynamicBreadcrumb, ApiErrorBoundary, TenantGuard.

## Graph surface (flagship)

```text
  graph/page.tsx
       |
       v
  graph-viewer.tsx (1041) ---- data: stream | REST | document-scope
       |
       +-- graph-renderer.tsx (889)  Sigma init + reducers  [LAW-155-3 break]
       +-- graph-filters / search / controls / legend / minimap / export
       +-- entity-browser-panel (824) / node-details (609) / edit dialogs
       |
       v
  use-graph-store.ts (1068) + use-graph-stream / expansion / keyboard
       |
       v
  lib/api/edgequake/graph.ts  -->  GET /graph, /graph/stream, neighborhood, …
```

| Module | Path | Role | Smell |
|--------|------|------|-------|
| Viewer | `components/graph/graph-viewer.tsx` | Orchestration | God file; truncation inverted :270 |
| Renderer | `components/graph/graph-renderer.tsx` | Sigma lifecycle | Rebuild deps :728 |
| Store | `stores/use-graph-store.ts` | Zustand data | Whole-store subs |
| Layouts | `lib/graph/layouts.ts` | FA2 etc. | Main thread; no worker |
| Colours | `lib/graph/entity-type-colors.ts` | Hex map | Not CSS tokens |
| Clustering | `lib/graph/clustering.ts` | Client Louvain | Disagrees with server |
| Export | `components/graph/graph-export.tsx` | PNG | First canvas = edges |
| Keyboard | `hooks/use-graph-keyboard-navigation.ts` | Keys | Global hijack |
| Camera | `lib/graph/camera-utils.ts` | Focus | Wrong normalisation |
| Dead | `components/graph/graph-events.tsx` | @react-sigma | Unused |

Pinned libs: `sigma@3.0.2` (npm latest 3.0.3), `graphology@0.26`,
`@sigma/node-border`, `@sigma/edge-curve`. Unused deps: `@react-sigma/*`.

## Design system / shell

| Surface | Path | Notes |
|---------|------|-------|
| Tokens | `app/globals.css`, `app/design-tokens.css` | oklch shadcn; no success/warn/info |
| Font | `app/layout.tsx` `--font-inter` vs `globals.css` `--font-geist-sans` | **Mismatch** |
| UI kit | `components/ui/*` | Radix + CVA |
| Sidebar/Header | `components/layout/*` | Hard-coded English strings |
| Shortcuts | `hooks/use-keyboard-shortcuts.ts` | `searchOpen` never mounts palette |
| i18n | `locales/{en,fr,zh}.json`, `lib/i18n.ts` | ja/ko offered without files |
| Shared (dead) | `shared/empty-state`, `skeletons`, `responsive-table` | Zero importers |

## Backend graph / query surfaces

| Route | Handler area | Issue |
|-------|--------------|-------|
| `GET /graph` | `handlers/graph/*` | `start_node` degrees 0; totals = reltuples |
| `GET /graph/stream` | `graph_stream.rs` | Ignores `start_node` |
| `POST /graph/degrees/batch` | `popular.rs` | No tenant filter |
| Entities / relationships | CRUD + merge | Synthetic edge id `{src}_{tgt}` |
| Neighborhood | depth clamp 1–3 | Fixed |
| Lineage document | `document_graph_*` | Silent LIMIT 5000 |
| Communities | storage `community.rs` | **No HTTP endpoint** |
| Query/chat stream | `query_stream` / `chat/streaming` | `subgraph` unused by graph UI |

## Dead code (purge candidates — verify import search in W1)

```text
  components/lineage/*          (~538+ lines, no importers)
  components/progress/*         (eta/live/stage; reimplemented in documents)
  query/conversation-history-panel.tsx  (v1 unused; v2 live)
  shared/empty-state.tsx        (needed — wire, don't delete)
  shared/skeletons.tsx          (needed — wire)
  shared/responsive-table.tsx   (needed — wire)
  @react-sigma/* packages       (remove from package.json)
  design-tokens *-dark vars     (unused)
```

## Per-screen ASCII (current → target sketch)

### Dashboard

```text
  NOW                              TARGET
  +----+----+----+----+            +---------------------------+
  |stat|stat|stat|stat|            | PageHeader  [Upload]      |
  +----+----+----+----+            +---------------------------+
  | QuickAction x3    |            | Primary metric row (3)    |
  +-------------------+            | Recent + health (header)  |
  | Recent (min-h 300)|            | First-run welcome if empty|
  +-------------------+            +---------------------------+
```

### Graph Studio

```text
  NOW                                      TARGET
  +--------+----------------+------+       +------+--------------------+------+
  |filters |  canvas        |det.  |       |tools | canvas (stable)    |det.  |
  |search  |  (rebuilds)    |panel |       | LOD  | hulls · dim focus  |panel |
  |legend  |  broken export |      |       |table | minimap OK         |      |
  +--------+----------------+------+       +------+--------------------+------+
```

### Query

```text
  NOW                                 TARGET
  modes wrap · history always-on      segmented modes · history &lt;lg drawer
  textarea disabled while stream      compose while stream + Stop
  no IME guard                        isComposing guard
  subgraph ignored                    "Show on graph"
```

### Documents / detail / settings / costs / pipeline / knowledge / login / w-slug

See [07-screens-spec](07-screens-spec.md) for full matrices. Critical defects:

- Costs period cosmetic; BudgetIndicator `null`
- Dashboard link `/documents?id=` unread
- `w/[slug]` layout missing AuthGuard / error boundary
- Settings flat 12+ cards; language picker broken
- Login no `autoComplete`; default landing `/graph`

## Ownership (implementation)

| Area | Primary lens | Wave |
|------|--------------|------|
| Tokens / PageShell | Front + UX | W1 |
| Quality gates | Full stack | W2 |
| Graph/query API | Database + Full stack | W3 |
| GraphEngine | Front + Full stack | W4–W5 |
| Answer-on-graph | AI Engineer | W6 |
| Screens | UX + Front | W7 |
| i18n | Front | W8 |
| Perf / release | Product + Full stack | W9 |

Cross-ref: [04-findings](04-findings.md) · [06-graph-studio-spec](06-graph-studio-spec.md).
