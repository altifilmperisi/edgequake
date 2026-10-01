# 04 — Findings register (F-155-*)

Parent: [README](README.md) · Laws: [01](01-first-principles.md) · Next: [05-design-system-spec](05-design-system-spec.md)

Severity: **P0** blocks trust / security / data honesty · **P1** major UX · **P2** polish.
Paths relative to `edgequake_webui/src/` unless noted `api:` for `edgequake-api`.

## Graph (G)

| ID | Sev | Summary | Citation | Law | EC | Wave |
|----|-----|---------|----------|-----|----|------|
| F-155-G01 | P0 | Sigma full teardown on nodes/edges/theme change; mental map lost | `components/graph/graph-renderer.tsx:728` deps | 3 | 10–14 | W4 |
| F-155-G02 | P0 | Filter search no debounce → rebuild per keystroke | `graph-filters.tsx:97-98`; viewer filter memo | 3,4 | 11 | W4 |
| F-155-G03 | P0 | Streaming truncation inverted; discards server totals | `graph-viewer.tsx:270-274` | 6,7,10 | 20 | W4 |
| F-155-G04 | P0 | PNG export captures WebGL edges canvas; SVG stub | `graph-export.tsx:28`; toast “coming soon” | 6 | 25 | W4 |
| F-155-G05 | P0 | `new Graph()` non-multi; parallel edges silently dropped | `graph-renderer.tsx:294` + catch ~387 | 10 | 15 | W4 |
| F-155-G06 | P1 | Time filter UI never applied in viewer | viewer `filteredNodes` ignores store time fields | 6 | 21 | W4 |
| F-155-G07 | P1 | Layout main-thread; no worker; e2e claims workers | `lib/graph/layouts.ts`; `e2e/graph-layouts.spec.ts` | 3,12 | 12 | W4 |
| F-155-G08 | P1 | Positions not persisted; stream/expand discarded by reinit | renderer init; `use-graph-expansion.ts` | 3 | 13 | W4 |
| F-155-G09 | P1 | Minimap + `normalizeGraphCoordinates` wrong space | `camera-utils.ts:92-108`; `graph-minimap.tsx:138-205` | 3 | 26 | W4 |
| F-155-G10 | P1 | Keyboard hijacks Tab/arrows/Enter; mutates store sort in place | `use-graph-keyboard-navigation.ts:56-58,168-196` | 8 | 30 | W4 |
| F-155-G11 | P1 | Whole-store subscriptions; hover re-renders viewer | `useGraphStore()` in viewer/export/details/… | 12 | 14 | W4 |
| F-155-G12 | P1 | Client Louvain ≠ server communities; legend type-only | `clustering.ts`; no community legend | 5,10 | 22 | W5 |
| F-155-G13 | P1 | Hard-coded hex colours; no colour-blind palette | `entity-type-colors.ts`, renderer, minimap | 1,8 | 31 | W5 |
| F-155-G14 | P1 | No dim-vs-hide; hover **hides** non-neighbours | `nodeReducer` `hidden: true` ~466 | 4 | 16 | W5 |
| F-155-G15 | P2 | Dead `@react-sigma/*`; unused curved program; dead store fields | package.json; graph-events unused | — | — | W4 |
| F-155-G16 | P2 | includeOrphans UI-only; depth not sent to stream | settings panel vs `graph.ts` | 6 | 21 | W4 |
| F-155-G17 | P1 | No LOD / hulls / path / ego / lasso / saved views with positions | absent | 5 | 17–19 | W5 |
| F-155-G18 | P0 | WebGL errors shown raw (`blendFunc`) | audit screenshot + fatal `window.location` | 7 | 27 | W4 |

## Design system (D)

| ID | Sev | Summary | Citation | Law | EC | Wave |
|----|-----|---------|----------|-----|----|------|
| F-155-D01 | P0 | Font never applied: `--font-geist-sans` undefined | `globals.css:11` vs `layout.tsx:26` | 13 | 40 | W1 |
| F-155-D02 | P0 | No semantic status tokens; 590 raw palette classes | grep across `src` | 1 | 41 | W1 |
| F-155-D03 | P1 | ~272 sub-12px text uses | `text-[8-11px]` | 13 | 42 | W1 |
| F-155-D04 | P1 | Four focus-ring styles; no z-index scale | button/sidebar/dialog/skip-link | 13 | 43 | W1 |
| F-155-D05 | P1 | Dead design-token utilities; duplicate keyframes | `design-tokens.css`, `globals.css` | 1 | — | W1 |
| F-155-D06 | P2 | Shared EmptyState/Skeletons/ResponsiveTable unused | `components/shared/*` | 2 | 44 | W1 |

