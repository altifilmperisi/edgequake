"use client";

/**
 * SPEC-151 — Scrollable page multi-select grid for the reprocess dialog.
 * Scales to 100+ pages: capped height, status filter, band jump chips.
 */

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import type { PageHealthDto } from "@/lib/api/edgequake/pages-health";
import {
  pageWorst,
  type StageStatus,
} from "@/lib/documents/page-health";
import { useEffect, useMemo, useRef } from "react";
import { useTranslation } from "react-i18next";

export type PagePickerFilter = "all" | "failed" | "selected";

const BAND_SIZE = 25;

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

function tileTone(status: StageStatus): string {
  switch (status) {
    case "ok":
      return "bg-emerald-500/10 text-foreground/80";
    case "failed":
      return "bg-destructive/15 text-destructive ring-1 ring-destructive/35";
    case "skipped":
      return "bg-muted/60 text-muted-foreground";
    case "running":
      return "bg-amber-500/20 text-amber-900 ring-1 ring-amber-500/40 dark:text-amber-100";
    case "pending":
    default:
      return "bg-muted/80 text-muted-foreground";
  }
}

function isFailedPage(p: PageHealthDto): boolean {
  return pageWorst({
    page_number: p.page_number,
    parse: p.parse.status as StageStatus,
    figures: p.figures.status as StageStatus,
    entities: p.entities.status as StageStatus,
  }) === "failed";
}

export interface PagePickerGridProps {
  pages: PageHealthDto[];
  selected: number[];
  filter: PagePickerFilter;
  onFilterChange: (f: PagePickerFilter) => void;
  onSelect: (page: number, shiftKey: boolean) => void;
  className?: string;
}

