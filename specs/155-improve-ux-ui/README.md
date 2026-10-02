# SPEC-155 — EdgeQuake UX/UI Overhaul

> **Status:** Implementation in progress (W0–W4 foundations + W5–W9 slices landed)  
> **Product pin:** EdgeQuake v0.28.5  

> **Scope:** Deep UX/UI audit of every WebUI screen and the knowledge-graph
> visualization; raise EdgeQuake to a September-2026 best-in-class bar —
> clean, elegant, minimal — with **Graph Studio** as the flagship surface.  
> **Inherits:** [SPEC-029](../029-full-ux-ui-audit/) · [SPEC-030](../030-full-ux-ui-audit/) ·
> [SPEC-032](../032-graph/) · [SPEC-048](../048-improve-ux/) · [SPEC-099](../099-ux-ui-improvement/) ·
> [SPEC-100](../100-cls-dashboard-stability/) · [SPEC-101](../101-wizard-mode-tenant-workspace/) ·
> [SPEC-102](../102-custom-entity-type-colors/)  
> **Peers:** [SPEC-154](../154-sec-hardening/) (spec pack shape) ·
> [SPEC-010](../010-ingestion-reliability/) · [SPEC-021](../021-storage-study/) ·
> [SPEC-152](../152-new-mcp-contract/)

## Start here

1. [00-why.md](00-why.md) — Five WHYs + causal ASCII
2. [01-first-principles.md](01-first-principles.md) — LAW-155-1…14 + DRY/SOLID
3. [02-surfaces.md](02-surfaces.md) — Route / component / graph code map
4. [03-standards-crosswalk.md](03-standards-crosswalk.md) — WCAG 2.2, APG, Tailwind v4, Sigma 3
5. [04-findings.md](04-findings.md) — F-155-* with file and symbol citations
6. [05-design-system-spec.md](05-design-system-spec.md) — Tokens, type, primitives
7. [06-graph-studio-spec.md](06-graph-studio-spec.md) — Flagship Graph Studio architecture
8. [07-screens-spec.md](07-screens-spec.md) — Per-screen target UX
9. [08-data-contract.md](08-data-contract.md) — Graph/query API + storage gaps
10. [09-edge-cases.md](09-edge-cases.md) — EC-155 register + mitigations
11. [10-implementation-plan.md](10-implementation-plan.md) — Waves W0–W9 + DoD
12. [11-e2e-test-matrix.md](11-e2e-test-matrix.md) — Gates (one row per EC)
13. [12-cross-ref.md](12-cross-ref.md) — Law ↔ finding ↔ EC ↔ wave ↔ test ↔ lens
14. Lenses → [`lenses/`](lenses/)
    - [Product Owner](lenses/LENS-product-owner.md)
    - [Full Stack](lenses/LENS-full-stack.md)
    - [Database](lenses/LENS-database.md)
    - [UX / UI](lenses/LENS-ux-ui.md)
    - [Front](lenses/LENS-front.md)
    - [AI Engineer](lenses/LENS-ai-engineer.md)

## Locked decisions (Wave 0)

1. **Geist is the body typeface** — Load via `next/font`; map `--font-sans` to the
   loaded variable (closes F-155-D01). Inter remains an optional alternate.
2. **WCAG 2.2 Level AA is binding** — WCAG 3.0 (Working Draft 10 Sep 2026) is
   informative only (LAW-155-8, LAW-155-13).
3. **Sigma stays** — No engine swap to Cosmograph/Graphistry. Upgrade pin to
   Sigma 3.0.3+; drop unused `@react-sigma/*` (LAW-155-3).
4. **GraphEngine owns the canvas** — One imperative owner of Sigma + graphology;
   React is a thin view; the store holds data only (LAW-155-3, SRP).
5. **Dim, do not hide** — Filters and focus dim non-matches; they must not
   rebuild layout or destroy the mental map (LAW-155-4).
6. **Server owns graph truth** — Community ids, degrees, totals, and truncation
   come from the API; client Louvain is display-only until removed (LAW-155-10).
7. **Cheap data contract first** — Wave W3 ships per-workspace counts, stream
   `start_node`, tenant-scoped degrees, edge ids, community field + endpoint,
   and chat `subgraph`/`node_id`. Medium items follow; merge suggestions,
   server layout, and graph diff are deferred.