## Shell (S)

| ID | Sev | Summary | Citation | Law | EC | Wave |
|----|-----|---------|----------|-----|----|------|
| F-155-S01 | P0 | Cmd+K advertised; no CommandDialog | `use-keyboard-shortcuts.ts` `searchOpen` | 6 | 45 | W1 |
| F-155-S02 | P1 | Keyboard hook mounted 3×; ⌘D/⌘G conflict browser | provider + both layouts | 8 | 46 | W1 |
| F-155-S03 | P1 | Breadcrumb missing pipeline/costs/knowledge/workspace | `dynamic-breadcrumb.tsx:31-38` | 2 | 47 | W1 |
| F-155-S04 | P1 | Mobile workspace selector hidden; toast lies | `header.tsx:99-105` | 7 | 48 | W7 |
| F-155-S05 | P1 | No `not-found.tsx`; error.tsx raw message English | `app/error.tsx` | 7,9 | 49 | W1 |
| F-155-S06 | P1 | I18nProvider null until hydrate; 95% client | `i18n-provider.tsx` | 12 | 50 | W9 |
| F-155-S07 | P2 | Sidebar/header version mismatch UI vs API | footer vs header | 7 | — | W7 |

## Pages (P)

| ID | Sev | Summary | Citation | Law | EC | Wave |
|----|-----|---------|----------|-----|----|------|
| F-155-P01 | P0 | Costs period cosmetic; BudgetIndicator null; zeros hard-coded | `costs/page.tsx:44-46,149-175` | 6,7 | 51 | W7 |
| F-155-P02 | P0 | `w/[slug]` missing AuthGuard/banner/error boundary; `push` trap | `w/[slug]/layout.tsx` | 7 | 52 | W7 |
| F-155-P03 | P1 | Dashboard `/documents?id=` deep link unread | `recent-activity.tsx:96` | 6 | 53 | W7 |
| F-155-P04 | P1 | Document manager still god file (1251; 34/32 props) | `document-manager.tsx` | SRP | 54 | W7 |
| F-155-P05 | P1 | Detail page mounts desktop+mobile trees; ErrorContent always 404 | `[id]/page.tsx` | 7,12 | 55 | W7 |
| F-155-P06 | P1 | Pipeline 5 pollers; no error; duplicates status dialog | `pipeline/*` | 7,12 | 56 | W7 |
| F-155-P07 | P1 | Settings flat stack; toast spam; Import not keyboard; ja/ko | `settings/page.tsx` | 6,8,9 | 57 | W7 |
| F-155-P08 | P1 | Knowledge detached poll; dropzone not keyboard; naming clash | `knowledge/page.tsx` | 7,8 | 58 | W7 |
| F-155-P09 | P1 | Workspace dead edit props; delete no typed confirm | `workspace/page.tsx` | 6 | 59 | W7 |
| F-155-P10 | P1 | ~3.5k dead lines (lineage/progress/v1 history) | import search | DRY | — | W1 |
| F-155-P11 | P2 | API Explorer title not h1; no i18n; no load error | `api-explorer-view` | 7,9 | — | W7 |
| F-155-P12 | P1 | Login no autocomplete; dual error toast; landing `/graph` | `login/page.tsx` | 8 | 60 | W1 |

## Query (Q)

| ID | Sev | Summary | Citation | Law | EC | Wave |
|----|-----|---------|----------|-----|----|------|
| F-155-Q01 | P0 | Textarea disabled while streaming | `query-interface.tsx:255` | 7 | 61 | W7 |
| F-155-Q02 | P0 | Enter-to-send no `isComposing` (zh/ja/ko) | `:249-254` | 8,9 | 62 | W7 |
| F-155-Q03 | P1 | `role="log" aria-live` on whole stream; duplicate banner | `:132-137,61-64` | 8 | 63 | W7 |
| F-155-Q04 | P1 | Gradient empty state / user bubble vs minimal bar | empty-state, chat-message | 13 | 64 | W7 |
| F-155-Q05 | P0 | `subgraph` unused for graph highlight | types present; no graph consumer | 11 | 65 | W6 |
| F-155-Q06 | P1 | source-citations 22× sub-12px; English aria-labels | `source-citations.tsx` | 9,13 | 42 | W7 |

