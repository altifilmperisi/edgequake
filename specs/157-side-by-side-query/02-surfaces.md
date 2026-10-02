# 02 — Surfaces (code map)

Parent: [README](README.md) · Prior: [01-first-principles](01-first-principles.md) ·
Next: [03-product-spec](03-product-spec.md)

## Route map

```text
  /query                          QueryInterface (primary)
  /query?pane=pdf&doc=&page=&chunk=   Companion Source open
  /query?pane=graph&msg=              Companion Graph open
  /documents/[id]?page&chunk          Full-page viewer (escape hatch)
  /graph?answerMessage&focus=answer   Full Graph Studio (escape hatch)
```

## Query surface (today)

| File | Role | SPEC-157 change |
|------|------|-----------------|
| [`app/(dashboard)/query/page.tsx`](../../edgequake_webui/src/app/%28dashboard%29/query/page.tsx) | Route shell | Wrap / host companion-aware interface |
| [`components/query/query-interface.tsx`](../../edgequake_webui/src/components/query/query-interface.tsx) | Chat + history flex | Add companion slot; layout modes |
| [`components/query/message/assistant-message.tsx`](../../edgequake_webui/src/components/query/message/assistant-message.tsx) | `handleShowOnGraph` → `/graph` | Call `openAnswerGraph` (pane primary) |
| [`components/query/message/message-actions.tsx`](../../edgequake_webui/src/components/query/message/message-actions.tsx) | “Show on graph” button | Keep; wire via parent |
| [`components/query/citations/citation-popover.tsx`](../../edgequake_webui/src/components/query/citations/citation-popover.tsx) | Chip → `router.push` | Same-tab → `openSource`; modifiers unchanged |
| [`components/query/citations/documents-tab.tsx`](../../edgequake_webui/src/components/query/citations/documents-tab.tsx) | `onDocumentClick` | Route through `useOpenSource` |
| [`components/query/citations/knowledge-tab.tsx`](../../edgequake_webui/src/components/query/citations/knowledge-tab.tsx) | Entity → document | Same |
| [`components/query/citations/passage-row.tsx`](../../edgequake_webui/src/components/query/citations/passage-row.tsx) | Passage open | Same |
| [`components/query/history/panel.tsx`](../../edgequake_webui/src/components/query/history/panel.tsx) | History `ResizablePanel` 280px | Auto-rail when companion open |
| [`stores/use-query-ui-store.ts`](../../edgequake_webui/src/stores/use-query-ui-store.ts) | History open flag | Coordinate with companion |
| [`hooks/use-query-scope.ts`](../../edgequake_webui/src/hooks/use-query-scope.ts) | `@` document scope | W4: open doc → scope chip |
| [`lib/citations/citation-href.ts`](../../edgequake_webui/src/lib/citations/citation-href.ts) | Full-page href builder | Keep for escape hatch / new tab |
| [`lib/utils/document-url.ts`](../../edgequake_webui/src/lib/utils/document-url.ts) | Canonical deeplink | Keep; companion URL is separate codec |

## Document / PDF surface (reuse)

| File | Role | SPEC-157 change |
|------|------|-----------------|
| [`components/documents/pdf-viewer.tsx`](../../edgequake_webui/src/components/documents/pdf-viewer.tsx) | Controlled page, windowed render | Consume as-is in companion |
| [`components/documents/pdf-markdown-split-view.tsx`](../../edgequake_webui/src/components/documents/pdf-markdown-split-view.tsx) | PDF+MD split | Optional mode inside Source |
| [`components/documents/side-by-side-viewer.tsx`](../../edgequake_webui/src/components/documents/side-by-side-viewer.tsx) | Generic split chrome | Not the Query shell |
| [`components/documents/document-viewer-dialog.tsx`](../../edgequake_webui/src/components/documents/document-viewer-dialog.tsx) | Modal viewer | Escape hatch only |
| [`app/(dashboard)/documents/[id]/page.tsx`](../../edgequake_webui/src/app/%28dashboard%29/documents/%5Bid%5D/page.tsx) | Full detail + URL sync | **Extract** location + source view (W0) |
| [`hooks/use-page-sync-controller.ts`](../../edgequake_webui/src/hooks/use-page-sync-controller.ts) | PDF↔MD sync | Reuse if Source shows MD |

## Graph surface (reuse carefully)

| File | Role | SPEC-157 change |
|------|------|-----------------|
| [`components/graph/graph-viewer.tsx`](../../edgequake_webui/src/components/graph/graph-viewer.tsx) | Full Studio (~1226 LOC) | **Do not embed**; keep answer deep-link |
| [`components/graph/graph-renderer.tsx`](../../edgequake_webui/src/components/graph/graph-renderer.tsx) | Thin Sigma shell | Embed via `EmbeddedAnswerGraph` |
| [`lib/graph/engine/`](../../edgequake_webui/src/lib/graph/engine/) | GraphEngine | Prefer isolated instance for Query |
| [`stores/use-graph-store.ts`](../../edgequake_webui/src/stores/use-graph-store.ts) | Global singleton | Isolation required (LAW-157-9) |
| [`stores/use-answer-graph-store.ts`](../../edgequake_webui/src/stores/use-answer-graph-store.ts) | Answer focus SSOT (used) | Merge with unused twin |
| `stores/use-answer-on-graph-store.ts` | Parallel / unused | **Deleted** in W3 (merged into `use-answer-graph-store`) |
| [`hooks/use-query-stream-session.ts`](../../edgequake_webui/src/hooks/use-query-stream-session.ts) | Sets answer subgraph on stream | Keep writing merged store |
| [`components/graph/graph-as-table.tsx`](../../edgequake_webui/src/components/graph/graph-as-table.tsx) | A11y / WebGL fallback | Use in Graph pane fallback |