export function PagePickerGrid({
  pages,
  selected,
  filter,
  onFilterChange,
  onSelect,
  className,
}: PagePickerGridProps) {
  const { t } = useTranslation();
  const scrollRef = useRef<HTMLDivElement>(null);
  const selectedSet = useMemo(() => new Set(selected), [selected]);
  const large = pages.length > 40;
  const failedCount = useMemo(
    () => pages.filter(isFailedPage).length,
    [pages],
  );

  const bands = useMemo(() => {
    if (pages.length === 0) return [];
    const out: { start: number; end: number }[] = [];
    const max = pages[pages.length - 1]?.page_number ?? pages.length;
    for (let start = 1; start <= max; start += BAND_SIZE) {
      out.push({ start, end: Math.min(start + BAND_SIZE - 1, max) });
    }
    return out;
  }, [pages]);

  const visible = useMemo(() => {
    if (filter === "failed") return pages.filter(isFailedPage);
    if (filter === "selected") {
      return pages.filter((p) => selectedSet.has(p.page_number));
    }
    return pages;
  }, [pages, filter, selectedSet]);

  // When selection changes and filter is "selected", keep list coherent.
  useEffect(() => {
    if (filter === "selected" && selected.length === 0) {
      onFilterChange("all");
    }
  }, [filter, selected.length, onFilterChange]);

  const jumpToBand = (start: number) => {
    onFilterChange("all");
    // Wait a tick so "all" filter remounts tiles, then scroll.
    requestAnimationFrame(() => {
      const el = scrollRef.current?.querySelector(
        `[data-page="${start}"]`,
      ) as HTMLElement | null;
      el?.scrollIntoView({ block: "nearest", behavior: "smooth" });
    });
  };

  const colsClass =
    pages.length > 60
      ? "grid-cols-10 sm:grid-cols-12"
      : pages.length > 30
        ? "grid-cols-8 sm:grid-cols-10"
        : "grid-cols-6 sm:grid-cols-8";

  return (
    <div
      className={cn("space-y-2", className)}
      data-testid="page-picker-grid"
      data-page-count={pages.length}
    >
      <div className="flex flex-wrap items-center gap-1.5">
        {(
          [
            ["all", t("documents.pageHealth.filterAll", { defaultValue: "All" })],
            [
              "failed",
              t("documents.pageHealth.filterFailed", {
                defaultValue: "Failed",
              }) + (failedCount > 0 ? ` (${failedCount})` : ""),
            ],
            [
              "selected",
              t("documents.pageHealth.filterSelected", {
                defaultValue: "Selected",
              }) + (selected.length > 0 ? ` (${selected.length})` : ""),
            ],
          ] as const
        ).map(([id, label]) => (
          <button
            key={id}
            type="button"
            data-testid={`page-picker-filter-${id}`}
            disabled={
              (id === "failed" && failedCount === 0) ||
              (id === "selected" && selected.length === 0)
            }
            className={cn(
              "rounded-md px-2 py-0.5 text-[11px] transition",
              filter === id
                ? "bg-primary text-primary-foreground"
                : "bg-muted/80 text-muted-foreground hover:bg-muted",
              "disabled:pointer-events-none disabled:opacity-40",
            )}
            onClick={() => onFilterChange(id)}
          >
            {label}
          </button>
        ))}
        <span className="ml-auto text-[11px] tabular-nums text-muted-foreground">
          {t("documents.pageHealth.showingOf", {
            defaultValue: "{{shown}} of {{total}}",
            shown: visible.length,
            total: pages.length,
          })}
        </span>
      </div>

      {large && bands.length > 1 ? (
        <div
          className="flex flex-wrap gap-1"
          data-testid="page-picker-bands"
          aria-label={t("documents.pageHealth.bandJumpLabel", {
            defaultValue: "Jump to page band",
          })}
        >
          {bands.map((b) => (
            <Button
              key={b.start}
              type="button"
              size="sm"
              variant="ghost"
              className="h-6 px-1.5 text-[10px] text-muted-foreground"
              data-testid={`page-picker-band-${b.start}`}
              onClick={() => jumpToBand(b.start)}
            >
              {b.start}–{b.end}
            </Button>
          ))}
        </div>
      ) : null}

      <div
        ref={scrollRef}
        className={cn(
          "rounded-lg border bg-background/60 p-2",
          large ? "max-h-56 overflow-y-auto overscroll-contain" : "max-h-48 overflow-y-auto",
        )}
      >
        {visible.length === 0 ? (
          <p className="px-1 py-6 text-center text-xs text-muted-foreground">
            {t("documents.pageHealth.pickerEmpty", {
              defaultValue: "No pages match this filter.",
            })}
          </p>
        ) : (
          <div
            className={cn("grid gap-1", colsClass)}
            role="listbox"
            aria-multiselectable
            aria-label={t("documents.pageHealth.heatmapLabel", {
              defaultValue: "Page status map",
            })}
          >
            {visible.map((p) => {
              const isSelected = selectedSet.has(p.page_number);
              const worst = pageWorst({
                page_number: p.page_number,
                parse: p.parse.status as StageStatus,
                figures: p.figures.status as StageStatus,
                entities: p.entities.status as StageStatus,
              });
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
                  role="option"
                  data-testid={`page-health-tile-${p.page_number}`}
                  data-page={p.page_number}
                  data-status={worst}
                  title={title}
                  aria-label={title}
                  aria-selected={isSelected}
                  className={cn(
                    "flex h-7 items-center justify-center rounded-md text-[10px] font-medium tabular-nums transition",
                    isSelected
                      ? "bg-primary text-primary-foreground shadow-sm ring-2 ring-primary/30"
                      : tileTone(worst),
                    worst === "running" && !isSelected && "animate-pulse",
                  )}
                  onClick={(e) => onSelect(p.page_number, e.shiftKey)}
                >
                  {p.page_number}
                </button>
              );
            })}
          </div>
        )}
      </div>

      <p className="text-[11px] text-muted-foreground">
        {t("documents.pageHealth.pickerHintShort", {
          defaultValue: "Click to toggle · Shift+click for a range",
        })}
        {large
          ? ` · ${t("documents.pageHealth.rangeHintLarge", {
              defaultValue: "Or type a range above (e.g. 40-55,90).",
            })}`
          : ""}
      </p>
    </div>
  );
}
