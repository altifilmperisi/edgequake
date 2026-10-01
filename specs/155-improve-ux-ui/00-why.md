# 00 — WHY (5-WHY)

Parent: [README](README.md) · Next: [01-first-principles](01-first-principles.md)

## The job to be done

An analyst, operator, or AI host using EdgeQuake must experience **one**
product quality bar:

1. Every screen feels clean, elegant, and minimal — September 2026 standard.
2. The knowledge graph is the most trustworthy and beautiful graph UX in its
   class: stable layout, honest state, progressive disclosure, answer linkage.
3. Accessibility, internationalisation, and performance are not afterthoughts;
   they are laws enforced by CI.

## Five WHYs

```text
WHY-1  Product quality feels uneven across screens
  WHY? Each route invents its own header, status colours, empty/error copy,
       and loading skeleton (no PageShell; 528+ raw palette uses).
  WHY? Prior UX specs (029/030/099) shipped islands, not a system.
  WHY? No binding design-token law or lint gate.
  WHY? No CI proof for a11y / locale / visual regression.
  --> ROOT: Missing first principles + enforcement (LAW-155-1,2,14)

WHY-2  Graph trust collapses on first filter or stream batch
  WHY? graph-renderer rebuilds Sigma on every nodes/edges/theme change
       (graph-renderer.tsx:728); positions and camera are lost.
  WHY? Filter search writes store on each keystroke without debounce;
       filtered props re-enter the init effect.
  WHY? Layout runs on the main thread; no worker; no position persistence.
  WHY? React owns the canvas lifecycle instead of a single GraphEngine.
  --> ROOT: Unstable mental map (LAW-155-3,4)

WHY-3  Controls lie (time filter, costs period, Cmd+K, truncation banner)
  WHY? UI state exists without a data path (timeFilter never applied;
       costs period cosmetic; searchOpen never mounts a palette).
  WHY? Streaming truncation inverted; server totals discarded.
  WHY? No "no control without effect" law + e2e gate.
  --> ROOT: Dishonest affordances (LAW-155-6,7)

WHY-4  Graph cannot explain answers
  WHY? Chat/query already return subgraph entities+relationships with scores,
       but no graph component reads them.
  WHY? Entity ids in context are "ent:{name}" while graph nodes use
       workspace-scoped ids — mapping gap.
  WHY? Answer-on-graph was never a product law.
  --> ROOT: Missing answer→graph linkage (LAW-155-11)

WHY-5  "Highest standard" cannot be claimed without evidence
  WHY? No axe, no toHaveScreenshot, no locale-parity test, Chrome-only
       Playwright, stale audit_ui screenshots (v0.12 vs v0.28.5).
  WHY? Vitest environment is node (no component DOM tests).
  --> ROOT: CI is not proof (LAW-155-14)
```

## Causal ASCII — product quality as a chain

```text
  Token SSOT ──► PageShell ──► Honest states ──► Screen polish
       │              │              │
       │              │              └── i18n / a11y / motion
       │              │
       └──────────────┴── GraphEngine (mutate, never rebuild)
                                │
                    +-----------+-----------+
                    |           |           |
                 LOD/hulls   ego/path    answer-on-graph
                    |           |           |
                    +-----------+-----------+
                                |
                          Trust + beauty
```

## What "best on the market" means for Graph Studio

Relative to Neo4j Bloom, Linkurious, Kumu, Gephi Lite, Cosmograph, Obsidian:

| Capability | EdgeQuake today | Target (SPEC-155) |
|------------|-----------------|-------------------|
| Stable layout under filter | Rebuilds | Dim-in-place |
| Worker layout | Main-thread FA2 | Worker FA2 + pin |
| Semantic zoom / LOD | Hide-on-move only | 3 tiers |
| Community hulls | Client Louvain colour only | Server `community_id` + hulls |
| Path / ego | 1-hop expand only | Ego depth + shortest path |
| Export | Broken PNG (edges layer) | `@sigma/export-image` PNG+SVG |
| Answer highlight | None | `subgraph` layer |
| A11y | Global key hijack | Roving focus + table alt |

## Non-goals (explicit)

- Engine swap away from Sigma/graphology
- Full RTL shipping (prepare logical properties; no RTL locale yet)
- Server-side layout coordinates, merge suggestions, graph diff (deferred)
- Reopening SPEC-099 Documents status SSOT / feedback-zone design

## Success narrative

When Waves 1–9 land, an operator can:

1. Open Graph Studio, filter by type, search, and stream more nodes **without
   the layout jumping**.
2. Click “Show on graph” on a Query answer and see retrieved entities lit.
3. Use keyboard and screen reader to select a node and open its menu.
4. See the same typography, status colours, and page chrome on every route.
5. Trust CI: axe green, locale parity green, visual baselines green, cargo
   graph-contract tests green.

Cross-ref: [01-first-principles](01-first-principles.md) · [04-findings](04-findings.md).
