# Lens — Product Owner

Parent: [README](../README.md) · Plan: [10](../10-implementation-plan.md)

## Job to be done

Ship a product that feels **one quality bar**: minimal chrome, trustworthy
Graph Studio, honest data, accessible and internationalised — credible as a
September 2026 flagship RAG/KG UI.

## Outcomes by wave

| Wave | User-visible outcome |
|------|----------------------|
| W1 | App looks intentional (font, tokens); Cmd+K works; login/a11y basics |
| W2 | Quality is measurable (axe/visual CI) |
| W3 | Graph numbers / isolation trustworthy |
| W4–W5 | Graph Studio = market-leading exploration |
| W6 | “Why this answer?” on the graph |
| W7 | Every screen polished; Costs/pipeline honest |
| W8 | en/fr/zh complete |
| W9 | Release-ready performance story |

## Prioritisation rules

1. P0 honesty/security (B01–B03, G01–G05, Costs, w-slug) before polish.
2. Do not reopen SPEC-099 Documents status/feedback.
3. Defer Cosmograph, merge suggestions, server layout until metrics demand.
4. Graph Studio is the hero narrative for marketing — protect W4–W6.

## Acceptance narrative (demo script)

1. Upload → Documents progress (unchanged 099).
2. Query → answer → **Show on graph** → entities lit, layout stable.
3. Filter types / search — graph **does not jump**.
4. Export PNG looks like canvas.
5. Switch language to fr — chrome translates; `lang=fr`.
6. Tab through Graph Studio without trapping focus.

## Risks / comms

- Visual baseline churn → warn design reviewers early (W1/W2).
- W3 tenant totals may change displayed counts (honesty) — release note.
- Scope creep into “new features” — push to deferred list in [08](../08-data-contract.md).

## KPIs

- axe serious/critical = 0
- Locale parity = 100%
- Graph filter rebuild count = 0 in e2e
- Support tickets: “graph jumps” / “costs wrong” → trending down
