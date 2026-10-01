# 01 — First Principles (SPEC-155)

Parent: [README](README.md) · WHY: [00-why](00-why.md) · Next: [02-surfaces](02-surfaces.md)

## Axioms

1. **Clarity beats ornament.** Gradients, glow, and sub-12px chrome are noise.
2. **The mental map is sacred.** A graph that re-layouts on every filter trains
   distrust.
3. **A control that does nothing is a bug.** Dead filters and cosmetic selectors
   destroy product credibility.
4. **State must be honest.** Loading / empty / error / stale / offline are
   distinct; never show “0” as success when the query failed.
5. **Language and accessibility are data contracts**, not polish tickets.
6. **Server owns graph truth.** Client heuristics must not contradict API
   communities, degrees, or totals.
7. **Evidence beats vibes.** Every law maps to a named gate (LAW-155-14).

## Laws

| Law | Statement |
|-----|-----------|
| **LAW-155-1** | One token source — semantic CSS variables (`--success/--warning/--info/--danger`, surfaces, graph) via Tailwind v4 `@theme inline`; raw palette classes outside an allow-list are forbidden. |
| **LAW-155-2** | One page shell — every authenticated route uses `PageShell` + `PageHeader` (title, description, primary action, breadcrumbs slot); no snowflake H1 sizes or containers. |
| **LAW-155-3** | Stable mental map — Sigma/graphology live in one `GraphEngine`; data deltas apply via `applyDelta`; theme/filter/search **must not** tear down the renderer; positions persist per workspace. |
| **LAW-155-4** | Dim, do not hide — focus, hover, search, and type filters dim non-matches; layout stays; hide-only is reserved for explicit prune/LOD. |
| **LAW-155-5** | Progressive disclosure — semantic zoom with ≥3 LOD tiers; hubs/labels by salience; community hulls before node hairballs; expand/collapse on demand. |
| **LAW-155-6** | No control without effect — every visible control writes a documented state that changes data or presentation; dead UI is removed or wired in the same wave. |
| **LAW-155-7** | Honest state — every data surface defines loading, empty, error, partial, stale, and offline; errors are humanised (no raw `error.message` / WebGL stacks). |
| **LAW-155-8** | Keyboard & AT parity — WCAG 2.2 AA; no global preventDefault of Tab/arrows/Enter; canvas uses roving focus; graph has a “table alternative”; reduced-motion and forced-colors respected. |
| **LAW-155-9** | Language is data — en/fr/zh parity; Settings languages ⊆ locale files; `<html lang/dir>` follow i18n; `Intl` + date-fns locales; no hard-coded English on user-facing surfaces. |
| **LAW-155-10** | Server owns graph truth — workspace-scoped totals, consistent degree (in/out/total), `community_id`, edge ids, truncation flags come from API; client Louvain is not SSOT. |
| **LAW-155-11** | Answer ↔ graph linkage — query/chat `subgraph` drives a highlight layer; messages expose “Show on graph”; entity ids map to graph node ids. |
| **LAW-155-12** | Measured performance — filter toggle &lt; 50ms (no rebuild); pan 60fps at 2k nodes; INP &lt; 200ms on primary paths; budgets enforced in CI. |
| **LAW-155-13** | Minimal visual language — neutral + one accent; ≥12px UI text; ≤2 radius roles; ≤3 shadows; no decorative gradients/glow on product chrome; focus ring is one SSOT. |
| **LAW-155-14** | CI is proof — every EC-155 has a named test (vitest / Playwright axe|visual|mocked / cargo e2e); green suite is DoD for each wave. |

## DRY / SOLID

| Principle | Application |
|-----------|-------------|
| **DRY** | One `StatusBadge`, one `formatCost`/`formatDuration`/`formatNumber`, one filter pipeline for the graph, one keyboard shortcuts mount, one connection indicator, one token resolver for canvas + CSS. |
| **SRP** | `GraphEngine` owns WebGL lifecycle; store owns data; React views own composition; `PageShell` owns chrome; screens own job-to-be-done only. |
| **OCP** | New graph modes (ego, path, answer-highlight) register as focus strategies without rewriting Sigma init. |
| **LSP** | Any surface claiming “healthy” / “error” / “empty” accepts the shared state primitives. |
| **ISP** | Graph store slices: data, selection, filters, view, streaming — consumers subscribe narrowly (selectors / `useShallow`). |
| **DIP** | Screens depend on `PageShell`, API clients, and engine interfaces — not on Sigma constructors or raw hex tables. |

## Visual language (normative sketch)

```text
  Neutrals (oklch)     Accent (one)        Status
  ---------------      ------------        ------
  bg / fg / muted      primary             success / warning / info / danger
  card / border        ring (= accent)     chart-1..5 retained for data only

  Type floor: 12px     Radius: sm | lg     Shadow: xs | sm | md
  Motion: 0 | 150 | 200 | 300ms; prefers-reduced-motion → 0 essential only
```

## GraphEngine contract (normative sketch)

```text
  React view  --props/events-->  GraphEngine
                                   |
                                   +-- graphology MultiGraph
                                   +-- Sigma instance (long-lived)
                                   +-- layout worker (FA2)
                                   +-- applyDelta(nodes+, edges+, remove ids)
                                   +-- setFocus({ mode, ids })
                                   +-- setTheme(tokens)
                                   +-- exportImage({ png|svg })

  Forbidden: unmount Sigma on filter; new Graph() per keystroke; whole-store
  subscription for hover.
```

## Relationship to prior specs

| Prior | Retained | Reopened |
|-------|----------|----------|
| SPEC-099 Documents | Status SSOT, feedback zone, toolbar | SRP split of manager still open |
| SPEC-029 / 030 | Sidebar groups, skip link, contrast tokens | PD-01 settings nav, PD-02 query modes, PD-04 welcome |
| SPEC-048 | Progress contract | Stage i18n, stuck heuristic |
| SPEC-032 / 102 | Graph routes, entity colours | Client Louvain vs server communities |
| SPEC-100 / 101 | CLS / wizard (`100-cls-dashboard-stability`, `101-wizard-mode-tenant-workspace`) | Keep green |
| SPEC-102 | Entity type colours | Tokens + overrides |
| SPEC-154 | Auth bar | Out of scope except login a11y |

## Residual / deferred

| Item | Trigger |
|------|---------|
| Cosmograph / GPU 100k+ | Product requires &gt;10k interactive nodes with LOD exhausted |
| Full RTL locale | Product ships Arabic/Hebrew |
| Server layout coords | Multi-client shared canvas |
| Merge suggestions / graph diff | Knowledge-ops product track |
| WCAG 3.0 conformance | W3C Recommendation (not before years; draft Sep 2026) |

Cross-ref: [03-standards-crosswalk](03-standards-crosswalk.md) · [10-implementation-plan](10-implementation-plan.md).
