/**
 * SPEC-099 — Documents layout stability (CLS / refresh jank).
 *
 * Thin wrappers over SPEC-100 shared helpers + Documents-specific pipeline signal.
 */
import {
  readSessionHint,
  shouldReserveSlot,
  writeSessionHint,
} from "@/lib/layout/cls-stability";
import type { PipelineStatus } from "@/types";

/**
 * Compact Active-run panel footprint (section title + one stepper card).
 * Sized to match live `ActiveRunsPanel` density so skeleton → live does not
 * shove the inventory table (measured ~200px for a single Extracting run).
 *
 * SPEC-155 intake strip: reserve matches a single compact run card (~120px)
 * rather than the old full nested PDF meter (~208px) — the strip budget and
 * inventory min-height now own the layout guarantee.
 */
export const FEEDBACK_ZONE_RESERVE_MIN_PX = 120;

/** Re-export intake strip budget SSOT for callers that already import here. */
export {
  INTAKE_STRIP_MAX_DVH,
  INTAKE_STRIP_MAX_REM,
  INVENTORY_MIN_PX,
  intakeStripMaxHeightCss,
} from "./intake-strip-state";

/** sessionStorage flag: last visit had live work (refresh reservation hint). */
export const LIVE_WORK_HINT_KEY = "edgequake.documents.liveWorkHint";

export function pipelineSuggestsLiveWork(
  pipelineStatus: PipelineStatus | null | undefined,
): boolean {
  if (!pipelineStatus) return false;
  if (pipelineStatus.is_busy) return true;
  const running =
    (pipelineStatus.running_tasks ?? 0) + (pipelineStatus.processing_tasks ?? 0);
  const queued =
    (pipelineStatus.pending_tasks ?? 0) + (pipelineStatus.queued_tasks ?? 0);
  return running > 0 || queued > 0;
}

export function shouldReserveFeedbackSlot(opts: {
  hasLiveWork: boolean;
  /** True only for cold load (no cached list yet). */
  isInitialLoading: boolean;
  pipelineStatus: PipelineStatus | null | undefined;
  liveWorkHint: boolean;
}): boolean {
  return shouldReserveSlot({
    hasContent: opts.hasLiveWork,
    isInitialLoading: opts.isInitialLoading,
    signal: pipelineSuggestsLiveWork(opts.pipelineStatus),
    hint: opts.liveWorkHint,
  });
}

export function readLiveWorkHint(): boolean {
  return readSessionHint(LIVE_WORK_HINT_KEY);
}

export function writeLiveWorkHint(active: boolean): void {
  writeSessionHint(LIVE_WORK_HINT_KEY, active);
}
