# 12 — Cross-reference matrix

Parent: [README](README.md)

## Law ↔ Finding ↔ EC ↔ Wave ↔ Test ↔ Lens

| Law | Findings | ECs | Wave | Primary gate(s) | Primary lens |
|-----|----------|-----|------|-----------------|--------------|
| LAW-155-1 | D02, D05, G13 | 41 | W1–W5 | `tokens_status_aa`, lint palette | Front / UX |
| LAW-155-2 | S03, P* headers | 47 | W1,W7 | `breadcrumb_paths` | UX / Front |
| LAW-155-3 | G01,G02,G07,G08,G11 | 11–14 | W4 | `graph_filter_no_rebuild` | Front / Full stack |
| LAW-155-4 | G14 | 16 | W5 | `graph_dim_hover` | UX / Front |
| LAW-155-5 | G12,G17 | 06,17–19 | W5 | `graph_ego`, LOD | UX / Front |
| LAW-155-6 | G03,G06,G16,S01,P01,P03 | 08,15,45,51,53 | W1,W4,W7 | truncation, cmdk, costs | Product / Full stack |
| LAW-155-7 | G18,S05,P05,P06,A02 | 27,49,55,56,75 | W1,W4,W7 | webgl fallback, errors | UX / Full stack |
| LAW-155-8 | G10,A01,P12,Q02 | 28,30,60,62,74,76 | W1–W2,W4,W7 | axe, keyboard, ime | UX / Front |
| LAW-155-9 | I01–I05,Q06 | 70–73 | W1,W8 | lang, locale_parity | Front |
| LAW-155-10 | B01–B08,G03,G05,G12 | 05,08,80–87 | W3–W4 | cargo + parallel edges | Database / Full stack |
| LAW-155-11 | Q05,B09 | 65 | W3,W6 | `answer_on_graph` | AI / Front |
| LAW-155-12 | G07,G11,S06 | 07 | W4,W9 | stream_2k, vitals | Full stack |
| LAW-155-13 | D01,D03,D04,Q04 | 40,42 | W1–W2 | font, type floor | Front / UX |
| LAW-155-14 | T01–T04 | 76,77 | W0,W2 | axe_routes, jsdom | Full stack / Product |

## Prior specs

| Prior | Topic | SPEC-155 link |
|-------|-------|---------------|
| [SPEC-029](../029-full-ux-ui-audit/) | Full UX audit roadmap | Open: PD-01/02/04; closed items retained |
| [SPEC-030](../030-full-ux-ui-audit/) | Follow-on UX | Graph toolbar still open → W5 |
| [SPEC-032](../032-graph/) | Graph backend | W3–W5; lineage UI dead → purge |
| [SPEC-048](../048-improve-ux/) | Ingestion UX | Progress retained; stage i18n → W8 |
| [SPEC-099](../099-ux-ui-improvement/) | Documents | Status/feedback **not** reopened; SRP → W7 |
| [SPEC-100](../100-cls-dashboard-stability/) | CLS | Keep green |
| [SPEC-101](../101-wizard-mode-tenant-workspace/) | Wizard | Keep green; welcome dashboard PD-04 → W7 |
| [SPEC-102](../102-custom-entity-type-colors/) | Colours | Tokens + overrides → W1/W5 |
| [SPEC-010](../010-ingestion-reliability/) | Ingestion reliability | Progress UX retained |
| [SPEC-021](../021-storage-study/) | Storage / stream conventions | Graph stream retry retained |
| [SPEC-152](../152-new-mcp-contract/) | MCP | Out of UI scope |
| [SPEC-153](../153-workload-benchmark/) | Perf | Informs W9 budgets |
| [SPEC-154](../154-sec-hardening/) | Auth | Login a11y only; pack shape peer |

## Code symbols index

| Symbol / file                               | Findings      |
| ---------------------------------------------| ---------------|
| `graph-renderer.tsx` init deps              | G01, G08      |
| `graph-viewer.tsx` truncation / filters     | G02, G03, G06 |
| `graph-export.tsx`                          | G04           |
| `new Graph()`                               | G05           |
| `use-graph-keyboard-navigation.ts`          | G10           |
| `camera-utils.ts` / `graph-minimap.tsx`     | G09           |
| `lib/graph/layouts.ts`                      | G07           |
| `lib/graph/clustering.ts`                   | G12           |
| `globals.css` / `layout.tsx` fonts          | D01           |
| `use-keyboard-shortcuts.ts` searchOpen      | S01           |
| `costs/page.tsx`                            | P01           |
| `w/[slug]/layout.tsx`                       | P02           |
| `query-interface.tsx`                       | Q01, Q02      |
| `analytics_ops` reltuples / stream metadata | B01           |
| `graph_stream.rs`                           | B02           |
| `popular.rs` degrees batch                  | B03           |
| `community_persist` / no HTTP               | B07           |
| chat streaming `subgraph`                   | B09, Q05      |

## Document map

| Doc | Role |
|-----|------|
| [README](README.md) | Entry, locked decisions, waves |
| [00-why](00-why.md) | 5-WHY + causal ASCII |
| [01-first-principles](01-first-principles.md) | LAW-155-* |
| [02-surfaces](02-surfaces.md) | Code map |
| [03-standards-crosswalk](03-standards-crosswalk.md) | WCAG / APG / Tailwind / Sigma |
| [04-findings](04-findings.md) | F-155-* |
| [05-design-system-spec](05-design-system-spec.md) | Tokens & primitives |
| [06-graph-studio-spec](06-graph-studio-spec.md) | Flagship graph |
| [07-screens-spec](07-screens-spec.md) | Per-screen UX |
| [08-data-contract](08-data-contract.md) | API/storage |
| [09-edge-cases](09-edge-cases.md) | EC-155-* |
| [10-implementation-plan](10-implementation-plan.md) | Waves W0–W9 |
| [11-e2e-test-matrix](11-e2e-test-matrix.md) | Gates |
| [12-cross-ref](12-cross-ref.md) | This file |
| [lenses/](lenses/) | Role views |

## Coverage checklist

- [x] Every LAW-155-1…14 appears above
- [x] Every P0 finding maps to wave + EC
- [x] Every EC in [09](09-edge-cases.md) appears in [11](11-e2e-test-matrix.md)
- [x] Lenses cover Product, Full stack, Database, UX/UI, Front, AI

Validate locally:

```bash
python3 specs/155-improve-ux-ui/scripts/validate-cross-ref.py
```
