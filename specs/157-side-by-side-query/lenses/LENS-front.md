# Lens — Front (UI engineer)

Parent: [README](../README.md) · Architecture: [05](../05-frontend-architecture.md) ·
UX: [04](../04-ux-ui-spec.md)

## Ownership

React components, Zustand stores, URL sync, resizable panels, i18n keys,
Playwright UI gates.

## Component inventory (new)

| Component | Notes |
|-----------|-------|
| `CompanionShell` | Panels or Sheet; owns splitter |
| `CompanionTabs` | Source / Graph |
| `SourcePane` | Header + `DocumentSourceView` |
| `GraphPane` | Header + `EmbeddedAnswerGraph` |
| `DocumentSourceView` | Shared with documents route |
| `EmbeddedAnswerGraph` | Not `GraphViewer` |

## State rules

- Subscribe with selectors / `useShallow` — hover on graph must not re-render chat.
- URL updates via `router.replace(..., { scroll: false })`.
- Persist width only after pointer-up / keyboard settle (debounce).

## CSS / layout

```text
  Use: flex min-h-0 min-w-0 overflow-hidden (SPEC-100 chain)
  Avoid: h-screen inside companion (breaks nested flex)
  Splitter: react-resizable-panels Separator with aria props
```

## i18n

Add `query.companion.*` to `en.json`, `fr.json`, `zh.json` in the same PR as
chrome. Run locale parity before merge.

## Anti-patterns

- Importing `GraphViewer` into Query
- Forking `workspace-layout.ts` zone ids for Query
- `key={page}` on `PDFViewer` (forces remount)
- Nested scroll on `body` when pane opens

## Front test focus

Layout, focus, URL, flag-off, axe, locale — see [10](../10-e2e-test-matrix.md).

Cross-ref: LAW-157-5,9,12 · F-157-03…07.
