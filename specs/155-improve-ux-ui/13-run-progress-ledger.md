# SPEC-155 note — Run progress ledger (Prepare / Extract that never lie)

**Date:** 2026-10-02  
**Status:** Implemented

## Problem

Active Runs progress was a single lossy snapshot: one `stage_message` string plus
one `stage_progress` float. Both ends regex-parsed an `N/M` counter out of the
message. That made the unit, numerator, and even *which item* the number referred
to change from event to event.

Symptoms:

1. **Prepare dropped to zero when figure analysis started** — pages `92/92` then
   figures `1/12` shared one slot; the UI preferred the parsed figure ratio.
2. **Out-of-order pages/chunks made the bar non-linear** — `on_page_start` and
   `chunk_idx + 1` were parsed as "done"; concurrent extraction showed `92/92`
   while only ~60% had completed.
3. **Refresh lost state** — monotonic smoothing lived only in the React Query
   cache / WS message regex guard.

## Fix (first principles)

Progress is `done / total` per unit of work, per phase, and `done` only grows.
It is recorded as typed counters by the code doing the work, persisted on
document metadata as `run_progress`, and served as a projection. The UI renders
it as a pure function — no message parsing, no client memory required for
correctness.

### Backend

- `services/run_progress.rs` — pure ledger + monotonic reducer + legacy
  projections (`stage_progress`, `progress_counts`, `stage_message`).
- `services/run_progress_writer.rs` — ordered, coalesced (~250 ms) writer with
  final flush and seq fencing.
- PDF pages/figures, chunk extract (completed + in-flight), embed, and graph-merge
  callbacks route through the ledger.
- Ledger survives convert→insert retarget; resets on reprocess; clears on
  terminal cancel/fail.
- Exposed on document list + track progress DTOs (`run_progress`).

### Frontend

- `lib/pipeline/run-progress.ts` — `phaseFill`, `clampMonotonic`, legacy synth.
- `IngestionRunView.phases` / `runProgress`; phase-segments + stage-timeline read
  the ledger.
- PhaseStrip / RunCaption: named sub-counters, done-segment summaries, reduced
  motion.
- WS cache + `merge-monotonic-list` clamp the ledger; regex-on-previous-message
  guard removed.

## Verification

- Rust: `run_progress` unit tests + `e2e_spec155_run_progress`.
- Vitest: `run-progress.test.ts`, phase-segments regressions.
- Playwright hermetic: Prepare-fill-with-figures, reload-identical caption/fill.
- OpenAPI: `make codegen-openapi-refresh` (+ `spec027` snapshot write).