## Layout primitives (SPEC-155)

| File | Role | SPEC-157 guidance |
|------|------|-------------------|
| [`lib/documents/workspace-layout.ts`](../../edgequake_webui/src/lib/documents/workspace-layout.ts) | Documents docking tree | **Do not fork zones**; new pure model |
| [`hooks/use-workspace-layout.ts`](../../edgequake_webui/src/hooks/use-workspace-layout.ts) | Persistence pattern | Mirror pattern for companion width |
| [`components/documents/workspace/collapsing-panel.tsx`](../../edgequake_webui/src/components/documents/workspace/collapsing-panel.tsx) | Panel collapse API | Reuse or thin wrapper |
| [`components/ui/resizable-panel.tsx`](../../edgequake_webui/src/components/ui/resizable-panel.tsx) | Pixel width panel | History already uses this |
| `react-resizable-panels` v4 | Group / Panel / Separator | Companion shell splitter |

## API / backend surfaces

| File | Role | Gap |
|------|------|-----|
| [`types/query.ts` `QueryContext`](../../edgequake_webui/src/types/query.ts) | Chunks with `page_start/end`, `chunk_id` | Sufficient for Source |
| [`message_context_mapper.rs`](../../edgequake/crates/edgequake-api/src/services/message_context_mapper.rs) | Persist entities + pages | May lack graph node id after reload |
| [`handlers/chat/streaming.rs`](../../edgequake/crates/edgequake-api/src/handlers/chat/streaming.rs) | Emits `subgraph` | Good path |
| [`handlers/chat/mod.rs`](../../edgequake/crates/edgequake-api/src/handlers/chat/mod.rs) | Non-stream `subgraph: None` | Optional W5 fix |
| Document PDF APIs (`getPdfContent`, `getPdfDownloadUrl`) | Authenticated PDF | Reuse |

## Proposed new modules (target)

```text
  edgequake_webui/src/
    lib/query/
      companion-pane.ts          # pure model + URL codec + clamp
      answer-graph.ts            # context → SubgraphBundle + focus ids
      __tests__/...
    stores/
      use-companion-pane-store.ts
    hooks/
      use-open-source.ts
      use-document-location.ts   # extracted from documents/[id]
    components/
      documents/
        document-source-view.tsx # shared PDF/text viewer
      query/companion/
        companion-shell.tsx
        companion-tabs.tsx
        source-pane.tsx
        graph-pane.tsx
      graph/
        embedded-answer-graph.tsx
  e2e/spec157/
    companion-source.spec.ts
    companion-graph.spec.ts
    companion-layout.spec.ts
    helpers/ (reuse spec155 mocks)
```

## Data flow ASCII

```text
  [Citation chip] --openSource--> companion store --URL sync--> /query?pane=pdf&...
        |                              |
        |                              +--> SourcePane --> DocumentSourceView
        |                                        |
        +-- modifier click --> buildCitationHref --> /documents/...

  [Show on graph] --openAnswerGraph--> companion store --URL--> ?pane=graph&msg=
        |                              |
        |                              +--> GraphPane --> EmbeddedAnswerGraph
        |                                        |
        +-- "Open in Graph Studio" --> /graph?answerMessage&focus=answer
```

## Findings index (F-157)

| ID | Finding | Evidence |
|----|---------|----------|
| F-157-01 | Citation navigates away | `citation-popover.tsx` `router.push` |
| F-157-02 | Show-on-graph navigates away | `assistant-message.tsx` `router.push('/graph…')` |
| F-157-03 | No companion slot | `query-interface.tsx` flex: chat + history only |
| F-157-04 | Location logic trapped in page | `documents/[id]/page.tsx` URL/chunk sync |
| F-157-05 | Duplicate answer stores | `use-answer-graph-store` + `use-answer-on-graph-store` |
| F-157-06 | Global graph store hazard | `use-graph-store` singleton + `GraphViewer` coupling |
| F-157-07 | Width budget unaccounted | Sidebar + history mins vs companion min |
| F-157-08 | Non-stream subgraph null | `handlers/chat/mod.rs` `subgraph: None` |
| F-157-09 | Entity id weak after reload | `message_context_mapper` name-first persistence |

Cross-ref: [05-frontend-architecture](05-frontend-architecture.md) ·
[06-data-and-db-contract](06-data-and-db-contract.md) · [08-edge-cases](08-edge-cases.md).
