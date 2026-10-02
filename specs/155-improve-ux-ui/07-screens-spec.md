# 07 — Screens specification

Parent: [README](README.md) · Surfaces: [02](02-surfaces.md) · Design: [05](05-design-system-spec.md) · Next: [08-data-contract](08-data-contract.md)

Every screen uses `PageShell` + `PageHeader` unless noted (Graph Studio,
Query fullscreen chat). State columns: L loading · E empty · R error · P partial · O offline.

## Dashboard `/`

**WHY:** Orient + jump to primary work.

```text
  +------------------------------------------+
  | Workspace · Overview          [Upload]   |
  +------------------------------------------+
  | Docs | Entities | Rel                     |  (3 metrics)
  +------------------------------------------+
  | Recent documents          health pill    |
  | … or FirstRunWelcome when zero docs      |
  +------------------------------------------+
```

| State | Behaviour |
|-------|-----------|
| L/E/R | Skeleton; welcome CTA; ErrorState with retry (today: failed → “0”) |
| O | ConnectionIndicator |

**Code:** wire `isError`; fix deep link → `/documents/[id]` or open preview;
remove dead `WorkspaceUrlUpdater` duplicate; drop decorative gradient on
stats-card; i18n section labels. Findings: P03, D02.

## Documents `/documents`

**WHY:** Ingest and monitor corpus. **Do not reopen** SPEC-099 status/feedback
design.

```text
  PageHeader Documents [Upload]
  Toolbar (selection mode) · Dropzone (collapsible)
  Feedback zone (existing)
  Table / Responsive cards  ·  Preview sheet
```

| State | L E R P O | Keep existing; ensure ErrorState; WS via ConnectionIndicator |

**Code (W7):** SRP-split `document-manager` → hooks + composition ≤300/file;
prop drill → context already started (`DocumentsActionsProvider`); use
`DataTableResponsive`; honour `?id=` for preview. Finding P04.

## Document detail `/documents/[id]`

**WHY:** Read PDF ↔ markdown; lineage; reprocess.

```text
  PageHeader filename [Download] [Graph] [Reprocess]
  +-------------+------------------+
  | PDF | MD    | Details tabs     |
  | resizable   | meta · lineage   |
  +-------------+------------------+
```

| State | Distinguish 404 vs network; one layout via `useMediaQuery` (no double mount) |

**Code:** reuse `ui/resizable-panel`; aria-labels on icon buttons; single
export menu; LifecycleBadge DRY; humanise ErrorContent. Finding P05.

## Query `/query`

**WHY:** Ask → streamed grounded answer that can be verified and continued.

```text
  Header: title · history toggle · settings (no mode strip)
  Messages: unboxed answer · stage timeline · early source chips · actions
  Composer card: attach · scope · mode menu (Smart default) · model · Send/Stop
  History: drawer < xl · docked panel xl+ · date groups · search all
```

| State | Idle · Retrieving · Reading · Writing · Stopped · Failed · Complete · Empty |

**Behaviour (W7Q):**
- Composer always enabled; Enter while streaming queues; Esc/Stop keeps partial.
- StageTimeline from `stage`/`thinking` SSE; source chips on `context` before tokens.
- One `role="status"` live region (stage + completion only) — not `role="log"` on list.
- CitationPopover on hover/focus; keyboard-complete Sources; no 7.5rem dead slot.
- Inline ErrorState + Retry; failed/stopped turns persist.
- ModeMenu in composer (outcome labels); drafts per conversation; `/` and `@` shortcuts.
- History: date groups, all-conversation search, undo delete, single mount, roving focus.
- Empty: no gradient; corpus-derived suggestions; first-run when zero docs.

**Code modules:** `composer/`, `message/`, `citations/`, `history/`,
`lib/query/build-chat-request`, `stream-session-reducer`, `conversation-recovery`,
`hooks/use-query-stream-session`, `use-stick-to-bottom`.

Findings Q01–Q27. Backend: Stage SSE (B1), message feedback (B2), abort partial (B3).

## Graph `/graph`

See [06-graph-studio-spec](06-graph-studio-spec.md). PageHeader optional
overlay; canvas is primary.

## Pipeline `/pipeline`

**WHY:** Observe ingestion fleet (not a “back to documents” child).

```text
  PageHeader Pipeline [Refresh]
  Snapshot cards (one poller / visibility pause)
  Advanced details collapsible
```

| State | L E R — today missing R |

**Code:** `usePipelineSnapshot` SSOT; i18n; demote toast-on-invalidate;
align with dialog or deep-link dialog → page. Finding P06.

## Costs `/costs`

**WHY:** Honest spend visibility.

```text
  PageHeader Costs [Export]
  Period selector → real API range
  Summary · BudgetIndicator(real) · Trend (accessible chart)
```

| State | L E R |

**Code:** pass period into `useWorkspaceCostSummary` / history; remove
hard-coded zeros or label “n/a”; CSV quoting; aria for chart. Finding P01.

## Workspace `/workspace`

**WHY:** Configure models & lifecycle for current workspace.

```text
  PageHeader + tabs: Overview | Models | Danger
  Remove dead isEditing={false} props
  Delete: typed-name confirm (parity with Clear All)
```

Finding P09.

## Settings `/settings`

**WHY:** Appearance, language, providers, admin.

```text
  +--------+----------------------+
  | Nav    | Section panel        |   SPEC-029 PD-01
  | appear |                      |
  | LLM    | auto-save indicator  |
  | PDF    | (not toast spam)     |
  | Admin  |                      |
  +--------+----------------------+
```

Languages ⊆ locales; Import keyboard-accessible; labels `htmlFor`. Finding P07.

## Knowledge `/knowledge`

**WHY:** Inject free-form knowledge (rename UI to “Injections” or
“Knowledge entries” to avoid clash with Knowledge Graph).

```text
  PageHeader + Create dialog component
  List with StatusBadge; detail with honest not-found vs error
```

Replace module-level poll with react-query; keyboard dropzone. Finding P08.

## API Explorer `/api-explorer`

PageHeader h1; i18n; ErrorState if OpenAPI fetch fails. Finding P11.

## Login `/login`

```text
  Minimal card — no gradients
  autocomplete username/current-password
  show-password toggle
  single error surface (alert OR toast, not both)
  default landing: / (dashboard) unless ?next=
```

Finding P12.

## Parallel `/w/[slug]/*`

**WHY:** Workspace-scoped URLs must equal dashboard security & chrome.

```text
  MUST: AuthGuard, BackendStatusBanner, ApiErrorBoundary, FirstRunWizard,
        WorkspaceUrlSync, h-dvh, router.replace (not push)
  DRY: share DashboardShell component — no forked layout
```

Finding P02.

## Screen wave map

| Screen | Wave | Primary findings |
|--------|------|------------------|
| Login + shell chrome | W1 | P12, S01–S05, D01 |
| Graph Studio | W4–W6 | G*, Q05 |
| Dashboard, Docs, Detail, Query, Pipeline, Costs, Workspace, Settings, Knowledge, API, w-slug | W7 | P*, Q*, S04 |
| i18n pass all | W8 | I* |

Cross-ref: [10-implementation-plan](10-implementation-plan.md) · [LENS-ux-ui](lenses/LENS-ux-ui.md).
