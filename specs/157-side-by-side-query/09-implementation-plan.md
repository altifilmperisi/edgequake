# 09 — Implementation plan

Parent: [README](README.md) · Architecture: [05](05-frontend-architecture.md) ·
ECs: [08](08-edge-cases.md)

## Principles

- Tests first for pure modules; behaviour parity when extracting.
- Small PRs per wave; flag-gated.
- DRY/SOLID as in [01](01-first-principles.md).
- Do not edit unrelated SPEC-155 Documents workspace code.

## Wave overview

```text
  W0 Foundations     extract DocumentSourceView + companion pure model + flag
  W1 Shell           CompanionShell, breakpoints, history rail, URL sync
  W2 Source mode     useOpenSource, citation wiring, PDF/text panes
  W3 Graph mode      merge stores, EmbeddedAnswerGraph, Show on graph
  W4 AI affordances  scope chip, quote-to-ask
  W5 Hardening       EC matrix, a11y, i18n, optional API contract
  W6 Docs / release  CHANGELOG, help, gates green
```

## W0 — Foundations

**Files**

- New: `lib/query/companion-pane.ts` + `__tests__/companion-pane.test.ts`
- New: `hooks/use-document-location.ts`
- New: `components/documents/document-source-view.tsx`
- Modify: `app/(dashboard)/documents/[id]/page.tsx` to consume extract
- New: feature flag helper / `NEXT_PUBLIC_ENABLE_QUERY_COMPANION`

**DoD**

- [ ] Document detail page behaviour parity (existing e2e `document-detail` /
      `document-viewer` still pass).
- [ ] Pure URL encode/decode/clamp tests green.
- [ ] Flag defaults documented.

**Rollback:** revert extract; page self-contained again.

## W1 — Companion shell

**Files**

- New: `stores/use-companion-pane-store.ts`
- New: `components/query/companion/companion-shell.tsx`, `companion-tabs.tsx`
- Modify: `query-interface.tsx` layout
- Modify: history panel auto-rail coordination
- E2E: `e2e/spec157/companion-layout.spec.ts`

**DoD**

- [ ] Desktop splitter resizes; width persists.
- [ ] Mobile sheet; breakpoint cross keeps kind/source.
- [ ] URL sync open/close (EC-157-13).
- [ ] Flag off: no companion chrome (EC-157-26).
- [ ] EC-157-11,12,14,21 (partial),28,29 gates.

**Rollback:** flag off or remove shell mount.

## W2 — Source (PDF / text)

**Files**

- New: `hooks/use-open-source.ts`
- New: `components/query/companion/source-pane.tsx`
- Modify: `citation-popover.tsx`, citation tabs / passage-row wiring
- E2E: `e2e/spec157/companion-source.spec.ts` (reuse `mock-pdf`, chat SSE)

**DoD**

- [ ] Same-tab citation stays on `/query` and shows page/passage.
- [ ] Text doc path (EC-157-01).
- [ ] 404 / idempotent / stream stable / modifier new-tab.
- [ ] “Open full page” escape hatch works.

**Rollback:** flag off restores `router.push`.

## W3 — Graph pane

**Files**

- New: `lib/query/answer-graph.ts` + tests
- Merge: answer stores → single SSOT; delete unused twin
- New: `components/graph/embedded-answer-graph.tsx`
- New: `components/query/companion/graph-pane.tsx`
- Modify: `assistant-message.tsx` Show on graph
- E2E: `e2e/spec157/companion-graph.spec.ts` + isolation visit `/graph`

**DoD**

- [ ] Show on graph opens pane (no pathname `/graph`).
- [ ] Empty / one / cap / name fallback / WebGL fallback / bypass.
- [ ] Isolation: `/graph` state intact after Query companion session.
- [ ] “Open in Graph Studio” deep link preserved.

**Rollback:** flag off; Show on graph → legacy navigation.

## W4 — AI affordances

**Files**

- Scope chip integration with `useQueryScope`
- Quote-to-ask UI on Source selection
- E2E: scope lifecycle + quote disabled without text layer

**DoD**

- [ ] EC-157-30,31.
- [ ] No auto-submit of quoted text.
- [ ] Auto scope ≠ user pin.

**Rollback:** hide W4 UI behind subflag if needed.

## W5 — Hardening

**Work**

- Complete EC matrix P1s.
- axe on `/query` with both panes.
- `bun run test:locale-parity`.
- `bun run test:perf-budget` / CLS smoke.
- Optional: non-stream `subgraph` API parity + `spec027` if touched.

**DoD**

- [ ] All P0/P1 ECs gated green.
- [ ] Locale keys complete.
- [ ] No serious axe violations.

## W6 — Docs / release

**Work**

- CHANGELOG entry.
- Short Query help / empty-state tip.
- `.env.example` if env flag used.
- Mark SPEC-157 status → Implementation complete (or partial by wave).

**DoD**

- [ ] Release notes mention companion + flag.
- [ ] Cross-ref validator green.

## Dependency graph

```text
  W0 ──► W1 ──► W2 ──► W3 ──► W4
                │       │
                └───────┴──► W5 ──► W6
```

W3 can start after W1 if Graph tab is stubbed; production Graph requires W2
only for tab switching polish, not data.

## Effort sketch (engineering days)

| Wave | Rough |
|------|-------|
| W0 | 1–2 |
| W1 | 2–3 |
| W2 | 2–3 |
| W3 | 3–4 |
| W4 | 1–2 |
| W5 | 2 |
| W6 | 0.5 |

## Definition of done (spec pack → product)

Feature flag may ship on when:

1. W0–W3 DoD met.
2. All P0 ECs green.
3. Locale parity + axe green for companion chrome.
4. Product acceptance in [03](03-product-spec.md) checked.

Cross-ref: [10-e2e-test-matrix](10-e2e-test-matrix.md) ·
[lenses/LENS-full-stack.md](lenses/LENS-full-stack.md).
