/**
 * SPEC-086: one progress presenter for all formats (ActiveRuns-style stepper).
 * PDF page detail is an optional nested slot under converting — not a second product.
 *
 * Cancelled/stopping: honest orange terminals with frozen progress (never Failed).
 */

"use client";

import { ServerStageStepper } from "@/components/documents/server-stage-stepper";
import { Progress } from "@/components/ui/progress";
import {
  formatQueueChrome,
  formatRunHeadline,
  shouldNestPdfPageMeter,
  shouldShowOverallMeter,
  stageDisplayName,
  type IngestionRunView,
} from "@/lib/pipeline/ingestion-run-view";
import { buildStageTimeline } from "@/lib/pipeline/stage-timeline";
import { useState, type ReactNode } from "react";

export { shouldNestPdfPageMeter, shouldShowOverallMeter };

export interface IngestionRunCardProps {
  run: IngestionRunView;
  /** Nested detail (e.g. PDF page N/M) — only while converting. */
  nestedDetail?: ReactNode;
  compact?: boolean;
  /** Cancel in-flight run (ActiveRuns / upload parity). */
  onCancel?: () => void;
  /**
   * Dismiss a terminal card (orphan failed shell delete, or cancelled ack hide).
   */
  onDismiss?: () => void;
  className?: string;
  "data-testid"?: string;
}

/** Orphan failed attention OR compact cancelled ack get Dismiss. */
export function canDismissTerminalRun(
  run: Pick<IngestionRunView, "stage" | "stageStatus">,
  hasDismissHandler: boolean,
): boolean {
  if (!hasDismissHandler) return false;
  if (run.stage === "cancelled" || run.stageStatus === "cancelled") return true;
  return run.stageStatus === "failed" || run.stage === "failed";
}

/** Failed attention runs get Dismiss (not Cancel). */
export function canDismissFailedRun(
  run: Pick<IngestionRunView, "stage" | "stageStatus">,
  hasDismissHandler: boolean,
): boolean {
  return (
    hasDismissHandler &&
    (run.stageStatus === "failed" || run.stage === "failed")
  );
}

/** Cancelled Working cards get Dismiss (local AR suppress — not document delete). */
export function canDismissCancelledRun(
  run: Pick<IngestionRunView, "stage" | "stageStatus">,
  hasDismissHandler: boolean,
): boolean {
  return (
    hasDismissHandler &&
    (run.stageStatus === "cancelled" || run.stage === "cancelled")
  );
}

function isCancelTerminal(
  run: Pick<IngestionRunView, "stage" | "stageStatus">,
): boolean {
  return (
    run.stageStatus === "cancelled" ||
    run.stage === "cancelled" ||
    run.stageStatus === "stopping" ||
    run.stage === "stopping"
  );
}

