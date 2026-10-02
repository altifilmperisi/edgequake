/**
 * One-line progress meter for ingestion run cards: [label] [bar] [pct].
 *
 * WHY: the percentage used to sit on a separate caption row above the bar
 * ("This stage · 1/30 pages · 3%" … "3%"), repeating the headline and
 * visually detaching the number from the bar it describes. The counts stay in
 * the DOM as screen-reader text so assistive tech and e2e contracts keep them.
 */

"use client";

import { Progress } from "@/components/ui/progress";
import { cn } from "@/lib/utils";

export interface RunMeterRowProps {
  /** Short visible caption (omit when the headline already names the stage). */
  label?: string;
  /** Counts / description kept for screen readers (not painted). */
  srLabel?: string;
  /** 0–100 */
  pct: number;
  ariaLabel: string;
  /** Tailwind arbitrary-variant colour for the indicator. */
  indicatorClassName: string;
  /** Bar height utility, default `h-2`. */
  barClassName?: string;
  testId: string;
  pctTestId?: string;
  className?: string;
}

export function RunMeterRow({
  label,
  srLabel,
  pct,
  ariaLabel,
  indicatorClassName,
  barClassName = "h-2",
  testId,
  pctTestId,
  className,
}: RunMeterRowProps) {
  return (
    <div
      className={cn(
        "flex items-center gap-2 text-xs text-muted-foreground",
        className,
      )}
      data-testid={testId}
    >
      {label ? (
        <span className="w-28 shrink-0 truncate">{label}</span>
      ) : null}
      {srLabel ? <span className="sr-only">{srLabel}</span> : null}
      <Progress
        aria-label={ariaLabel}
        value={pct}
        className={cn("flex-1", barClassName, indicatorClassName)}
      />
      <span
        className="w-9 shrink-0 text-right font-medium tabular-nums text-foreground/80"
        data-testid={pctTestId}
      >
        {pct}%
      </span>
    </div>
  );
}

export default RunMeterRow;
