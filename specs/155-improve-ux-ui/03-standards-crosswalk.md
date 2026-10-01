# 03 — Standards crosswalk

Parent: [README](README.md) · Laws: [01](01-first-principles.md) · Next: [04-findings](04-findings.md)

Sources verified **2026-10-01** (live web). Status: **MET** / **GAP** / **INFORMATIONAL**.

## WCAG 2.2 (binding — Level AA)

Normative: [WCAG 2.2 Recommendation](https://www.w3.org/TR/WCAG22/) ·
[What's new in 2.2](https://www.w3.org/WAI/standards-guidelines/wcag/new-in-22/).

| Criterion | Level | EdgeQuake status | Evidence / gap | Law |
|-----------|-------|------------------|----------------|-----|
| 1.3.5 Identify Input Purpose | AA | **GAP** | Login lacks `autoComplete` (`login/page.tsx`) | 8 |
| 1.4.3 Contrast (Minimum) | AA | Partial | muted-foreground tuned; sub-12px muted risk | 13 |
| 1.4.10 Reflow | AA | Partial | Documents table no responsive column drop; query modes wrap | 2 |
| 1.4.11 Non-text Contrast | AA | Partial | Header status colour-only below `sm` | 1,8 |
| 1.4.13 Content on Hover/Focus | AA | Partial | Graph hover cards canvas-only | 8 |
| 2.1.1 Keyboard | A | **GAP** | Graph Tab/arrows/Enter hijacked; side-by-side resizer mouse-only | 8 |
| 2.1.4 Character Key Shortcuts | A | **GAP** | `?` shortcut not disableable | 8 |
| 2.2.1 Timing Adjustable | A | Risk | Toasts 3s auto-dismiss (close helps) | 7 |
| 2.4.7 Focus Visible | AA | Partial | Four competing ring styles | 13 |
| 2.4.11 Focus Not Obscured (Min) | AA | Risk | Sticky header + banner; no scroll-padding | 8 |
| 2.4.13 Focus Appearance | AAA | Partial (aspire) | Ring/50 may fail 3:1 | 13 |
| 2.5.7 Dragging Movements | AA | Partial | ResizablePanel OK; graph drag / PDF no keyboard alt for all | 8 |
| 2.5.8 Target Size (Minimum) | AA | Partial | h-5/h-6 icon buttons in query/docs; dialog close ~16px | 8 |
| 3.2.6 Consistent Help | A | Partial | `?` dialog exists; no persistent help link | 2 |
| 3.3.7 Redundant Entry | A | Partial | Wizard drafts OK; login/settings gaps | 7 |
| 3.3.8 Accessible Authentication | AA | Partial | No cognitive test; password managers blocked by missing autocomplete | 8 |
| 4.1.3 Status Messages | AA | Partial | 23 aria-live; header connection / route change silent | 7 |

## WCAG 3.0 (informative only)

- [WCAG 3.0 Working Draft 10 Sep 2026](https://www.w3.org/TR/wcag-3.0/)
- [WAI news: For Review Sep 2026](https://www.w3.org/WAI/news/2026-09-10/wcag3/)

**Status:** incomplete Working Draft; years from Recommendation; does **not**
replace WCAG 2. EdgeQuake tracks WCAG 2.2 AA. WCAG 3 outlook informs
assertions (e.g. broader cognitive support) but is **not** a gate.

## WAI-ARIA APG patterns

| Pattern | Where used | Gap |
|---------|------------|-----|
| Toolbar | Zoom controls | Good; graph top chrome incomplete |
| Listbox | Entity browser | Good |
| Combobox | Workspace selector | Good |
| Separator (window splitter) | `ui/resizable-panel` | Side-by-side viewer must reuse |
| Dialog / Menu | Radix | Context menu keyboard open missing (Shift+F10) |
| Application | Graph canvas | Needs tabindex + roving focus, not global hijack |
| Command palette | Advertised Cmd+K | **Not mounted** |

## Design system standards (2026)

| Standard | Citation | Gap |
|----------|----------|-----|
| Tailwind CSS v4 `@theme` / `@theme inline` semantic tokens | [Tailwind v4 + Next 16 setup](https://nextjslaunchpad.com/article/tailwind-v4-nextjs-16-css-first-theme-setup); [semantic tokens guide](https://thefrontkit.com/blogs/tailwind-css-design-tokens-for-saas) | No `--success/--warning/--info`; 590 raw palette classes |
| oklch perceptual colour | Same | Used for shadcn base; status colours still Tailwind named |
| `next/font` | Next 16 | Inter loaded as `--font-inter`; CSS maps to undefined `--font-geist-sans` |
| View Transitions / React Compiler | Next 16 / React 19 | Not enabled; evaluate W9 |

## Graph visualization standards

| Source | Takeaway | EdgeQuake gap |
|--------|----------|---------------|
| Cambridge Intelligence — meaningful graph UX | Progressive disclosure, filtering, clustering, accessible colour | Hairball defaults; no LOD tiers |
| CMGV complexity management (2025) | Semantic zoom, expand/collapse, edge bundling | No semantic zoom; parallel edges dropped |
| NetworkCanvas CHI 2026 | Progressive exploration, user agency | No guided recommendations (defer) |
| GI 2025 visual summaries | Contour / cluster disks for large nets | No hull/summary layer |
| Sigma.js v3 migration & packages | Instanced programs; `@sigma/export-image`; workers via graphology | Pin 3.0.2 (latest 3.0.3 Aug/Sep 2026); broken export; no worker FA2 |

Package pins to target (npm as of 2026-10-01):

- `sigma` **3.0.3**
- `@sigma/export-image` **3.0.0**
- `@axe-core/playwright` **4.13.0**
- `graphology-layout-forceatlas2` **0.10.1** (worker supervisor)
- `graphology-shortest-path` **2.1.0**
- `graphology-metrics` **2.4.2**

## Web Vitals / performance budgets (LAW-155-12)

| Metric | Budget | Gate |
|--------|--------|------|
| LCP (shell) | &lt; 2.5s on mid laptop | Playwright web-vitals helper W9 |
| INP (primary click) | &lt; 200ms | Graph filter / query send |
| CLS | &lt; 0.1 | Keep SPEC-100 green |
| Graph filter apply | &lt; 50ms, **no** Sigma rebuild | vitest bench + e2e |
| Graph pan | 60fps @ 2k nodes | layout-performance e2e refresh |
| Graph materialization | Server 15s timeout; UI honour 503 + retry | Existing stream retry |

## Security / auth UX intersection

SPEC-154 remains binding. SPEC-155 only touches:

- Login form autocomplete / show-password (a11y)
- Humanised auth errors (no raw JWT messages)
- No new secret surfaces in URLs

## Mapping summary

```text
  WCAG 2.2 AA ──────────────► LAW-155-8,13 + EC a11y + axe gates
  Tailwind v4 tokens ───────► LAW-155-1 + design-system-spec
  Sigma 3 + graph research ─► LAW-155-3,4,5 + graph-studio-spec
  Data honesty ─────────────► LAW-155-6,7,10 + data-contract
  Answer linkage ───────────► LAW-155-11 + AI lens
  CI proof ─────────────────► LAW-155-14 + e2e matrix
```

Cross-ref: [04-findings](04-findings.md) · [11-e2e-test-matrix](11-e2e-test-matrix.md).
