# 04 — UX / UI specification

Parent: [README](README.md) · Product: [03](03-product-spec.md) ·
Architecture: [05](05-frontend-architecture.md)

## Design intent

Clean, minimal, SPEC-155 visual language. Companion is a **tool**, not a second
app: same tokens, 12px floor, one accent, no decorative chrome.

## Width budget

| Region | Min (px) | Notes |
|--------|----------|-------|
| App sidebar | 64 collapsed / 256 expanded | Existing |
| Chat column | 420 | Messages + composer usable |
| Companion | 440 | PDF / graph readable |
| History | 280 open / 28 rail | Auto-rail when companion open and `innerWidth < 1440` |

```text
  Constraint (desktop):
    sidebar + chatMin + companionMin + historyEffective <= viewport

  If overflow:
    1) rail history
    2) collapse sidebar if user already collapsed OR offer tip
    3) clamp companion width to remaining (never < companionMin unless sheet)
```

## Breakpoints

| Name | Width | Companion presentation |
|------|-------|------------------------|
| `sm` / mobile | &lt; 768 | Bottom sheet ~75vh, drag handle, overlay |
| `md`–`lg` | 768–1279 | Side pane; history is mobile sheet (existing) |
| `xl` | ≥ 1280 | Side pane + history; history auto-rails when companion open if width tight |
| `2xl` | ≥ 1536 | Side pane + history open comfortable |

## ASCII wireframes

### Desktop — Source open

```text
+------+---------------------------+------------------------+------+
| Nav  | Query                     | Source            [×]  | Hist |
|      | Ask questions…            | report.pdf  p.12 / 48  | |||| |
|      |                           | [PDF] [MD]  Open full  | rail |
|      |  User: What is X?         | +--------------------+ |      |
|      |  Assistant: … [3] …       | |   PDF page 12      | |      |
|      |  [Show on graph]          | |   highlight span   | |      |
|      |  15 Sources · Strong      | +--------------------+ |      |
|      | +-----------------------+ |                        |      |
|      | | Ask a question…  Send | |                        |      |
|      | +-----------------------+ |                        |      |
+------+---------------------------+------------------------+------+
         <-------- chat >=420 -----> <---- companion >=440 --->
```

### Desktop — Graph open

```text
+------+---------------------------+------------------------+
| Nav  | Query                     | Graph             [×]  |
|      | … answer …                | Answer subgraph        |
|      | [Show on graph] (active)  | +--------------------+ |
|      |                           | |  (nodes lit)       | |
|      | composer                  | |  expand · Studio   | |
|      |                           | +--------------------+ |
+------+---------------------------+------------------------+
```

### Mobile — Sheet

```text
+---------------------------+
| Query (dimmed behind)     |
| …                         |
+---------------------------+
| ==== sheet handle ====    |
| Source · report.pdf  [×]  |
| PDF page 12 …             |
| Open full page            |
+---------------------------+
```

## Companion chrome

- **Tabs:** Source | Graph (disabled Graph until a message with entities exists;
  disabled Source until a location exists — or enable and show empty state).
- **Title:** document name or “Answer graph”.
- **Actions:** Close; Open full page / Open in Graph Studio; optional maximize
  (temporary full-bleed over chat, Esc restores).
- **Splitter:** vertical separator between chat and companion; keyboard operable.

## Interaction model

| Action | Result |
|--------|--------|
| Click citation (same tab) | Open/focus Source; navigate page/chunk; announce |
| Click citation again (same) | Update highlight only; no remount |
| Click different citation | Update location; keep pane open |
| Show on graph | Open/focus Graph; set message focus |
| Esc (pane focused / open) | Close pane; focus returns to trigger |
| Cmd+\ (or Ctrl+\) | Toggle companion (last kind) |
| Arrow keys on splitter | Resize by step (e.g. 16px / 10%) |
| Home / End on splitter | Min / max width of the current budget |
| Drag sheet down (mobile) | Dismiss |

## Focus order

1. Trigger (citation / button) activates pane.
2. Focus moves to companion header Close or first interactive control.
3. Tab cycles within companion until Esc or Close.
4. On close, focus returns to the triggering control (`aria-controls` /
   stored trigger ref).

## Live region

Single polite status region (reuse Query pattern):

- “Opened source: {title}, page {n}”
- “Opened answer graph for this message”
- “Companion closed”

## Accessibility (normative)

| Requirement | Spec |
|-------------|------|
| WCAG 2.2 AA | Contrast, target size, focus visible |
| Splitter | WAI-ARIA APG Window Splitter: `role="separator"`, orientation vertical, valuemin/max/now, arrow keys |
| Pane naming | `aria-label` on chat region and companion region |
| Sheet | Dialog semantics or `aria-modal` bottom sheet; focus trap while open on mobile |
| Reduced motion | No slide animation when `prefers-reduced-motion: reduce` |
| Graph fallback | `graph-as-table` when WebGL unavailable |
| PDF toolbar | Existing controls remain keyboard reachable |

## Visual states

| State | Source | Graph |
|-------|--------|-------|
| Loading | Skeleton / spinner in pane | Skeleton |
| Ready | PDF/text | Canvas or table |
| Empty | “Select a citation to view a source” | “This answer has no graph entities” |
| Error | Retry + Open full page | Retry + Open Studio |
| Partial | — | List unresolved entity names |

## Motion

- Open/close: 150–200ms opacity + width; 0ms if reduced motion.
- Sheet: 200ms translateY; 0ms if reduced motion.
- No layout thrash: CLS budget inherits SPEC-100 (companion uses `min-h-0` flex).

## i18n keys (prefix)

```text
query.companion.title
query.companion.source
query.companion.graph
query.companion.close
query.companion.openFullPage
query.companion.openStudio
query.companion.emptySource
query.companion.emptyGraph
query.companion.errorSource
query.companion.errorGraph
query.companion.announceOpenSource
query.companion.announceOpenGraph
query.companion.announceClosed
query.companion.resize
query.companion.unresolvedEntities
```

Locales: `en.json`, `fr.json`, `zh.json` — parity script must pass.

## Content design notes

- Prefer document title over raw UUID in chrome.
- Page badge uses existing `formatChunkPageBadge` (en-dash spans).
- Confidence / source chips in chat stay; companion does not duplicate the
  whole sources panel.

Cross-ref: [01-first-principles](01-first-principles.md) LAW-157-6,7,13 ·
[08-edge-cases](08-edge-cases.md) · [lenses/LENS-ux-ui.md](lenses/LENS-ux-ui.md).
