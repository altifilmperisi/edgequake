# 04 — UX journeys

Parent: [README](README.md) · REQs: [03](03-requirements.md) · UI: [05](05-ui-spec.md)

## WHY (UX)

Users currently experience failure as a document-level red badge. They need a **page-native** mental model: open a dedicated dialog, select pages (range or grid), preview, confirm — with zero surprise about what will be destroyed.

## Quiet header (committed)

Detail pages do **not** show an idle page-health strip or page-number selector. Selection lives only in `ReprocessPagesDialog` (`PagePickerGrid`). While a page-reprocess track is live, a slim progressive banner may appear under the header.

## Primary journey — Fix two bad pages

```text
  1. Open /documents/{id} (completed PDF)
  2. Click header "Reprocess specific pages"
  3. Dialog:
       Range: 3-5 (Apply) or click tiles / Select failed
       Filters: All · Failed · Selected
       Stage cards:
         ( ) Parse (OCR)         → also Figures + Entities
         ( ) Figures & charts    → also Entities
         (•) Entities
       Optional Preview (dry_run) → impact card
  4. Start reprocess → dialog closes
  5. Detail progress slot + quiet progressive banner; toast queued
```

## Entry points

| Entry | Behaviour |
|-------|-----------|
| Detail header | Solid "Reprocess specific pages" → pages dialog |
| Detail outline Reprocess | Full / Entities / **pages** → pages dialog when pages chosen |
| List row ⋮ menu | "Reprocess specific pages" → pages dialog; enqueue pins feedback-zone progress |
| Selection-bar Reprocess | Bulk dialog; **pages** when exactly one eligible PDF |
| Empty markdown state | Same CTA when `canReprocessPages` |

## Empty / error states

```text
  No page markers in markdown
    → strip hidden; dialog explains "Upload a PDF with page markers"

  Document processing
    → dialog disabled; show "Wait for current job or cancel"

  409 Conflict
    → toast: "Another task is running on this document"

  422 Unprocessable
    → inline alert with reason (not a PDF / no vision provider)
```

## ASCII wireframe — dialog

```text
  ┌─ Reprocess pages ─────────────────────────────────────┐
  │ Pages                                                 │
  │ [3 ×] [5 ×]   Range: [ 3,5________ ]  Select failed   │
  │                                                       │
  │ What to re-run                                        │
  │ ┌─────────────────────┐  ┌─────────────────────┐      │
  │ │ ○ Parse text        │  │ ○ Figures & charts  │      │
  │ │   Re-OCR pages      │  │   Locked if Parse   │      │
  │ │   + figures+ents    │  │   selected          │      │
  │ └─────────────────────┘  └─────────────────────┘      │
  │ ┌─────────────────────┐                               │
  │ │ ● Entities only     │                               │
  │ │   Keep markdown     │                               │
  │ └─────────────────────┘                               │
  │                                                       │
  │ Impact                                                │
  │  2 pages · 4 dirty chunks · 118 reused · ~0 vision    │
  │  ⓘ Failed pages keep their current content            │
  │                                                       │
  │              [ Cancel ]  [ Reprocess 2 pages ]        │
  └───────────────────────────────────────────────────────┘
```

## Accessibility

- Strip tiles are buttons with `aria-pressed` when selected.
- Stage cards are radiogroup.
- Dialog traps focus; Esc cancels.
- Colour is not the only status signal (icon + text).

Cross-refs: UI tokens/testids [05](05-ui-spec.md).
