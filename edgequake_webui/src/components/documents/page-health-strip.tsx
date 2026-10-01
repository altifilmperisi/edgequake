"use client";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import type { PageHealthDto } from "@/lib/api/edgequake/pages-health";
import {
  countFailed,
  failedPageNumbers,
  pageWorst,
  progressiveReprocessStats,
  stageCompletionRatio,
  type StageKey,
  type StageStatus,
} from "@/lib/documents/page-health";
import { RefreshCw } from "lucide-react";
import { useTranslation } from "react-i18next";

function stageLabel(status: string): string {
  switch (status) {
    case "ok":
      return "ok";
    case "failed":
      return "failed";
    case "running":
      return "running";
    case "skipped":
      return "skipped";
    default:
      return "pending";
  }
}

function tileTone(status: StageStatus, forcedRunning: boolean): string {
  if (forcedRunning || status === "running") {
    return "bg-amber-500/20 text-amber-900 ring-1 ring-amber-500/40 dark:text-amber-100";
  }
  switch (status) {
    case "ok":
      return "bg-emerald-500/10 text-foreground/80";
    case "failed":
      return "bg-destructive/15 text-destructive ring-1 ring-destructive/35";
    case "skipped":
      return "bg-muted/60 text-muted-foreground";
    case "pending":
    default:
      return "bg-muted/80 text-muted-foreground";
  }
}

export interface PageHealthStripProps {
  pages: PageHealthDto[];
  /**
   * overview — detail page: status / progress only (no page picker by default).
   * picker — modal: multi-select pages for reprocess.
   */
  mode?: "overview" | "picker";
  selected?: number[];
  onSelect?: (page: number, shiftKey: boolean) => void;
  /** picker: replace selection (e.g. select all failed). */
  onSelectPages?: (pages: number[]) => void;
  onJump?: (page: number) => void;
  /** overview: open reprocess modal (optionally with failed pages preselected). */
  onOpenReprocess?: (initialPages?: number[]) => void;
  className?: string;
  calmInFlight?: boolean;
  reprocessActive?: boolean;
  pendingPages?: number[];
  /** Hide stage meters (dialog already has stage cards). */
  hideMeters?: boolean;
  /** Hide title / chips / CTA chrome (picker embeds its own step header). */
  hideChrome?: boolean;
  /** Hide page tiles (quiet overview: progress banner only). */
  hideTiles?: boolean;
}

function toTile(p: PageHealthDto) {
  return {
    page_number: p.page_number,
    parse: p.parse.status as StageStatus,
    figures: p.figures.status as StageStatus,
    entities: p.entities.status as StageStatus,
  };
}

function StageMeter({
  label,
  stage,
  pages,
}: {
  label: string;
  stage: StageKey;
  pages: ReturnType<typeof toTile>[];
}) {
  const { done, total, running, failed } = stageCompletionRatio(pages, stage);
  const pct = total === 0 ? 0 : Math.round((done / total) * 100);
  return (
    <div
      className="flex min-w-30 flex-1 flex-col gap-0.5"
      data-testid={`page-health-meter-${stage}`}
    >
      <div className="flex items-center justify-between gap-2 text-xs text-muted-foreground">
        <span>{label}</span>
        <span className="tabular-nums">
          {failed > 0
            ? `${failed} fail`
            : running > 0
              ? `${running} run`
              : `${pct}%`}
        </span>
      </div>
      <div className="h-1 overflow-hidden rounded-full bg-muted">
        <div
          className={cn(
            "h-full rounded-full transition-[width] duration-300",
            failed > 0
              ? "bg-destructive"
              : running > 0
                ? "bg-amber-500"
                : "bg-emerald-500/80",
          )}
          style={{ width: `${Math.max(pct, running > 0 ? 8 : 0)}%` }}
        />
      </div>
    </div>
  );
}

