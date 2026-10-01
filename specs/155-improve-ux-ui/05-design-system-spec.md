# 05 — Design system specification

Parent: [README](README.md) · Findings: [04](04-findings.md) · Laws: LAW-155-1,2,7,8,9,13 · Next: [06-graph-studio-spec](06-graph-studio-spec.md)

## WHY

Without one token source and one page shell, every screen invents chrome,
status colour, and empty/error grammar. That is the root of uneven quality
(WHY-1). This doc is the normative design system for Waves 1–2 and 7–8.

## Locked visual decisions

1. **Typeface:** Geist Sans via `next/font` → `--font-sans`. Mono: Geist Mono
   (or system ui-monospace). Inter optional via settings later — not Wave 1.
2. **Colour model:** oklch semantic roles under `:root` / `.dark`, mapped with
   `@theme inline` (Tailwind v4).
3. **Accent:** single primary (retain current shadcn primary hue family).
4. **Status:** four roles only — success, warning, info, danger.
5. **Density:** comfortable default; no “compact” mode in W1.
6. **Ornament ban:** no decorative gradients, glow blurs, or hover-lift on
   product chrome (query empty glow, login gradients, stats-card lines go).

## Semantic tokens (normative)

```text
  :root / .dark
  -------------
  --background --foreground --card --popover
  --primary --primary-foreground
  --secondary --muted --muted-foreground --accent
  --destructive  (alias danger for shadcn compat)
  --success --success-foreground
  --warning --warning-foreground
  --info --info-foreground
  --border --input --ring
  --sidebar-* (retain)
  --graph-canvas-bg --graph-edge --graph-label-fg --graph-label-bg
  --graph-focus --graph-dim --graph-hull-stroke --graph-hull-fill
  --graph-community-0 … --graph-community-9  (Okabe–Ito inspired, AA on canvas bg)

  @theme inline {
    --color-success: var(--success);
    --color-warning: var(--warning);
    --color-info: var(--info);
    --color-danger: var(--destructive);
    --font-sans: var(--font-geist-sans);
    --font-mono: var(--font-geist-mono);
  }
```

**AA requirement:** text-on-status pairs ≥ 4.5:1; non-text status glyphs ≥ 3:1.
Document measured oklch values in a short appendix during W1 (contrast table
in PR). Graph community colours must remain distinguishable under deuteranopia
(shape redundancy in Graph Studio — [06](06-graph-studio-spec.md)).

### Runtime token resolver (canvas)

```text
  getCssToken('--graph-focus') → resolved colour string
  Used by: GraphEngine theme apply, minimap, export background
  Forbidden: hard-coded #hex in renderer/expansion/minimap (allow-list: tests)
```

## Type scale

| Token | Size | Use |
|-------|------|-----|
| `text-xs` | **12px** floor | Meta, badges |
| `text-sm` | 14px | Body secondary, tables |
| `text-base` | 16px | Body |
| `text-lg` | 18px | Section |
| `text-xl` / `text-2xl` | 20 / 24 | Page titles via PageHeader only |

**Ban:** `text-[8px]`…`text-[11px]` — ESLint rule W2. Sidebar footer may use
`text-xs` only.

## Radius, shadow, z-index, motion

```text
  Radius:  --radius-sm (controls)  --radius-lg (cards/panels)   [drop parallel --rounded-* chaos]
  Shadow:  --shadow-xs  --shadow-sm  --shadow-md
  Z:       base 0 | sticky 20 | dropdown 40 | modal 50 | toast 60 | skip 9999
  Motion:  --duration-0|150|200|300 ; --ease-standard
           @media (prefers-reduced-motion: reduce) → duration 0; keep opacity fades ≤150ms
           Spinners: replace with static progress / aria-busy text when reduced
```

## Focus SSOT

One utility class (or CVA slot) for interactive controls:

```text
  focus-visible:outline-none
  focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2
  focus-visible:ring-offset-background
```

Forced-colours: `outline: 2px solid transparent; outline-offset: 2px;
forced-color-adjust: none` on focusable chrome. Prefer-contrast: strengthen
border on inputs.

## Shared primitives (must exist and be used)

| Primitive                                        | Responsibility                                            | Replaces                         |
| --------------------------------------------------| -----------------------------------------------------------| ----------------------------------|
| `PageShell`                                      | max-width rhythm, padding, main landmark props            | ad-hoc `container` / `max-w-7xl` |
| `PageHeader`                                     | h1, description, primary/secondary actions, optional tabs | snowflake headers                |
| `StatusBadge`                                    | maps domain status → semantic token                       | 8× statusConfig maps             |
| `EmptyState`                                     | icon/illustration, title, description, CTA                | duplicates                       |
| `ErrorState`                                     | humanised code + retry                                    | raw `error.message`              |
| `ConnectionIndicator`                            | online / reconnecting / offline / backend-down            | documents-only banner            |
| `formatCost` / `formatDuration` / `formatNumber` | Intl SSOT                                                 | 6× formatCost copies             |
| `CommandPalette`                                 | Cmd+K routes, actions, recent                             | dead `searchOpen`                |
| `DataTableResponsive`                            | column priority collapse                                  | unused responsive-table          |

### PageHeader ASCII

```text
  +----------------------------------------------------------+
  | Title (h1 text-xl)                    [Secondary] [Primary]|
  | Description (text-sm muted)                                |
  | optional: tabs / filters row                               |
  +----------------------------------------------------------+
```

## Command palette (LAW-155-6)

- Mount once in shell provider.
- Sources: navigable routes, “New query”, “Upload document”, “Open graph”,
  workspace switch (if entitled), theme toggle, language, keyboard help.
- Library: existing `cmdk` + `ui/command.tsx`.
- If delayed past W1: **remove** Cmd+K from shortcut help in the same PR.

## i18n contract (shell half — full in W8)

- Settings language list = `{en, fr, zh}` only until more locales ship.
- `setLanguage` **must** call `i18n.changeLanguage` and set
  `document.documentElement.lang` (+ `dir` when RTL ships).
- W2 gate: locale key parity test fails CI on missing keys.

## Migration order (W1)

1. Fix font variables (`layout.tsx` + `globals.css`).
2. Add status + graph tokens; map `@theme inline`.
3. Add primitives under `components/shared/` (wire EmptyState/Skeletons that
   already exist).
4. ESLint allow-list for raw palette (graph canvas interim + charts).
5. Replace Header/Sidebar hard-coded strings with `t()`.
6. Mount CommandPalette; dedupe keyboard hook to one listener.
7. Add `not-found.tsx` + humanise `error.tsx` / `global-error.tsx`.

## Out of scope here

Graph interaction behaviour → [06](06-graph-studio-spec.md).  
Per-screen layouts → [07](07-screens-spec.md).

Cross-ref: [03-standards-crosswalk](03-standards-crosswalk.md) · [LENS-front](lenses/LENS-front.md).
