# SPEC-157 — Side-by-side Query (Chat + PDF | Chat + Graph)

> **Status:** Implemented (W0–W5) — see [As built](#as-built-deviations-from-the-plan)  
> **Product pin:** EdgeQuake v0.28.5  
> **Scope:** Keep the Query conversation on screen while verifying sources
> (PDF / text) or answer entities (subgraph) in a single companion pane.  
> **Inherits:** [SPEC-155](../155-improve-ux-ui/) ·
> [SPEC-033](../033-page-lineage/) ·
> [SPEC-143](../143-view-pdf-markdown-sync-view/) ·
> [SPEC-099](../099-ux-ui-improvement/) ·
> [SPEC-100](../100-cls-dashboard-stability/) ·
> [SPEC-032](../032-graph/)  
> **Peers:** [SPEC-154](../154-sec-hardening/) (pack shape) ·
> [SPEC-156](../156-ingestion-first-principles/)

## Start here

1. [00-why.md](00-why.md) — Five WHYs + causal ASCII
2. [01-first-principles.md](01-first-principles.md) — LAW-157-1…14 + DRY/SOLID
3. [02-surfaces.md](02-surfaces.md) — Route / component / store / API map
4. [03-product-spec.md](03-product-spec.md) — Stories, modes, acceptance, non-goals
5. [04-ux-ui-spec.md](04-ux-ui-spec.md) — Wireframes, breakpoints, keyboard, a11y
6. [05-frontend-architecture.md](05-frontend-architecture.md) — Modules, stores, SOLID
7. [06-data-and-db-contract.md](06-data-and-db-contract.md) — Context fields + gaps
8. [07-ai-engineer-spec.md](07-ai-engineer-spec.md) — Scope, quote-to-ask, grounding
9. [08-edge-cases.md](08-edge-cases.md) — EC-157 register + mitigations
10. [09-implementation-plan.md](09-implementation-plan.md) — Waves W0–W6 + DoD
11. [10-e2e-test-matrix.md](10-e2e-test-matrix.md) — One gate per EC
12. [11-cross-ref.md](11-cross-ref.md) — Law ↔ WHY ↔ EC ↔ wave ↔ test ↔ lens
13. Lenses → [`lenses/`](lenses/)
    - [Product Owner](lenses/LENS-product-owner.md)
    - [Full Stack](lenses/LENS-full-stack.md)
    - [Database](lenses/LENS-database.md)
    - [UX / UI](lenses/LENS-ux-ui.md)
    - [Front](lenses/LENS-front.md)
    - [AI Engineer](lenses/LENS-ai-engineer.md)

## Locked decisions (Wave 0)

1. **One companion pane at a time** — `Chat | Source` **or** `Chat | Graph`.
   Three panes (Chat + PDF + Graph) are a non-goal and deferred.
2. **Graph pane = answer subgraph** — entities and relationships used for the
   selected answer, with neighbour expansion and “Open in Graph Studio”.
   Full Graph Studio is **not** embedded in Query.
3. **Verify in place** — citation click and “Show on graph” open the companion
   pane. Modifier / middle-click and “Open full page” keep today’s navigation.
4. **URL is addressable state** — `/query?pane=pdf|graph&…` survives refresh,
   share, and back/forward (LAW-157-3).
5. **Chat is never displaced** — companion may rail history and shrink chat, but
   must not unmount the composer or message list (LAW-157-2).
6. **One entry point per intent** — `useOpenSource` / `openAnswerGraph` replace
   duplicated `router.push` sites (LAW-157-4, DRY).
7. **Reuse, do not fork** — `PDFViewer`, `GraphRenderer`, `react-resizable-panels`,
   and `document-url` helpers. Extract `DocumentSourceView` from
   `documents/[id]/page.tsx` rather than copy (LAW-157-5).
8. **Isolate graph state** — Query’s embedded graph must not clobber
   `useGraphStore` for `/graph` (LAW-157-9).
9. **Unify answer stores** — merge `use-answer-graph-store` and
   `use-answer-on-graph-store` (LAW-157-10).
10. **CI is proof** — every EC-157 maps to a named gate (LAW-157-14).

## Job in one screen

```text
  Analyst asks → answer streams → citations appear
         |                              |
         |  click [3]                   |  "Show on graph"
         v                              v
  +------------------+  +------------------------+
  | Chat (primary)   |  | Companion (Source|Graph)|
  | messages+compose |  | PDF page / answer nodes |
  +------------------+  +------------------------+
         ^                       |
         |  Esc / Close          |  Open full page (escape hatch)
         +-----------------------+
```

## Non-goals (explicit)

- Three-pane Chat + PDF + Graph docking (defer; reuse SPEC-155 tree later).
- Embedding full Graph Studio (filters, entity browser, export chrome) in Query.
- Claim-level entailment re-pass (out of scope; cite existing retrieval context).
- Replacing `/documents/[id]` or `/graph` as full-page destinations.
- Reopening SPEC-099 Documents status/feedback design.
- New backend schema / migrations in the default path (contract-only optional W5).

## Success narrative

When Waves 0–6 land, an analyst can:

1. Ask a question on `/query` and click a citation — the PDF (or text) opens
   beside chat on the cited page with the passage highlighted.
2. Click “Show on graph” — the answer’s entities light in a compact graph pane
   without leaving the conversation.
3. Resize the companion, refresh, or share the URL — pane state restores.
4. Use keyboard and screen reader to open, switch, and close the pane with
   focus returning to the trigger.
5. Trust CI: vitest pure modules green, Playwright `@spec157` green, axe green,
   locale parity green.

## As built (deviations from the plan)

| Plan | As built | Why |
|------|----------|-----|
| Radix Sheet for narrow screens | Fixed bottom panel (`62vh`) inside `CompanionShell`; Esc closes from inside the pane | Chat must stay interactive underneath; no focus trap |
| Snapshot/restore of the workspace graph store (EC-157-19) | `GraphRenderer` `isolated` mode: selection comes from props and never touches the graph store's focus/ego/sigma | Isolation by construction beats restore |
| EC-157-18 name fallback via a dedicated lookup | `buildAnswerGraphModel` resolves names to ids from the persisted flat entities/relationships and adds `UNKNOWN` placeholders | Persisted messages carry names only |
| Enter on splitter collapses the pane | Dropped; arrows, Shift+arrows, Home/End resize and the close button closes | Collapse duplicated Close and confused the WAI-ARIA splitter value |
| Maximize companion (EC-157-28) | Not built; "Open full page" / "Open in Graph Studio" instead | Full-page routes already exist (non-goal to replace) |
| Flag | `localStorage edgequake.query.companion.enabled=0` or `NEXT_PUBLIC_QUERY_COMPANION=0` | Same rollback, no new backend setting |
| Max pane width 920 | Capped further by the width budget while history is docked (e.g. 584 at 1600px) | Chat >= 420px and history never squeeze each other |

Verified by `bun run test:e2e:spec157` (33 hermetic Playwright tests on the
`mock-api` project) and 44 vitest tests (`src/lib/query/__tests__`,
`viewer-target.test.ts`). Screenshots, reviewed visually, live in
[`e2e/screenshots/`](e2e/screenshots/):

| Shot | Scenario |
|------|----------|
| `03-source-pane-pdf` | Chat + PDF page 3 with passage strip |
| `04-source-pane-text` | Text source with highlighted passage |
| `05-source-missing` | Deleted source — honest empty state |
| `06-graph-pane-canvas` | Answer subgraph on the Sigma canvas |
| `07-graph-pane-list-selected` | Accessible list + node card |
| `08-graph-pane-expanded` | After "Show neighbours" (6 → 8 entities) |
| `09-graph-empty` | Stale message id |
| `10-layout-1280/1440/1920` | Width budget at three breakpoints |
| `11-layout-sheet` | Bottom sheet at 820px |
| `12-quote-to-ask`, `13-scope-to-document` | AI affordances |
| `14-dark-source`, `15-dark-graph` | Dark theme |

## Validate locally

```bash
python3 specs/157-side-by-side-query/scripts/validate-cross-ref.py
```

Cross-ref: [00-why](00-why.md) · [11-cross-ref](11-cross-ref.md).
