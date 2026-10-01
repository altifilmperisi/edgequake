# Lens — Front Designer / Frontend Engineer

Parent: [README](../README.md) · Design: [05](../05-design-system-spec.md) · Graph: [06](../06-graph-studio-spec.md)

## Ownership

- Tailwind v4 tokens, CVA primitives, layout shell, GraphEngine integration,
  i18n wiring, Playwright UI gates.

## W1 checklist

- [ ] `next/font` Geist → `--font-geist-sans` / mono
- [ ] `@theme inline` success/warning/info/danger + graph tokens
- [ ] `getCssToken` helper for canvas
- [ ] PageShell / PageHeader shipped and adopted on ≥3 routes
- [ ] CommandPalette mounted
- [ ] ESLint drafts for palette + type floor (enforce W2)
- [ ] Remove unused `@react-sigma/*`

## GraphEngine engineering notes

```text
  lib/graph/engine/
    create-engine.ts
    apply-delta.ts
    focus-strategies.ts
    lod.ts
    export.ts
    theme.ts

  React: useGraphEngine(containerRef) — effect create/destroy once
  Tests: reducers with graphology Graph in node/jsdom
```

- Subscribe with `useShallow` / atomic selectors.
- Debounce search ≥150ms shared `useDebounce`.
- RAF batch stream upserts.

## i18n

- Never add ja/ko until files exist.
- Prefer keys in JSON over inline defaults long-term (parity script).
- `Intl.NumberFormat(i18n.language)` in formatters.

## Perf

- Dynamic import Graph Studio route.
- Avoid importing sigma from non-graph stores.
- Triple highlighter debt (shiki/hljs/prism) — consolidate later W9.

## Definition of done (front)

PR includes screenshots, axe summary, and list of closed F-155-D/S/G ids.
