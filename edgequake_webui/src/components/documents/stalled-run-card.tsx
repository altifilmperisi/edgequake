/**
 * SPEC-155: honest card for a run that claims to be working but whose server
 * has been silent. No progress bar, no live stepper — those would be lies.
 * Always offers a way out: Cancel (works without a track_id) and Reprocess.
 */

"use client";

import { AlertTriangle, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  stageDisplayName,
  type IngestionRunView,
} from "@/lib/pipeline/ingestion-run-view";
import { formatSilence } from "@/lib/pipeline/run-liveness";

export interface StalledRunCardProps {
  run: IngestionRunView;
  onCancel?: () => void;
  onReprocess?: () => void;
  /** Cancel request in flight — prevents double submit. */
  isCancelling?: boolean;
}

/** "Last step: Preprocessing · 1%" — what we last heard, not what is happening. */
function lastKnownStep(run: IngestionRunView): string {
  const step = stageDisplayName(run.stage, run.sourceType);
  const pct =
    typeof run.progress01 === "number" && run.progress01 > 0
      ? ` · ${Math.round(run.progress01 * 100)}%`
      : "";
  return `Last step: ${step}${pct}`;
}

export function StalledRunCard({
  run,
  onCancel,
  onReprocess,
  isCancelling = false,
}: StalledRunCardProps) {
  const silence = formatSilence(run.stalledForMs ?? 0);
  return (
    <div
      className="flex items-start gap-2.5 rounded-md border border-amber-300/70 bg-background/90 px-2.5 py-2 dark:border-amber-800/70"
      data-testid="spec155-stalled-run-card"
      data-document-id={run.documentId}
      data-stalled="true"
      data-stage={run.stage}
    >
      <AlertTriangle
        className="mt-0.5 h-4 w-4 shrink-0 text-amber-600 dark:text-amber-400"
        aria-hidden="true"
      />
      <div className="min-w-0 flex-1 space-y-0.5">
        <div className="truncate text-sm font-medium text-foreground">
          {run.filename}
        </div>
        <p
          className="text-xs font-medium text-amber-700 dark:text-amber-300"
          data-testid="spec155-stalled-headline"
        >
          Stalled · no progress for {silence}
        </p>
        <p className="text-xs text-muted-foreground">{lastKnownStep(run)}</p>
      </div>
      <div className="flex shrink-0 items-center gap-1.5">
        {onReprocess ? (
          <Button
            type="button"
            variant="outline"
            size="sm"
            className="h-7 text-xs"
            onClick={onReprocess}
            data-testid="spec155-stalled-reprocess"
          >
            Reprocess
          </Button>
        ) : null}
        {onCancel ? (
          <Button
            type="button"
            variant="outline"
            size="sm"
            className="h-7 text-xs"
            onClick={onCancel}
            disabled={isCancelling}
            data-testid="spec086-run-cancel"
          >
            {isCancelling ? (
              <Loader2 className="mr-1 h-3 w-3 animate-spin" aria-hidden="true" />
            ) : null}
            Cancel
          </Button>
        ) : null}
      </div>
    </div>
  );
}

export default StalledRunCard;
