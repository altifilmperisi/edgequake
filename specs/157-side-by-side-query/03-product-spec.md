# 03 — Product specification

Parent: [README](README.md) · Surfaces: [02](02-surfaces.md) · UX: [04](04-ux-ui-spec.md)

## Problem statement

EdgeQuake Query answers with citations and entity context, but verifying either
forces a full navigation away from the chat. Analysts lose scroll position,
composer focus, and conversational continuity — so verification is skipped.

## Goals

1. Open a cited source **beside** chat (PDF page + passage, or text lines).
2. Open the answer’s **subgraph** beside chat without embedding full Graph Studio.
3. Keep chat primary; make companion state shareable via URL.
4. Preserve escape hatches to full document and Graph Studio pages.
5. Ship with e2e coverage for every registered edge case.

## Non-goals

- Simultaneous Chat + PDF + Graph (three panes).
- Embedding Graph Studio chrome (entity browser, full filters, export).
- Claim-level entailment re-ranking pass.
- Server-synced layout preferences (localStorage is enough for v1).
- Replacing `/documents/[id]` or `/graph` routes.

## Personas

| Persona | Need |
|---------|------|
| Analyst | Verify a claim against the PDF while iterating on follow-up questions |
| Knowledge engineer | Inspect which entities grounded an answer before editing the graph |
| Support / demos | Share a URL that restores chat + evidence pane |
| Keyboard / AT user | Open, switch, close pane without a pointer |

## User stories

| ID | Story | Acceptance |
|----|-------|------------|
| US-157-01 | As an analyst, when I click a citation chip, I see the source beside chat on the cited page/passage | Same-tab click opens Source pane; chat messages remain mounted; page/chunk match citation |
| US-157-02 | As an analyst, when I click “Show on graph”, I see answer entities beside chat | Graph pane opens with focused nodes; no navigation to `/graph` unless I choose Studio |
| US-157-03 | As an analyst, I can switch between Source and Graph tabs without losing either target | Last source location and last messageId retained while switching |
| US-157-04 | As an analyst, I can resize or close the companion | Width persists; close clears URL pane params; Esc returns focus to trigger |
| US-157-05 | As an analyst on a laptop, history does not steal width when companion is open | History auto-rails at constrained widths |
| US-157-06 | As a mobile user, companion opens as a sheet | Bottom sheet ~62vh; chat remains underneath; dismiss restores focus |
| US-157-07 | As a sharer, I can copy the URL and restore the pane | `/query?pane=…` restores kind + target after refresh |
| US-157-08 | As a power user, Cmd/Ctrl/middle-click still opens full page | Modifier behaviour unchanged from today |
| US-157-09 | As a knowledge engineer, I can expand neighbours and jump to Studio | Expand adds 1-hop; “Open in Graph Studio” uses existing deep link |
| US-157-10 | As an analyst, opening a source can scope the next question to that document | W4: open doc becomes optional `@` scope chip |

## Modes

```text
  CompanionKind
    none   — no pane (default)
    pdf    — Source pane (PDF or text document; name kept for URL compat)
    graph  — Answer subgraph pane
```

Only **one** kind is active. Switching tabs changes `kind` but may retain the
inactive target in store for instant restore.

## Entry points

| Trigger | Primary action | Escape hatch |
|---------|----------------|--------------|
| Citation chip click | `openSource` | Modifier → new tab; “Open full page” in pane |
| Sources panel doc / passage | `openSource` | Same |
| “Show on graph” | `openAnswerGraph` | “Open in Graph Studio” |
| Header layout toggle | Toggle last kind / close | — |
| Keyboard `Cmd+\` (configurable) | Toggle companion | — |
| Deep link | Hydrate from URL | Invalid → closed |

## Acceptance criteria (product)

1. From a happy-path mocked chat with citations, same-tab citation click does
   **not** change `location.pathname` away from `/query`.
2. Source pane shows the correct page (PDF) or line range (text) within 2s of
   mock load.
3. “Show on graph” does not navigate to `/graph`; Graph pane shows ≥1 focused
   node when context has entities.
4. Closing the pane removes `pane` from the URL.
5. Refresh with `?pane=pdf&doc=…&page=…` restores Source.
6. Viewport &lt; 768px uses sheet; ≥1280px uses side pane.
7. Locale keys exist in en/fr/zh for all new chrome.
8. Every EC-157 in [08](08-edge-cases.md) has a gate in [10](10-e2e-test-matrix.md).

## Metrics / KPIs

| KPI | Target |
|-----|--------|
| Same-tab citation stays on `/query` | 100% in e2e |
| Time to visible page after citation (mocked) | &lt; 2s |
| `/graph` store clobber after Query round-trip | 0 failures |
| axe serious/critical on `/query` with pane open | 0 |
| Locale parity for `query.companion.*` | 100% |

## Rollout

- Feature flag `ENABLE_QUERY_COMPANION` (default on in dev; staged in prod).
- When flag off: restore today’s `router.push` behaviour (no behaviour change).
- Docs: Query help tip + CHANGELOG entry in W6.

## Prioritisation

1. W0–W2: Source beside chat (highest user pain).
2. W3: Graph beside chat (closes LAW-155-11 in-place gap).
3. W4: AI niceties (scope chip, quote-to-ask).
4. W5–W6: Hardening and release.

Cross-ref: [00-why](00-why.md) · [04-ux-ui-spec](04-ux-ui-spec.md) ·
[lenses/LENS-product-owner.md](lenses/LENS-product-owner.md).