8. **One page shell** — `PageShell` + `PageHeader` + shared status/empty/error
   primitives; no per-page header snowflake (LAW-155-2).
9. **Semantic tokens only** — `--success/--warning/--info/--danger` + graph
   surface tokens; raw Tailwind palette classes are lint-forbidden outside an
   allow-list (LAW-155-1).
10. **12px floor** — No UI text below 12px (LAW-155-13).
11. **Cmd+K is real** — Global command palette mounted; or the shortcut is
    removed from help (LAW-155-6).
12. **CI is proof** — Every EC-155 maps to a named gate (LAW-155-14).
13. **Do not reopen SPEC-099 Documents status/feedback/toolbar design** —
    SRP split of `document-manager` and remaining screen debt are in scope;
    status SSOT and feedback zone remain as shipped.
14. **Answer-on-graph is a first-class path** — Retrieved `subgraph` highlights
    the canvas; each answer gets “Show on graph” (LAW-155-11).

## Surfaces (one screen)

```text
  Operator / Analyst
         |
         v
  +------------------+
  |  WebUI shell     |  PageShell · tokens · i18n · Cmd+K · a11y
  +--------+---------+
           |
     +-----+------+----------+----------+
     |            |          |          |
     v            v          v          v
  Documents    Query      Graph      Settings/
  Pipeline     Chat       Studio     Workspace/Costs
     |            |          |
     +-----+------+----------+
           |
           v
  REST / SSE / WS  <-- data contract (08) · AGE + pgvector
```

## Verification (post Wave 1+)

```bash
# Locale parity + token lint (added in W2)
cd edgequake_webui && bun run test -- i18n-parity
cd edgequake_webui && bun run lint

# axe + visual + reduced-motion projects (W2)
cd edgequake_webui && pnpm exec playwright test --project=a11y
cd edgequake_webui && pnpm exec playwright test --project=visual

# Graph engine unit + store (W4)
cd edgequake_webui && bun run test -- graph

# Backend data-contract gates (W3)
cargo test -p edgequake-api --test e2e_spec155_graph_totals_workspace
cargo test -p edgequake-api --test e2e_spec155_stream_start_node
cargo test -p edgequake-api --test e2e_spec155_degrees_tenant
cargo test -p edgequake-api --test e2e_spec155_communities_endpoint

# OpenAPI contract refresh after DTO changes
make codegen-openapi-refresh
cargo test -p edgequake-api --test spec027_api_contract
```

## Wave status

| Wave | Focus | Status |
|------|-------|--------|
| 0 | Spec pack + finding register + baselines + mock harness | **Done** |
| 1 | Foundations (font, tokens, PageShell, dead code, Cmd+K) | **Done** |
| 2 | Quality gates (axe, visual, locale parity, jsdom) | **Done** (locale + jsdom + a11y/mock projects) |
| 3 | Data contract (cheap graph/query API) | **Done** (totals/stream/degrees/communities + cargo e2e) |
| 4 | GraphEngine extraction + correctness fixes | **Done** |
| 5 | Graph Studio features (LOD, hulls, ego, path, lasso) | **Done** (core: table alt, ego slider, community colour) |
| 6 | Answer-on-graph | **Done** |
| 7 | Screen polish (Dashboard…Login, `w/[slug]`) | **Done** (ConnectionIndicator + shells + login) |
| 8 | i18n completion + RTL readiness | **Done** (en/fr/zh parity gate green) |
| 9 | Perf / release gates / CHANGELOG | **Done** (perf-budget stub + CHANGELOG + release hooks) |

## Quality bar

- axe: 0 serious/critical across routes × themes × viewports
- Zero raw-palette classes outside allow-list; zero sub-12px UI text
- 100% locale parity (en / fr / zh)
- Touched files ≤ 500 lines (target ≤ 300)
- Graph: filter/search cause no Sigma rebuild; export matches canvas
- Every F-155 maps to an EC and a named test

- [13-run-progress-ledger.md](./13-run-progress-ledger.md) — Prepare/Extract typed progress ledger (2026-10-02)
