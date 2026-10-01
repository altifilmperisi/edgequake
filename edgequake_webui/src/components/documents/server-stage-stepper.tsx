/**
 * SPEC-048: Server-aligned stage stepper with per-step detail progress.
 *
 * Renders full UnifiedStage timeline (skip/fail/active/done) and shows
 * countable detail on the active (or failed) step.
 */

"use client";

import { AdmissionPhaseRow } from "@/components/documents/admission-phase-row";
import { PhaseStrip } from "@/components/documents/phase-strip";
import { cn } from "@/lib/utils";
import type { IngestionRunView } from "@/lib/pipeline/ingestion-run-view";
import {
  buildStageTimeline,
  formatStepDetailLine,
  type StageStepStatus,
} from "@/lib/pipeline/stage-timeline";

interface ServerStageStepperProps {
  run: IngestionRunView;
  /** When true, hide skipped converting for non-PDF (still shown muted by default). */
  hideSkipped?: boolean;
  /**
   * `phases` (default): SPEC-091 IS3 4-phase strip for ActiveRuns.
   * `wire`: full UnifiedStage chips (Pipeline / Details).
   */
  variant?: "phases" | "wire";
  className?: string;
}

function statusClasses(status: StageStepStatus): string {
  switch (status) {
    case "done":
      return "text-emerald-700 dark:text-emerald-400";
    case "active":
      return "bg-sky-100 text-sky-800 dark:bg-sky-950 dark:text-sky-200";
    case "failed":
      return "bg-rose-100 text-rose-800 dark:bg-rose-950 dark:text-rose-200";
    case "cancelled":
      return "bg-orange-100 text-orange-800 dark:bg-orange-950 dark:text-orange-200";
    case "skipped":
      return "text-muted-foreground/50 line-through decoration-muted-foreground/40";
    default:
      return "text-muted-foreground";
  }
}

function dotClasses(status: StageStepStatus): string {
  switch (status) {
    case "done":
      return "bg-emerald-500";
    case "active":
      return "bg-sky-500 animate-pulse";
    case "failed":
      return "bg-rose-500";
    case "cancelled":
      return "bg-orange-500";
    case "skipped":
      return "bg-muted-foreground/25";
    default:
      return "bg-muted-foreground/40";
  }
}

export function ServerStageStepper({
  run,
  hideSkipped = false,
  variant = "phases",
  className,
}: ServerStageStepperProps) {
  const timeline = buildStageTimeline(run);
  const steps = hideSkipped
    ? timeline.steps.filter((s) => s.status !== "skipped")
    : timeline.steps;
  const active = steps.find(
    (s) =>
      s.status === "active" ||
      s.status === "failed" ||
      s.status === "cancelled",
  );
  const detailLine = formatStepDetailLine(active?.detail);
  const admissionPhase = timeline.admissionPhase;
  const isCancelTerminal =
    run.stageStatus === "cancelled" ||
    run.stage === "cancelled" ||
    run.stageStatus === "stopping" ||
    run.stage === "stopping";

  return (
    <div
      className={cn("space-y-2", className)}
      data-testid="spec048-server-stage-stepper"
      data-stage={run.stage}
      data-admission={admissionPhase ?? "running"}
      data-overall-progress={timeline.overallProgress01.toFixed(3)}
      data-variant={variant}
    >
      {admissionPhase ? (
        <AdmissionPhaseRow
          phase={admissionPhase}
          stageMessage={run.message}
          variant="pill"
        />
      ) : null}

      {variant === "phases" ? (
        <PhaseStrip run={run} />
      ) : (
      <div className="flex flex-wrap items-center gap-1.5 text-xs">
        {steps.map((step) => (
          <span
            key={step.id}
            className={cn(
              "inline-flex items-center gap-1 rounded px-1.5 py-0.5",
              statusClasses(step.status),
            )}
            data-testid={`spec048-stage-${step.id}`}
            data-state={step.status}
            title={
              step.status === "skipped"
                ? `${step.label} (skipped)`
                : step.label
            }
          >
            <span
              className={cn("h-1.5 w-1.5 rounded-full", dotClasses(step.status))}
            />
            {step.label}
            {step.status === "skipped" ? (
              <span className="sr-only">skipped</span>
            ) : null}
          </span>
        ))}
      </div>
      )}

      {active && detailLine && !isCancelTerminal ? (
        <div
          className={cn(
            "rounded-md border px-2 py-1.5 text-xs tabular-nums",
            active.status === "failed"
              ? "border-rose-200 bg-rose-50/80 text-rose-800 dark:border-rose-900 dark:bg-rose-950/40 dark:text-rose-200"
              : "border-sky-200/80 bg-sky-50/60 text-sky-900 dark:border-sky-900 dark:bg-sky-950/30 dark:text-sky-100",
          )}
          data-testid="spec048-step-detail"
          data-stage={active.id}
        >
          <span className="font-medium">{active.label}</span>
          <span className="mx-1.5 text-muted-foreground">·</span>
          <span>{detailLine}</span>
        </div>
      ) : null}
    </div>
  );
}

export default ServerStageStepper;
