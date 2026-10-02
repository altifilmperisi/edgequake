/**
 * SPEC-091 IS3 — default 4-phase ActiveRuns strip (Admit / Prepare / Extract / Materialize).
 * Full wire UnifiedStage chips stay on Pipeline / Details (ServerStageStepper).
 *
 * Two looks, one component:
 *  - chips (default): label chips separated by chevrons.
 *  - segmented (`progress` given): the strip IS the progress bar — each phase
 *    owns a quarter of one track and the active segment fills with the stage
 *    percentage. No second full-width meter underneath.
 */

"use client";

import { cn } from "@/lib/utils";
import { ChevronRight } from "lucide-react";
import { Fragment } from "react";
import {
  mapWireStageToPhase,
  PHASE_STRIP_LABELS,
  PHASE_STRIP_ORDER,
  type IngestionRunView,
} from "@/lib/pipeline/ingestion-run-view";
import {
  buildPhaseSegments,
  phaseSegmentStatus,
  type PhaseSegment,
  type PhaseSegmentStatus,
} from "@/lib/pipeline/phase-segments";
import { buildStageTimeline } from "@/lib/pipeline/stage-timeline";

function statusClasses(status: PhaseSegmentStatus): string {
  switch (status) {
    case "done":
      return "text-emerald-700 dark:text-emerald-400";
    case "active":
      return "bg-sky-100 text-sky-800 dark:bg-sky-950 dark:text-sky-200";
    case "failed":
      return "bg-rose-100 text-rose-800 dark:bg-rose-950 dark:text-rose-200";
    default:
      return "text-muted-foreground";
  }
}

function dotClasses(status: PhaseSegmentStatus): string {
  switch (status) {
    case "done":
      return "bg-emerald-500";
    case "active":
      return "bg-sky-500 animate-pulse";
    case "failed":
      return "bg-rose-500";
    default:
      return "bg-muted-foreground/40";
  }
}

function fillClasses(status: PhaseSegmentStatus): string {
  switch (status) {
    case "done":
      return "bg-emerald-500/80";
    case "failed":
      return "bg-rose-500";
    default:
      return "bg-sky-500";
  }
}

/** Phase-strip progress input: `stagePct` undefined ⇒ indeterminate. */
export interface PhaseStripProgress {
  stagePct?: number;
}

export interface PhaseStripProps {
  run: IngestionRunView;
  /** Present ⇒ segmented progress look; absent ⇒ plain chips. */
  progress?: PhaseStripProgress;
  className?: string;
}

/** The bar under one phase label. */
function SegmentTrack({ segment }: { segment: PhaseSegment }) {
  const { status, fillPct, indeterminate } = segment;
  const isActive = status === "active";
  return (
    <div
      className="relative h-1.5 w-full overflow-hidden rounded-full bg-muted"
      role={isActive && !indeterminate ? "progressbar" : undefined}
      aria-label={
        isActive && !indeterminate ? "Current stage progress" : undefined
      }
      aria-valuemin={isActive && !indeterminate ? 0 : undefined}
      aria-valuemax={isActive && !indeterminate ? 100 : undefined}
      aria-valuenow={
        isActive && !indeterminate ? Math.round(fillPct) : undefined
      }
      data-testid={
        isActive && indeterminate ? "spec048-run-progress-indeterminate" : undefined
      }
      data-segment-fill={Math.round(fillPct)}
    >
      {indeterminate ? (
        <div className="h-full w-1/3 animate-pulse rounded-full bg-sky-400/70 motion-reduce:animate-none" />
      ) : (
        <div
          className={cn(
            "h-full rounded-full transition-[width] duration-500 ease-out motion-reduce:transition-none",
            fillClasses(status),
          )}
          style={{ width: `${fillPct}%` }}
        />
      )}
    </div>
  );
}

