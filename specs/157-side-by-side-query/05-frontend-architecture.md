# 05 — Frontend architecture

Parent: [README](README.md) · Surfaces: [02](02-surfaces.md) · UX: [04](04-ux-ui-spec.md)

## Goals

- Implement companion panes with **DRY / SOLID**, small modules, and tests first.
- Reuse `PDFViewer`, `GraphRenderer`, resizable panels — extract shared document
  location logic from `documents/[id]/page.tsx`.
- Keep Query chat lifecycle (`useQueryInterface`, stream session) untouched
  except for wiring intents.

## Module map

```text
  lib/query/companion-pane.ts          PURE — encode/decode URL, clamp, migrate
  lib/query/answer-graph.ts            PURE — context → focus + SubgraphBundle
  stores/use-companion-pane-store.ts   UI — kind, source, messageId, width, trigger
  hooks/use-open-source.ts             INTENT — openSource / close / sync URL
  hooks/use-document-location.ts       EXTRACT — page/chunk resolution helpers
  components/documents/document-source-view.tsx   SHARED viewer
  components/query/companion/*         SHELL + panes
  components/graph/embedded-answer-graph.tsx      ISOLATED graph host
```

## Pure model (`companion-pane.ts`)

```ts
// Normative shape (implementation may refine names)
type CompanionKind = "none" | "pdf" | "graph";

type SourceLocation = {
  documentId: string;
  page?: number;       // 1-indexed
  pageEnd?: number;
  chunkId?: string;
  startLine?: number;
  endLine?: number;
  title?: string;
};

type CompanionState = {
  version: 1;
  kind: CompanionKind;
  source: SourceLocation | null;
  messageId: string | null;
  widthPx: number;     // desktop side pane
};

// encodeCompanionSearch(state) -> URLSearchParams patch
// decodeCompanionSearch(params) -> CompanionState (sanitised)
// clampWidth(px, viewport, flags) -> number
// migrateCompanion(raw) -> CompanionState | default
```

**URL keys:** `pane`, `doc`, `page`, `chunk`, `msg` (short, stable).  
Omit keys when `kind === "none"`. Never put secrets in the URL.

Storage key: `edgequake.query.companion.v1` (width + last kind only; location
comes from URL when present).

## State ownership

```text
  URL search params     ──hydrate──►  companion store
  companion store       ──replace──►  URL (router.replace, scroll:false)
  openSource / openAnswerGraph      ►  store + URL
  useQueryUIStore.historyPanelOpen  ◄  auto-rail when companion + tight width
  useAnswerGraphStore (merged)      ◄  stream session + Show on graph
  EmbeddedAnswerGraph local engine  ║  MUST NOT mutate Graph Studio singleton
                                     ║  nodes/edges/sigma for /graph
```

## Intent hooks

### `useOpenSource`

```text
  openSource(location | CitationChunk, triggerEl?)
    - If ENABLE_QUERY_COMPANION false → router.push(buildCitationHref)
    - Else set kind=pdf, source=mapped, sync URL, focus pane, announce
  openSourceNewTab(chunk) → window.open(buildCitationHref)  // modifiers
  closeCompanion() → kind=none, clear pane params, restore focus
```

Replace call sites:

1. `citation-popover.tsx` same-tab branch
2. `documents-tab` / `knowledge-tab` / `passage-row` via `onDocumentClick`
3. Any other `invokeDocumentClick` from Query citations

### `openAnswerGraph(messageId)`

```text
  - Map message.context via answer-graph.ts → setAnswerSubgraph
  - kind=graph, messageId, sync URL
  - Escape: openStudio() → existing router.push(/graph?answerMessage&focus=answer)
```

## `DocumentSourceView`

Extracted responsibilities from `documents/[id]/page.tsx`:

- Resolve PDF vs text (`pdf_id` / `source_type`)
- Fetch document + PDF content (React Query keys unchanged)
- Drive `PDFViewer` with `currentPage` / `onPageChange`
- Highlight chunk / lines for text via `ContentRenderer`
- Error / loading / empty states

**Not** included in companion extract (stay on full page): reprocess dialogs,
metadata sidebar, page health strip, assets include side-effect — unless needed
for viewing. Companion is read-mostly.

Props sketch:

```ts
type DocumentSourceViewProps = {
  documentId: string;
  page?: number;
  chunkId?: string;
  startLine?: number;
  endLine?: number;
  className?: string;
  onReady?: (meta: { title: string; numPages?: number }) => void;
  onError?: (error: Error) => void;
};
```

## `EmbeddedAnswerGraph`

```text
  Inputs: messageId | subgraph bundle
  Render: GraphRenderer with local nodes/edges derived from subgraph
  Focus: engineFocus mode=answer
  Actions: expand 1-hop (neighbourhood API or client BFS on loaded edges),
           Open in Graph Studio, table fallback
  Isolation options (pick one in W3, prove with e2e):
    A) createStore scoped instance passed via React context
    B) separate zustand store `use-answer-graph-view-store`
    C) GraphEngine created with local graphology; never call useGraphStore.setGraph
  Forbidden: mounting GraphViewer; writing allNodes into useGraphStore
```

## QueryInterface layout change

```text
  BEFORE:  [ chat flex-1 | history? ]
  AFTER:   [ chat flex-1 | companion? | history? ]

  Implementation:
    - react-resizable-panels Group horizontal for chat|companion
    - history stays ResizablePanel or rail
    - < md: companion via Sheet (portal), chat full width
```

Feature flag gate at the top of `QueryInterface` / intent hooks.

## SOLID / DRY checklist

| Rule | Enforcement |
|------|-------------|
| DRY open path | Only `useOpenSource` / `openAnswerGraph` write companion open |
| SRP | Pure codec vs store vs panes vs PDF vs graph |
| OCP | `CompanionKind` switch in shell; panes register independently |
| DIP | Panes depend on DocumentSourceView / GraphRenderer, not route pages |
| No remount | Stable `key={documentId}` on Source; page via props (LAW-157-12) |

## Refactors required (ordered)

1. Extract `useDocumentLocation` + `DocumentSourceView` (W0) — documents page
   becomes a consumer (behaviour parity tests).
2. Add `companion-pane.ts` + store + shell (W1).
3. Wire `useOpenSource` (W2).
4. Merge answer stores + embedded graph (W3).
5. Scope chip + quote-to-ask (W4).

## Rollout flag

```text
  ENABLE_QUERY_COMPANION
    - env: NEXT_PUBLIC_ENABLE_QUERY_COMPANION (optional)
    - or settings / ui-preferences store boolean
  Default: true in development; staged for production release notes
  Off path: existing router.push behaviour (no companion UI)
```

## Performance notes

- Lazy-load `DocumentSourceView` and `EmbeddedAnswerGraph` with `next/dynamic`
  (`ssr: false` for PDF/graph).
- Do not prefetch PDF until pane opens.
- Keep PDF windowed render (`WINDOW_THRESHOLD` in `pdf-viewer.tsx`).
- Companion open must not trigger full Query remount.

## Testing seams

| Unit | Assert |
|------|--------|
| `encode/decodeCompanionSearch` | Round-trip; strip bad params |
| `clampWidth` | Viewport / rail combinations |
| `mapContextToAnswerFocus` | ids + name fallback + empty |
| `openSource` (mock router) | Flag off → push; on → store |

Cross-ref: [09-implementation-plan](09-implementation-plan.md) ·
[lenses/LENS-front.md](lenses/LENS-front.md) ·
[lenses/LENS-full-stack.md](lenses/LENS-full-stack.md).