export function PageHealthStrip({
  pages,
  mode = "picker",
  selected = [],
  onSelect,
  onSelectPages,
  onJump,
  onOpenReprocess,
  className,
  calmInFlight = false,
  reprocessActive = false,
  pendingPages = [],
  hideMeters = false,
  hideChrome = false,
  hideTiles = false,
}: PageHealthStripProps) {
  const { t } = useTranslation();
  const tiles = pages.map(toTile);
  const failed = countFailed(tiles);
  const dense = pages.length > 24;
  const pendingSet = new Set(pendingPages);
  const isOverview = mode === "overview";
  const isPicker = mode === "picker";

  const progressScope =
    selected.length > 0
      ? selected
      : pendingPages.length > 0
        ? pendingPages
        : undefined;
  const progressive = progressiveReprocessStats(tiles, progressScope);
  const showProgress =
    reprocessActive ||
    progressive.running > 0 ||
    pendingPages.length > 0;

  const stageName = (key: StageKey | null) => {
    if (key === "parse")
      return t("documents.pageHealth.stageParse", { defaultValue: "Parse" });
    if (key === "figures")
      return t("documents.pageHealth.stageFigures", {
        defaultValue: "Figures",
      });
    if (key === "entities")
      return t("documents.pageHealth.stageEntities", {
        defaultValue: "Entities",
      });
    return t("documents.pageHealth.progressQueued", {
      defaultValue: "Queued",
    });
  };

  const showChrome = !hideChrome;
  const showTiles = !hideTiles;
  const showMeters = !hideMeters && !isPicker;

  return (
    <div
      className={cn("flex flex-col gap-2", className)}
      data-testid="page-health-strip"
      data-mode={mode}
    >
      {showChrome ? (
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
            <span className="font-medium text-foreground/80">
              {t("documents.pageHealth.title", {
                defaultValue: "Page health · {{count}} pages",
                count: pages.length,
              })}
            </span>
            {failed === 0 && !calmInFlight && !showProgress ? (
              <span
                className="rounded-md bg-muted px-2 py-0.5 text-xs text-muted-foreground"
                data-testid="page-health-all-clear"
              >
                {t("documents.pageHealth.allClear", {
                  defaultValue: "All clear",
                })}
              </span>
            ) : null}
            {failed > 0 ? (
              <button
                type="button"
                data-testid="page-health-filter-failed"
                className="rounded-md bg-destructive/10 px-2 py-0.5 text-xs text-destructive transition hover:bg-destructive/15"
                onClick={() => {
                  const pagesFailed = failedPageNumbers(tiles);
                  if (isOverview) {
                    onOpenReprocess?.(pagesFailed);
                  } else {
                    onSelectPages?.(pagesFailed);
                  }
                }}
              >
                {t("documents.pageHealth.needsAttention", {
                  defaultValue: "Needs attention ({{count}})",
                  count: failed,
                })}
              </button>
            ) : null}
          </div>
          <div className="flex shrink-0 items-center gap-1.5">
            {isOverview && onOpenReprocess ? (
              <Button
                type="button"
                size="sm"
                variant="default"
                className="h-8 gap-1.5"
                data-testid="page-health-open-reprocess"
                disabled={reprocessActive}
                onClick={() => onOpenReprocess()}
              >
                <RefreshCw
                  className={cn(
                    "h-3.5 w-3.5",
                    reprocessActive && "animate-spin",
                  )}
                />
                {reprocessActive
                  ? t("documents.pageHealth.reprocessingBusy", {
                      defaultValue: "Reprocessing…",
                    })
                  : t("documents.pageHealth.menuAction", {
                      defaultValue: "Reprocess specific pages",
                    })}
              </Button>
            ) : null}
          </div>
        </div>
      ) : null}

      {showMeters ? (
        <div
          className="flex flex-wrap gap-3"
          data-testid="page-health-stage-meters"
        >
          <StageMeter
            label={t("documents.pageHealth.meterParse", {
              defaultValue: "Parse",
            })}
            stage="parse"
            pages={tiles}
          />
          <StageMeter
            label={t("documents.pageHealth.meterFigures", {
              defaultValue: "Figures",
            })}
            stage="figures"
            pages={tiles}
          />
          <StageMeter
            label={t("documents.pageHealth.meterEntities", {
              defaultValue: "Entities",
            })}
            stage="entities"
            pages={tiles}
          />
        </div>
      ) : null}

      {showProgress ? (
        <div
          className="rounded-md border border-amber-500/25 bg-amber-500/5 px-2.5 py-2"
          data-testid="page-health-progressive"
          aria-live="polite"
        >
          <div className="mb-1 flex items-center justify-between gap-2 text-xs">
            <span className="font-medium text-foreground/90">
              {t("documents.pageHealth.progressTitle", {
                defaultValue: "Reprocessing · {{done}}/{{total}} pages",
                done: progressive.done,
                total: Math.max(progressive.total, pendingPages.length),
              })}
            </span>
            <span className="text-muted-foreground">
              {stageName(progressive.activeStage)}
            </span>
          </div>
          <div className="h-1.5 overflow-hidden rounded-full bg-muted">
            <div
              className="h-full rounded-full bg-amber-500 transition-[width] duration-500 ease-out"
              style={{
                width: `${Math.round(
                  (progressive.total > 0
                    ? progressive.ratio
                    : pendingPages.length > 0
                      ? 0.05
                      : 0) * 100,
                )}%`,
              }}
            />
          </div>
        </div>
      ) : null}

      {showTiles ? (
        <div
          className={cn(
            "flex gap-1",
            dense ? "overflow-x-auto pb-0.5" : "flex-wrap",
          )}
          role={isPicker ? "listbox" : "list"}
          aria-label={t("documents.pageHealth.heatmapLabel", {
            defaultValue: "Page status map",
          })}
          aria-multiselectable={isPicker || undefined}
        >
          {pages.map((p) => {
            const isSelected = isPicker && selected.includes(p.page_number);
            const forcedRunning = pendingSet.has(p.page_number);
            const worst = pageWorst(toTile(p));
            const title = t("documents.pageHealth.tileTitle", {
              defaultValue:
                "Page {{page}} — Parse {{parse}} · Figures {{figures}} · Entities {{entities}}",
              page: p.page_number,
              parse: stageLabel(p.parse.status),
              figures: stageLabel(p.figures.status),
              entities: stageLabel(p.entities.status),
            });
            return (
              <button
                key={p.page_number}
                type="button"
                role={isPicker ? "option" : undefined}
                data-testid={`page-health-tile-${p.page_number}`}
                data-page={p.page_number}
                data-status={forcedRunning ? "running" : worst}
                data-parse={p.parse.status}
                data-figures={p.figures.status}
                data-entities={p.entities.status}
                title={
                  isOverview
                    ? t("documents.pageHealth.tileJumpTitle", {
                        defaultValue: "Go to page {{page}}",
                        page: p.page_number,
                      })
                    : title
                }
                aria-label={title}
                aria-selected={isPicker ? isSelected : undefined}
                className={cn(
                  "flex shrink-0 items-center justify-center rounded-md text-xs font-medium tabular-nums transition",
                  dense ? "h-7 w-7" : "h-8 w-8",
                  isSelected
                    ? "bg-primary text-primary-foreground shadow-sm ring-2 ring-primary/40 ring-offset-1 ring-offset-background"
                    : tileTone(worst, forcedRunning),
                  !isSelected &&
                    (forcedRunning || worst === "running") &&
                    "animate-pulse",
                )}
                onClick={(e) => {
                  if (isPicker) {
                    onSelect?.(p.page_number, e.shiftKey);
                    return;
                  }
                  onJump?.(p.page_number);
                }}
              >
                {p.page_number}
              </button>
            );
          })}
        </div>
      ) : null}

      {isPicker && showTiles && showChrome ? (
        <p className="text-xs text-muted-foreground">
          {t("documents.pageHealth.pickerHint", {
            defaultValue:
              "Click to select · Shift+click for a range · {{count}} selected",
            count: selected.length,
          })}
        </p>
      ) : null}
    </div>
  );
}