function PhaseLabel({
  phase,
  status,
  wireStage,
  chip,
  summary,
}: {
  phase: (typeof PHASE_STRIP_ORDER)[number];
  status: PhaseSegmentStatus;
  wireStage: string;
  /** Chips keep their pill background; segmented labels stay flat. */
  chip: boolean;
  summary?: string | null;
}) {
  return (
    <span
      className={cn(
        "inline-flex min-w-0 items-center gap-1.5 text-xs transition-colors",
        chip ? "rounded-md px-2 py-0.5" : "leading-none",
        status === "active" && "font-medium",
        chip
          ? statusClasses(status)
          : status === "active"
            ? "text-sky-800 dark:text-sky-200"
            : status === "failed"
              ? "text-rose-700 dark:text-rose-300"
              : status === "done"
                ? "text-emerald-700 dark:text-emerald-400"
                : "text-muted-foreground",
      )}
      data-testid={`spec091-phase-${phase}`}
      data-state={status}
      data-wire-stage={wireStage}
      data-summary={summary ?? undefined}
      title={summary ? `${PHASE_STRIP_LABELS[phase]}: ${summary}` : PHASE_STRIP_LABELS[phase]}
      aria-current={status === "active" ? "step" : undefined}
      aria-label={
        summary
          ? `${PHASE_STRIP_LABELS[phase]}: ${summary}`
          : PHASE_STRIP_LABELS[phase]
      }
    >
      <span
        className={cn(
          "h-1.5 w-1.5 shrink-0 rounded-full",
          dotClasses(status),
          status === "active" && "motion-reduce:animate-none",
        )}
      />
      <span className="truncate">{PHASE_STRIP_LABELS[phase]}</span>
    </span>
  );
}

/** Screen-reader + e2e wire markers (skipped converting omitted). */
function WireMarkers({ run }: { run: IngestionRunView }) {
  const wireSteps = buildStageTimeline(run).steps;
  return (
    <span className="sr-only" aria-hidden="true">
      {wireSteps.map((step) => (
        <span
          key={step.id}
          data-testid={`spec048-stage-${step.id}`}
          data-state={step.status}
          data-wire-stage={step.id}
        />
      ))}
    </span>
  );
}

export function PhaseStrip({ run, progress, className }: PhaseStripProps) {
  const active = mapWireStageToPhase(run.stage);
  const failed = run.stageStatus === "failed" || run.stage === "failed";

  if (progress) {
    const segments = buildPhaseSegments(run, progress.stagePct);
    return (
      <div
        className={cn("grid grid-cols-4 gap-2", className)}
        data-testid="spec091-phase-strip"
        data-variant="segmented"
        data-wire-stage={run.stage}
        data-phase={active}
      >
        {segments.map((segment) => (
          <div
            key={segment.phase}
            className="min-w-0 space-y-1.5"
            title={
              segment.status === "done" && segment.summary
                ? segment.summary
                : undefined
            }
          >
            <PhaseLabel
              phase={segment.phase}
              status={segment.status}
              wireStage={run.stage}
              chip={false}
              summary={segment.summary}
            />
            <SegmentTrack segment={segment} />
          </div>
        ))}
        <WireMarkers run={run} />
      </div>
    );
  }

  // IS-AC-06: preserve wire stage ids as data attributes / contract hooks.
  // Include skipped (e.g. merge mode) so e2e can assert data-state=skipped.
  return (
    <div
      className={cn("flex flex-wrap items-center gap-1.5 text-xs", className)}
      data-testid="spec091-phase-strip"
      data-wire-stage={run.stage}
      data-phase={active}
    >
      {PHASE_STRIP_ORDER.map((phase, index) => (
        <Fragment key={phase}>
          {index > 0 ? (
            <ChevronRight
              className="h-3 w-3 shrink-0 text-muted-foreground/40"
              aria-hidden="true"
            />
          ) : null}
          <PhaseLabel
            phase={phase}
            status={phaseSegmentStatus(phase, active, failed)}
            wireStage={run.stage}
            chip
          />
        </Fragment>
      ))}
      <WireMarkers run={run} />
    </div>
  );
}