## i18n (I)

| ID | Sev | Summary | Citation | Law | EC | Wave |
|----|-----|---------|----------|-----|----|------|
| F-155-I01 | P0 | Settings language ≠ i18n.changeLanguage; ja/ko empty | settings + `use-settings-store` | 9 | 70 | W1 |
| F-155-I02 | P0 | `<html lang="en">` static; no `dir` | `app/layout.tsx` | 9 | 71 | W1 |
| F-155-I03 | P1 | 687 keys missing from en.json; fr/zh −~90 | locales vs `t()` | 9 | 72 | W8 |
| F-155-I04 | P1 | Whole areas no useTranslation (pipeline/cost/login…) | grep | 9 | 72 | W8 |
| F-155-I05 | P1 | No Intl; date-fns without locale; 373 physical ml/mr | grep | 9 | 73 | W8 |

## Accessibility / tests (A / T)

| ID | Sev | Summary | Citation | Law | EC | Wave |
|----|-----|---------|----------|-----|----|------|
| F-155-A01 | P1 | prefers-contrast / forced-colors / motion-reduce:0 in TSX | grep | 8 | 74 | W2 |
| F-155-A02 | P1 | Connection/offline only on Documents | document-header | 7 | 75 | W7 |
| F-155-T01 | P0 | No axe, no toHaveScreenshot, Chrome-only | package / e2e | 14 | 76 | W2 |
| F-155-T02 | P1 | Vitest `environment: 'node'` — no component DOM tests | `vitest.config.mjs` | 14 | 77 | W2 |
| F-155-T03 | P1 | Stale graph e2e (workers / Start Animation) | graph-layouts.spec | 14 | 12 | W4 |
| F-155-T04 | P2 | audit_ui screenshots v0.12 vs product v0.28.5 | `audit_ui/` | 14 | — | W0 |

## Backend (B) — see also [08-data-contract](08-data-contract.md)

| ID | Sev | Summary | Citation | Law | EC | Wave |
|----|-----|---------|----------|-----|----|------|
| F-155-B01 | P0 | `total_nodes/edges` = shared AGE reltuples, not workspace | `api:analytics_ops` / stream metadata | 10 | 80 | W3 |
| F-155-B02 | P0 | `/graph/stream` ignores `start_node` | `graph_stream.rs` | 6,10 | 81 | W3 |
| F-155-B03 | P0 | `degrees/batch` no tenant scope | `popular.rs` | 10 | 82 | W3 |
| F-155-B04 | P1 | `start_node` graph path degree 0 | `traversal.rs` | 10 | 83 | W3 |
| F-155-B05 | P1 | Degree definition inconsistent (out vs in+out) | search vs popular SQL | 10 | 84 | W3 |
| F-155-B06 | P1 | Edge DTO missing id/description/keywords; synthetic id | relationships list | 10 | 85 | W3 |
| F-155-B07 | P1 | `community_id` persisted; no field / endpoint | community persist | 10 | 86 | W3 |
| F-155-B08 | P1 | No document_ids on /graph; lineage silent 5000 | scan_ops LIMIT | 10 | 87 | W3–med |
| F-155-B09 | P1 | Chat `done` omits rich stats; entity id `ent:{name}` | chat streaming / mapper | 11 | 65 | W3 |
| F-155-B10 | P2 | UI types expect metadata/created_at not sent | `types/graph.ts` | 10 | — | W3 |

## Finding → wave rollup

```text
  W0  T04
  W1  D01–D06, S01–S03,S05, P10,P12, I01–I02
  W2  A01, T01–T02
  W3  B01–B07,B09–B10 (+ B08 medium)
  W4  G01–G11,G15–G16,G18, T03
  W5  G12–G14,G17
  W6  Q05, B09 (UI half)
  W7  S04,S07, P01–P09,P11, Q01–Q04,Q06, A02
  W8  I03–I05
  W9  S06 + perf budgets
```

Cross-ref: [09-edge-cases](09-edge-cases.md) · [12-cross-ref](12-cross-ref.md).
