# 05 — UI specification

Parent: [README](README.md) · Journeys: [04](04-ux-journeys.md) · Frontend WP: [10](10-implementation-plan.md)

## Component inventory

| Component | File (target) | Role |
|-----------|---------------|------|
| `PageHealthStrip` | `components/documents/page-health-strip.tsx` | Quiet progress banner while page-reprocess track is live (idle: hidden) |
| `PagePickerGrid` | `components/documents/page-picker-grid.tsx` | Scrollable multi-select grid inside the dialog |
| `ReprocessPagesDialog` | `components/documents/reprocess-pages-dialog.tsx` | Pages → stage → preview/confirm |
| `BulkReprocessDialog` | `components/documents/bulk-reprocess-dialog.tsx` | Full / entities / pages (single PDF) |
| `ReprocessDialog` | `components/documents/reprocess-dialog.tsx` | Full / entities / pages for one doc |

## Status colours (tokens)

| Status | Tailwind hint | Meaning |
|--------|---------------|---------|
| `ok` | `bg-emerald-500` | Stage succeeded |
| `failed` | `bg-destructive` | Stage failed |
| `pending` | `bg-muted` | Not run / unknown |
| `running` | `bg-amber-500 animate-pulse` | In flight |
| `skipped` | `bg-muted-foreground/30` | N/A (e.g. no figures) |

## data-testid contract (live)

| Test ID | Element |
|---------|---------|
| `detail-page-reprocess-pages-button` | Detail header solid CTA |
| `detail-page-reprocess-button` | Detail outline full Reprocess |
| `detail-page-reprocess-progress` | Detail progress panel after enqueue |
| `list-reprocess-pages-action` | Documents row ⋮ menu item |
| `bulk-reprocess-dialog` / `bulk-reprocess-option-pages` / `bulk-reprocess-confirm` | Selection-bar Reprocess → pages |
| `reprocess-dialog` / `reprocess-option-pages` / `reprocess-dialog-confirm` | Full Reprocess → pages handoff |
| `reprocess-pages-dialog` | Pages dialog root |
| `page-picker-grid` | Picker root |
| `page-picker-filter-all\|failed\|selected` | Picker filters |
| `page-health-tile-{n}` | Tile (`data-page`, `data-status`) inside picker |
| `reprocess-pages-range-input` / `reprocess-pages-apply-range` | Range field + Apply |
| `select-failed-pages` / `select-all-pages` / `clear-page-selection` | Quick actions |
| `stage-card-parse\|figures\|entities` | Stage radios (`data-locked`) |
| `reprocess-pages-preview` / `reprocess-impact-preview` / `reprocess-pages-confirm` | Preview + start |
| `never-downgrade-note` / `reprocess-ready-summary` / `reprocess-selected-count` | Confirm chrome |
| `page-health-strip` / `page-health-progressive` | In-flight only on detail |
| `spec051-reprocess-progress-panels` / `spec051-reprocess-panel` | List feedback-zone pin |

## i18n keys (en/fr/zh)

```text
documents.pageHealth.*
documents.reprocessDialog.pagesDescription
documents.reprocessDialog.pagesBadge
documents.reprocessDialog.pagesContinue
documents.reprocessDialog.pagesUnavailableHint
```

## Pure libs (Vitest)

| Module | Responsibility |
|--------|----------------|
| `lib/documents/page-range.ts` | Parse/format `1-3,7` |
| `lib/documents/page-health.ts` | Aggregate failed counts, worst status |
| `lib/documents/reprocess-stages.ts` | Stage closure mirror of backend |

Cross-refs: e2e [11](11-test-plan.md).
