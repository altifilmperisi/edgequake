# 11 — Cross-reference matrix

Parent: [README](README.md)

## Law ↔ WHY ↔ Finding ↔ EC ↔ Wave ↔ Test ↔ Lens

| Law | WHY | Findings | ECs | Wave | Primary gate(s) | Primary lens |
|-----|-----|----------|-----|------|-----------------|--------------|
| LAW-157-1 | WHY-1 | F-157-01,02 | 01,06,09,22,26 | W2–W3 | `companion_citation_idempotent`, `companion_modifier_newtab`, `companion_flag_off` | Product / AI |
| LAW-157-2 | WHY-2 | F-157-03 | 09,10 | W1–W2 | `companion_stream_stable`, `companion_conversation_switch` | UX / Front |
| LAW-157-3 | WHY-2 | F-157-03 | 13,14 | W1 | `companion_url_roundtrip`, `companion_storage_migrate` | Front / Full stack |
| LAW-157-4 | WHY-3 | F-157-01 | 06,22,26 | W2 | `useOpenSource` unit + source e2e | Full stack |
| LAW-157-5 | WHY-3 | F-157-04 | 01,04 | W0–W2 | document parity + `companion_pdf_no_remount` | Front |
| LAW-157-6 | WHY-5 | F-157-07 | 11,12,28,29 | W1 | `companion_width_rail`, `companion_mobile_sheet` | UX / Front |
| LAW-157-7 | WHY-5 | — | 21,23 | W1/W5 | `companion_a11y_keyboard`, `companion_esc_focus` | UX |
| LAW-157-8 | WHY-4 | F-157-02,06 | 15–17,32 | W3 | `companion_graph_*` | AI / Product |
| LAW-157-9 | WHY-4 | F-157-06 | 19,20 | W3 | `companion_graph_isolation`, `companion_graph_webgl_fallback` | Front / Full stack |
| LAW-157-10 | WHY-3 | F-157-05,09 | 18,27 | W3/W5 | `companion_graph_name_fallback`, `companion_nonstream_entities` | AI / Database |
| LAW-157-11 | — | F-157-08 | 02,03,05,08,15 | W2–W3 | error/empty gates | UX / Full stack |
| LAW-157-12 | — | F-157-04 | 04,06 | W2 | `companion_pdf_no_remount`, idempotent | Front |
| LAW-157-13 | — | — | 24,25 | W5 | `companion_theme_motion`, `companion_locale_parity` | Front / UX |
| LAW-157-14 | WHY-5 | — | all | W0–W6 | [10-e2e-test-matrix](10-e2e-test-matrix.md) | Product / Full stack |

## Findings index

| ID | Summary | Docs |
|----|---------|------|
| F-157-01 | Citation navigates away | [02](02-surfaces.md) |
| F-157-02 | Show-on-graph navigates away | [02](02-surfaces.md) |
| F-157-03 | No companion slot | [02](02-surfaces.md) |
| F-157-04 | Location logic trapped in page | [02](02-surfaces.md) · [05](05-frontend-architecture.md) |
| F-157-05 | Duplicate answer stores | [02](02-surfaces.md) · [05](05-frontend-architecture.md) |
| F-157-06 | Global graph store hazard | [02](02-surfaces.md) · [05](05-frontend-architecture.md) |
| F-157-07 | Width budget unaccounted | [02](02-surfaces.md) · [04](04-ux-ui-spec.md) |
| F-157-08 | Non-stream subgraph null | [02](02-surfaces.md) · [06](06-data-and-db-contract.md) |
| F-157-09 | Entity id weak after reload | [02](02-surfaces.md) · [06](06-data-and-db-contract.md) |

## Prior specs

| Prior | Topic | SPEC-157 link |
|-------|-------|---------------|
| [SPEC-155](../155-improve-ux-ui/) | UX overhaul; answer-on-graph deep link | Inherits; extends LAW-155-11 in-place |
| [SPEC-033](../033-page-lineage/) | Page deeplinks | Reuse page params in companion URL |
| [SPEC-143](../143-view-pdf-markdown-sync-view/) | PDF↔MD page sync | Optional inside Source |
| [SPEC-099](../099-ux-ui-improvement/) | Documents UX | Not reopened |
| [SPEC-100](../100-cls-dashboard-stability/) | CLS / flex chain | Companion must keep green |
| [SPEC-032](../032-graph/) | Graph | Studio remains full-page escape |
| SPEC-002 (code FEAT tags) | Document viewer (historical) | Source reuses `PDFViewer` / split view |
| [SPEC-154](../154-sec-hardening/) | Pack shape peer | Auth PDF paths respected |
| [SPEC-156](../156-ingestion-first-principles/) | Ingestion peer | Out of UI scope |

## Document map

| Doc | Role |
|-----|------|
| [README](README.md) | Entry, locked decisions |
| [00-why](00-why.md) | 5-WHY + causal ASCII |
| [01-first-principles](01-first-principles.md) | LAW-157-* |
| [02-surfaces](02-surfaces.md) | Code map + findings |
| [03-product-spec](03-product-spec.md) | Stories / acceptance |
| [04-ux-ui-spec](04-ux-ui-spec.md) | Wireframes / a11y |
| [05-frontend-architecture](05-frontend-architecture.md) | Modules / SOLID |
| [06-data-and-db-contract](06-data-and-db-contract.md) | Fields / optional API |
| [07-ai-engineer-spec](07-ai-engineer-spec.md) | Grounding / W4 |
| [08-edge-cases](08-edge-cases.md) | EC-157-* |
| [09-implementation-plan](09-implementation-plan.md) | Waves W0–W6 |
| [10-e2e-test-matrix](10-e2e-test-matrix.md) | Gates |
| [11-cross-ref](11-cross-ref.md) | This file |
| [lenses/](lenses/) | Role views |

## User stories ↔ waves

| Story | Wave |
|-------|------|
| US-157-01 Citation beside chat | W2 |
| US-157-02 Show on graph beside chat | W3 |
| US-157-03 Tab switch retain targets | W1–W3 |
| US-157-04 Resize / close / Esc | W1 |
| US-157-05 History auto-rail | W1 |
| US-157-06 Mobile sheet | W1 |
| US-157-07 Shareable URL | W1 |
| US-157-08 Modifier new tab | W2 |
| US-157-09 Expand + Studio | W3 |
| US-157-10 Scope chip | W4 |

## Coverage checklist

- [x] Every LAW-157-1…14 appears above
- [x] Every F-157-01…09 appears above
- [x] Every EC-157-01…32 appears in [08](08-edge-cases.md) and [10](10-e2e-test-matrix.md)
- [x] Lenses cover Product, Full stack, Database, UX/UI, Front, AI

Validate locally:

```bash
python3 specs/157-side-by-side-query/scripts/validate-cross-ref.py
```
