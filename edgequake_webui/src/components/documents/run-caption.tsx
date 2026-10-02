/**
 * Single caption line under the segmented phase bar:
 *   [headline ………………………]  [~35%]  [Details ▾]
 *
 * WHY: the headline used to float top-right and the percentage at the far end
 * of a separate bar. Keeping both on one line, directly under the segment they
 * describe, makes the progress read as one thing.
 */

"use client";

import { cn } from "@/lib/utils";
import { ChevronDown } from "lucide-react";

export interface RunCaptionProps {
  headlineText: string;
  pct: number;
  /** Overall estimate (no determinate N/M) — painted with a leading "~". */
  estimated: boolean;
  /** Counts / description for assistive tech (not painted). */
  srLabel?: string;
  overallPct: number;
  /** Details toggle, only when the run has a message to reveal. */
  details?: { open: boolean; onToggle: () => void };
  className?: string;
}

export function RunCaption({
  headlineText,
  pct,
  estimated,
  srLabel,
  overallPct,
  details,
  className,
}: RunCaptionProps) {
  return (
    <>
      <div
        className={cn(
          "flex items-center justify-between gap-3 text-xs",
          className,
        )}
        data-testid={
          estimated ? "spec048-overall-progress" : "spec048-stage-progress"
        }
        data-collapsed={estimated ? "false" : undefined}
      >
        {srLabel ? <span className="sr-only">{srLabel}</span> : null}
        <span
          className="min-w-0 truncate tabular-nums text-sky-700 dark:text-sky-300"
          data-testid="spec048-run-headline"
        >
          {headlineText}
        </span>
        <span className="flex shrink-0 items-center gap-2">
          <span
            className="font-medium tabular-nums text-foreground/80"
            data-testid={
              estimated ? "spec048-run-overall-pct" : "spec048-run-stage-pct"
            }
            title={estimated ? "Estimated overall progress" : undefined}
          >
            {estimated ? "~" : ""}
            {pct}%
          </span>
          {details ? (
            <button
              type="button"
              className="inline-flex items-center gap-0.5 rounded text-muted-foreground transition-colors hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
              onClick={details.onToggle}
              aria-expanded={details.open}
              data-testid="spec099-run-expand-details"
            >
              {details.open ? "Hide" : "Details"}
              <ChevronDown
                className={cn(
                  "h-3 w-3 transition-transform motion-reduce:transition-none",
                  details.open && "rotate-180",
                )}
                aria-hidden="true"
              />
            </button>
          ) : null}
        </span>
      </div>
      {!estimated ? (
        // LAW-IS2: overall collapses while stage counts exist (kept for AT/e2e).
        <div
          className="sr-only"
          data-testid="spec048-overall-progress"
          data-collapsed="true"
        >
          Overall (est.) {overallPct}%
        </div>
      ) : null}
    </>
  );
}

export default RunCaption;
