/**
 * SPEC-086 / SPEC-099 — ActiveRuns panel admission (pure SSOT).
 *
 * Ordinary Failed docs stay in the inventory table; only live/queued/cancelled
 * work and orphan failed attention shells open the feedback zone.
 */

import type { IngestionRunView } from "@/lib/pipeline/ingestion-run-view";

export function isOrphanFailedAttention(run: IngestionRunView): boolean {
  return (
    run.stageStatus === "failed" &&
    /re-upload|interrupted|orphaned staging|document received,\s*starting processing|prior interrupted/i.test(
      run.message,
    )
  );
}

/** SPEC-155: claims to be working but the server has been silent too long. */
export function isStalledAttention(run: IngestionRunView): boolean {
  return typeof run.stalledForMs === "number";
}

/**
 * Live work + brief cancelled acknowledgement (TTL applied later).
 * Stopping always stays; cancelled passes partition then retention filter.
 */
export function isLiveWorkingOrQueued(run: IngestionRunView): boolean {
  if (isOrphanFailedAttention(run) || isStalledAttention(run)) return false;
  // LAW-28: keep Stopping… / Cancelled on ActiveRuns (TTL applied later).
  if (
    run.stage === "stopping" ||
    run.stage === "cancelled" ||
    run.stageStatus === "stopping" ||
    run.stageStatus === "cancelled"
  ) {
    return true;
  }
  return (
    run.stageStatus === "active" ||
    run.stageStatus === "pending" ||
    (Boolean(run.trackId) &&
      run.stage !== "completed" &&
      run.stage !== "failed" &&
      run.stageStatus !== "failed")
  );
}

/** Split live work from orphan attention shells (testable SSOT). */
export function partitionActiveRuns(runs: IngestionRunView[]): {
  working: IngestionRunView[];
  attention: IngestionRunView[];
} {
  const working: IngestionRunView[] = [];
  const attention: IngestionRunView[] = [];
  for (const run of runs) {
    if (isOrphanFailedAttention(run) || isStalledAttention(run)) {
      attention.push(run);
    } else if (isLiveWorkingOrQueued(run)) {
      working.push(run);
    }
  }
  return { working, attention };
}

/** True when ActiveRunsPanel would paint at least one card (not return null). */
export function hasPanelVisibleActiveRuns(runs: IngestionRunView[]): boolean {
  const { working, attention } = partitionActiveRuns(runs);
  return working.length > 0 || attention.length > 0;
}
