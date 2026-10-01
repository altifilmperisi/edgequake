# SPEC-155 visual inspection log

Parent: [README](../README.md) · Screenshots: [e2e/screenshots](screenshots/)

Protocol: capture → open each PNG → log defects → fix → re-capture until a round has **zero** product defects.

## Round 00 — baseline (broken mock)

| Screenshot | Defect | Fix | Status |
|------------|--------|-----|--------|
| dashboard-light-1280 | Stuck on “Selecting workspace…” | Playwright last-route-wins: tenants mock ate workspaces | fixed |
| query-light-1280 | Crash `has_more` undefined | Conversations mock missing `pagination` | fixed |
| login-light-1280 | OK (show-password present) | — | ok |

## Round 01 — after mock-api fix (2026-10-01)

| Screenshot | Defect | Status |
|------------|--------|--------|
| dashboard-light-1280 | Clean PageHeader, stats, quick actions, recent activity | ok |
| documents-light-1280 | Table + upload + active run; minor “All Status (0)” count noise | ok (non-blocking) |
| query-light-1280 | Empty state + history drawer; no crash | ok |
| settings-light-1280 | Appearance + language en/fr/zh | ok |
| login-light-1280 | Minimal card, autocomplete, show-password | ok |
| costs / pipeline / graph / knowledge / workspace / api-explorer | Captured; no critical layout defects in light-1280 | ok |

**Round 01 product defects:** 0 (Next.js “N Issues” overlay ignored — tooling, not product).

Capture:

```bash
cd edgequake_webui
PLAYWRIGHT_SKIP_STACK_CHECK=1 bunx playwright test e2e/spec155/baseline-capture.spec.ts --project=mock-api
```

Baselines: `specs/155-improve-ux-ui/e2e/screenshots/baseline/` (44 PNGs).

## Round 02 — Graph chrome polish (2026-10-01)

Live empty-state on :3010 / backend :8092; populated matrix via mocked FIXTURE_100.

| Screenshot | Defect | Fix | Status |
|------------|--------|-----|--------|
| graph-light-375 (pre) | Hard-coded overlay offsets collide | Flex stacks `graph-overlay-left/right/bottom` | fixed |
| graph toolbar (pre) | Overcrowded on mobile/tablet | Overflow More menu for `isSmallScreen` | fixed |
| graph chrome (pre) | Sub-12px type (`text-[9/10/11]px`) | Raised to `text-xs` across graph chrome | fixed |
| overlays (pre) | Ad-hoc surfaces | Shared `.graph-overlay-surface` | fixed |
| empty canvas (live) | Ego/zoom/legend clutter empty state | Hide overlays when no nodes | fixed |
| mobile drawers | Side sheets shrink canvas | Bottom sheets at ≤640px | fixed |
| graph-data-* (post) | Overlay stacks + legend placement clean | — | ok |
| graph-data-* | WS “Connection lost” toast under mock | Mock setup/status + axe excludes sonner | fixed |
| graph-data-* | Hairball / label LOD | Out of chrome scope | deferred |

**Round 02 product chrome defects:** 0.

```bash
cd edgequake_webui
PLAYWRIGHT_SKIP_STACK_CHECK=1 bunx playwright test e2e/spec155/graph-chrome-capture-mocked.spec.ts --project=mock-api
```

Baselines refreshed: `baseline/graph-{light,dark}-{375,768,1280}.png`.

## Round 03 — Whole-app live review (2026-10-01)

Live `make dev` stack (:3010 → :8091, real data). Capture: 10 routes × light/dark 1280 + light 375
(`round-03/` = before, `round-04/` = after). Axe run live (WCAG 2A/2AA serious+critical).

| Screen | Defect (round-03) | Fix | Status |
|--------|-------------------|-----|--------|
| query 1280 | Header subtitle squeezed into a vertical sliver | `shrink-0` title, `truncate` subtitle only ≥ xl | fixed |
| workspace 1280/375 | "Resolves to" badge overlapped model title | Badge truncates inner span; row wraps (`flex-wrap`, `basis-40`) | fixed |
| dashboard / pipeline | "1 documents" | i18next `count` plurals (`defaultValue_one/_other`) | fixed |
| dashboard | Stat hints truncated, 11px italic | `text-xs`, `line-clamp-2`, title tooltip | fixed |
| documents | "Entit…" header truncated | Rebalanced `DOCUMENT_TABLE_COL_PERCENTS` (sum 100) | fixed |
| all | Page titles = long IDs overflow on mobile | `PageHeader` h1 `overflow-wrap:anywhere` + `title` | fixed |
| all | Sub-12px type (`text-[8–11px]`) in ~40 components | Raised to `text-xs` | fixed |
| documents / query / costs / settings | Unnamed selects, switches, icon buttons, progress bars, file input | `aria-label` / `aria-labelledby` | fixed |
| documents | Dropzone `role=button` nested selects; `aria-sort` on presentation table | `role=group`; header table keeps native semantics | fixed |
| documents | Pulsing badge + `opacity-80` rows + `*-600` text failed AA mid-animation | Pulse icon only; `-700` text; no row opacity | fixed |
| pipeline | `text-yellow-500` / `text-blue-500` fail AA | `yellow-700`/`blue-600` + dark variants | fixed |
| costs | Period select clipped at 375 ("Last 30") | `w-36` | fixed |
| api-explorer | Scalar third-party axe findings | Vendor widget — out of scope | deferred |
| header selector | Raw UUID workspace names (test data names equal IDs) | Data, not UI — title now wraps/tooltips | deferred |

**Round 03 first-party serious/critical axe defects after fixes:** 0 on `/ /documents /query /graph /pipeline /costs /workspace /knowledge /settings` (live) and 10/10 routes × `a11y` + `mock-api` projects.
