"use client";

import { cn } from "@/lib/utils";
import { Search, BookOpen, Sparkles, Loader2, Brain } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { StreamStage } from "@/lib/query/stream-session-reducer";

const STAGE_ICONS = {
  retrieving: Search,
  reading: BookOpen,
  thinking: Brain,
  generating: Sparkles,
  complete: Sparkles,
  error: Loader2,
  stopped: Loader2,
} as const;

interface StageTimelineProps {
  stage: StreamStage | null;
  detail?: string;
  sourceCount?: number;
}

/** Collapsible stage strip: Searching → Reading → Thinking → Writing (SPEC-155). */
export function StageTimeline({
  stage,
  detail,
  sourceCount,
}: StageTimelineProps) {
  const { t } = useTranslation();
  if (!stage || stage === "complete") return null;

  const Icon = STAGE_ICONS[stage] ?? Loader2;
  const label =
    stage === "retrieving"
      ? t("query.stage.retrieving", "Searching knowledge…")
      : stage === "reading"
        ? t("query.stage.reading", "Reading sources…")
        : stage === "thinking"
          ? t("query.stage.thinking", "Thinking…")
          : stage === "generating"
            ? t("query.stage.generating", "Writing answer…")
            : stage === "stopped"
              ? t("query.stage.stopped", "Stopped")
              : t("query.stage.error", "Failed");

  const meta =
    typeof sourceCount === "number"
      ? t("query.stage.sourceCount", "{{count}} sources", {
          count: sourceCount,
        })
      : detail;

  return (
    <div
      className={cn(
        "flex items-center gap-2 text-sm text-muted-foreground mb-3",
        "rounded-lg border bg-muted/30 px-3 py-2",
      )}
      data-testid="query-stage-timeline"
      role="status"
    >
      <Icon
        className={cn(
          "h-4 w-4 shrink-0 text-primary",
          stage !== "stopped" &&
            stage !== "error" &&
            "motion-safe:animate-pulse",
        )}
        aria-hidden
      />
      <span className="font-medium text-foreground/80">{label}</span>
      {meta ? (
        <span className="text-xs text-muted-foreground truncate">{meta}</span>
      ) : null}
    </div>
  );
}
