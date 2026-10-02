# 01 — First Principles (SPEC-157)

Parent: [README](README.md) · WHY: [00-why](00-why.md) · Next: [02-surfaces](02-surfaces.md)

## Axioms

1. **Verification must cost seconds.** If checking a citation requires a new
   route, users stop verifying (industry consensus: Multigrid, source-anchoring).
2. **Chat is the primary durable surface.** Companion panes assist; they never
   unmount the conversation or composer.
3. **Evidence is local to the answer.** The graph pane shows what supported
   *this* answer — not the whole knowledge graph.
4. **One intent, one entry point.** Citations, passages, and “Show on graph”
   must not invent parallel navigation paths.
5. **State is addressable.** Anything the user can see beside chat must be
   representable in the URL and restore after refresh.
6. **Reuse before invent.** PDF, markdown, GraphRenderer, and resizable panels
   already exist; extract shared views — do not fork.
7. **Evidence beats vibes.** Every law maps to a named gate (LAW-157-14).

## Laws

| Law | Statement |
|-----|-----------|
| **LAW-157-1** | Verify in place — primary citation / show-on-graph actions open the companion pane on `/query`; they must not navigate away unless the user chooses an escape hatch (modifier click, “Open full page”, “Open in Graph Studio”). |
| **LAW-157-2** | Chat is never displaced — opening, resizing, or closing the companion must not unmount messages, composer, or streaming session; scroll stick-to-bottom is preserved when the user is at the bottom. |
| **LAW-157-3** | URL is addressable state — companion kind and target (`pane`, `doc`, `page`, `chunk`, `msg`) live in the query string; back/forward and refresh restore; invalid params sanitise to `pane` absent / closed. |
| **LAW-157-4** | One entry point per intent — `openSource(chunk\|location)` and `openAnswerGraph(messageId)` are the only write paths for companion open; citation chip, tabs, and passage rows call them (DRY). |
| **LAW-157-5** | Extract, do not copy — document page/chunk/page resolution and PDF/text rendering live in shared modules (`DocumentSourceView`, `useDocumentLocation`); Query and `/documents/[id]` both consume them. |
| **LAW-157-6** | Width honesty — minimum chat 420px, companion 440px; when budget is exceeded, auto-rail history (and optionally collapse nav); below `md`, companion is a bottom sheet (~75vh). Persisted widths are clamped on viewport change. |
| **LAW-157-7** | Accessible splitter — WAI-ARIA window-splitter pattern: focusable separator, named panes, `aria-valuenow/min/max`, arrow-key resize (Home/End clamp); Esc closes companion and returns focus to the trigger. |
| **LAW-157-8** | Graph pane = answer evidence — render the selected answer’s subgraph (nodes + edges); neighbour expand is opt-in; full Studio remains `/graph` via escape hatch. |
| **LAW-157-9** | Isolate graph runtime — Query’s embedded graph must not write into the global Graph Studio singleton in a way that clobbers `/graph` after a round trip (scoped store / isolated engine instance). |
| **LAW-157-10** | One answer-graph SSOT — merge duplicate answer stores into a single module; mapping from `message.context` → focus ids lives in one pure helper with name-fallback. |
| **LAW-157-11** | Honest pane state — Source and Graph panes define loading, empty, error, partial (unresolved entities), and offline; humanised copy; no raw stack traces. |
| **LAW-157-12** | No remount on resize / same-doc navigate — changing page/chunk within the same document, or dragging the splitter, must not remount `PDFViewer` / WebGL canvas (controlled props only). |
| **LAW-157-13** | Language & motion — en/fr/zh keys for all companion chrome; `prefers-reduced-motion` disables non-essential transitions; type floor ≥12px (inherits LAW-155-9/13). |
| **LAW-157-14** | CI is proof — every EC-157 has a named vitest and/or Playwright gate; green suite is DoD for each wave. |

## DRY / SOLID

| Principle | Application |
|-----------|-------------|
| **DRY** | One `openSource`, one `openAnswerGraph`, one `DocumentSourceView`, one URL codec (`companion-pane.ts`), one answer-subgraph mapper, one width-clamp helper. |
| **SRP** | Pure model (`companion-pane.ts`) owns encode/decode/clamp; store owns UI persistence; shell owns layout; source/graph panes own their media; hooks own intent. |
| **OCP** | New companion kinds (future: video, lineage) register as `kind` handlers without rewriting `QueryInterface`. |
| **LSP** | Any surface that claims “open source” accepts the shared location type (`documentId`, optional `page`/`chunk`/`lines`). |
| **ISP** | Companion store slices: `kind`, `source`, `messageId`, `width`, `sheetOpen` — consumers subscribe narrowly. |
| **DIP** | Panes depend on `PDFViewer` / `GraphRenderer` interfaces and document APIs — not on Next.js route pages or `GraphViewer` chrome. |

## Relationship to prior laws

| Prior law | How SPEC-157 extends it |
|-----------|-------------------------|
| LAW-155-11 Answer ↔ graph | Keep deep link to Studio; **add** in-pane path as primary. |
| LAW-155-3 Stable mental map | Embedded graph must also avoid tear-down on filter/theme. |
| LAW-155-8 Keyboard & AT | Splitter + focus return on Query. |
| SPEC-033 / 143 page sync | Reuse `currentPage` / page markers inside companion Source. |
| SPEC-100 CLS | Companion open must not inflate document scrollHeight. |

## Normative module sketch

```text
  QueryInterface
       |
       +-- CompanionShell (react-resizable-panels | Sheet)
       |      +-- SourcePane --> DocumentSourceView --> PDFViewer | ContentRenderer
       |      +-- GraphPane  --> EmbeddedAnswerGraph --> GraphRenderer
       |
       +-- useOpenSource / openAnswerGraph  (intents)
       |
       +-- companion-pane.ts  (pure URL + state)
       +-- use-companion-pane-store (persist width / last kind)
```

## Residual / deferred

| Item | Trigger |
|------|---------|
| Three-pane Chat+PDF+Graph | Product requests simultaneous evidence + graph |
| Claim-level citation entailment pass | Trust / compliance track |
| Embed full Graph Studio | Never preferred; keep Studio on `/graph` |
| Backend schema for companion prefs | Only if server-synced layout is required |

Cross-ref: [05-frontend-architecture](05-frontend-architecture.md) ·
[09-implementation-plan](09-implementation-plan.md) · [08-edge-cases](08-edge-cases.md).