export function IngestionRunCard({
  run,
  nestedDetail,
  compact = false,
  onCancel,
  onDismiss,
  className,
  "data-testid": testId,
}: IngestionRunCardProps) {
  // SPEC-099: compact cards hide verbose message until expanded
  const [detailsOpen, setDetailsOpen] = useState(false);
  const timeline = buildStageTimeline(run);
  const admission = timeline.admissionPhase;
  const isAdmission = Boolean(admission);
  const cancelTerminal = isCancelTerminal(run);
  const overallPct = Math.round(timeline.overallProgress01 * 100);
  const stagePct =
    typeof timeline.stageProgress01 === "number"
      ? Math.round(timeline.stageProgress01 * 100)
      : undefined;
  const hasStageCounts = Boolean(run.counts && run.counts.total > 0);
  // LAW-IS2: nest only when list SSOT lacks page/figure counts (no second bar).
  const showPdfDetail =
    Boolean(nestedDetail) && shouldNestPdfPageMeter(run);
  const showOverall = shouldShowOverallMeter(run, isAdmission);
  const canCancel =
    Boolean(onCancel) &&
    !isAdmission &&
    !cancelTerminal &&
    run.stage !== "completed" &&
    run.stage !== "failed";
  const canDismissFailed = canDismissFailedRun(run, Boolean(onDismiss));
  const canDismissCancelled = canDismissCancelledRun(run, Boolean(onDismiss));
  const canDismiss = canDismissFailed || canDismissCancelled;

  const headlineClass = cancelTerminal
    ? run.stageStatus === "cancelled" || run.stage === "cancelled"
      ? "text-xs tabular-nums text-orange-700 dark:text-orange-300"
      : "text-xs tabular-nums text-orange-700/80 dark:text-orange-300/80"
    : "text-xs tabular-nums text-sky-700 dark:text-sky-300";

  const headlineText = cancelTerminal
    ? stageDisplayName(run.stage, run.sourceType)
    : isAdmission
      ? formatQueueChrome(run) || formatRunHeadline(run)
      : formatRunHeadline(run).replace(` · ${run.filename}`, "");

  return (
    <div
      className={
        className ??
        (compact
          ? cancelTerminal
            ? "space-y-1 rounded-md border border-orange-200/70 bg-orange-50/30 px-2 py-1.5 dark:border-orange-900/50 dark:bg-orange-950/20"
            : "space-y-1 rounded-md border border-border/60 bg-background/90 px-2 py-1.5"
          : cancelTerminal
            ? "space-y-2 rounded-md border border-orange-200/80 bg-orange-50/40 p-2.5 shadow-sm dark:border-orange-900/50 dark:bg-orange-950/20"
            : "space-y-2 rounded-md border border-border/80 bg-background p-2.5 shadow-sm")
      }
      data-testid={testId ?? "spec086-ingestion-run-card"}
      data-document-id={run.documentId}
      data-stage={run.stage}
      data-stage-status={run.stageStatus}
      data-source-type={run.sourceType}
      data-mode={run.mode ?? "full"}
      data-admission={cancelTerminal ? "cancelled" : (admission ?? "running")}
      data-compact={compact ? "true" : "false"}
    >
      <div className="flex items-center justify-between gap-2 text-sm">
        <span className="truncate font-medium text-foreground">
          {run.filename}
        </span>
        <div className="flex shrink-0 items-center gap-2">
          <span className={headlineClass} data-testid="spec048-run-headline">
            {headlineText}
          </span>
          {canCancel ? (
            <button
              type="button"
              className="text-xs text-muted-foreground hover:text-foreground underline-offset-2 hover:underline"
              onClick={onCancel}
              data-testid="spec086-run-cancel"
            >
              Cancel
            </button>
          ) : null}
          {canDismiss ? (
            <button
              type="button"
              className="text-xs text-muted-foreground hover:text-foreground underline-offset-2 hover:underline"
              onClick={onDismiss}
              title={
                canDismissCancelled
                  ? "Hide this cancelled run from Active Runs. The document stays in the list."
                  : "Remove this failed upload. Re-upload the file to try again."
              }
              data-testid="spec086-run-dismiss"
            >
              Dismiss
            </button>
          ) : null}
        </div>
      </div>

      {/* IS3: 4-phase strip by default (wire chips on Pipeline dialog). */}
      <ServerStageStepper run={run} variant="phases" />

      {showPdfDetail ? (
        <div data-testid="spec086-pdf-converting-detail" className="pt-0.5">
          {nestedDetail}
        </div>
      ) : null}

      {isAdmission ? (
        <div
          className="h-1.5 w-full overflow-hidden rounded bg-muted"
          data-testid="spec048-run-progress-indeterminate"
          data-admission={admission ?? undefined}
        >
          <div
            className={
              admission === "cleaning"
                ? "h-full w-1/3 animate-pulse rounded bg-rose-400/70"
                : "h-full w-1/3 animate-pulse rounded bg-amber-400/70"
            }
          />
        </div>
      ) : cancelTerminal ? (
        <div className="space-y-1.5" data-testid="spec086-cancel-progress-frozen">
          <div className="space-y-0.5" data-testid="spec048-overall-progress">
            <div className="flex items-center justify-between gap-2 text-xs text-muted-foreground">
              <span>Overall (frozen)</span>
              <span
                className="tabular-nums"
                data-testid="spec048-run-overall-pct"
              >
                {overallPct}%
              </span>
            </div>
            <Progress
              aria-label="Overall progress (frozen)"
              value={overallPct}
              className="h-1 [&_[data-slot=progress-indicator]]:bg-orange-400/70"
            />
          </div>
        </div>
      ) : (
        <div className="space-y-1.5">
          {hasStageCounts && typeof stagePct === "number" ? (
            <div className="space-y-0.5" data-testid="spec048-stage-progress">
              <div className="flex items-center justify-between gap-2 text-xs text-muted-foreground">
                <span>
                  This stage
                  {timeline.stageCountsLabel
                    ? ` · ${timeline.stageCountsLabel}`
                    : ""}
                </span>
                <span className="tabular-nums">{stagePct}%</span>
              </div>
              <Progress
              aria-label="Current stage progress"
                value={stagePct}
                className="h-1.5 [&_[data-slot=progress-indicator]]:bg-sky-500"
              />
            </div>
          ) : (
            <div
              className="h-1.5 w-full overflow-hidden rounded bg-muted"
              data-testid="spec048-run-progress-indeterminate"
            >
              <div className="h-full w-1/3 animate-pulse rounded bg-sky-400/70" />
            </div>
          )}

          {/* LAW-IS2: overall only when stage has no determinate N/M (one primary meter). */}
          {showOverall ? (
            <div className="space-y-0.5" data-testid="spec048-overall-progress">
              <div className="flex items-center justify-between gap-2 text-xs text-muted-foreground">
                <span>Overall (est.)</span>
                <span
                  className="tabular-nums"
                  data-testid="spec048-run-overall-pct"
                >
                  {overallPct}%
                </span>
              </div>
              <Progress
              aria-label="Overall progress"
                value={overallPct}
                className="h-1 [&_[data-slot=progress-indicator]]:bg-sky-400/80"
              />
            </div>
          ) : (
            <div
              className="sr-only"
              data-testid="spec048-overall-progress"
              data-collapsed="true"
            >
              Overall (est.) {overallPct}%
            </div>
          )}
        </div>
      )}

      {run.message && (!compact || detailsOpen) ? (
        <p
          className="text-xs text-muted-foreground line-clamp-2"
          data-testid="spec086-run-message"
        >
          {run.message}
        </p>
      ) : null}

      {compact && run.message && !detailsOpen ? (
        <button
          type="button"
          className="text-xs text-muted-foreground underline-offset-2 hover:underline"
          onClick={() => setDetailsOpen(true)}
          data-testid="spec099-run-expand-details"
        >
          Details
        </button>
      ) : null}

      {run.mode && run.mode !== "full" ? (
        <div
          className="text-xs text-muted-foreground"
          data-testid="spec048-run-mode"
        >
          Reprocess mode: {run.mode}
        </div>
      ) : null}

      {/* IS3: optional cost chip when spend is non-zero. */}
      {typeof run.costUsd === "number" && run.costUsd > 0 ? (
        <div
          className="text-xs tabular-nums text-muted-foreground"
          data-testid="spec091-run-cost"
        >
          Cost so far ${run.costUsd.toFixed(2)}
        </div>
      ) : null}
    </div>
  );
}

export default IngestionRunCard;
