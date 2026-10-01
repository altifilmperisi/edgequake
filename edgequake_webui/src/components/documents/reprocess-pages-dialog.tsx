"use client";

/**
 * SPEC-151 — Reprocess selected pages dialog (modal-first UX).
 * Clear three-step flow: pages → stage → preview/start.
 * Scales to 100+ pages via scrollable PagePickerGrid + range-first selection.
 */

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  PagePickerGrid,
  type PagePickerFilter,
} from "@/components/documents/page-picker-grid";
import { usePageHealth, useReprocessPages } from "@/hooks/use-page-health";
import { failedPageNumbers } from "@/lib/documents/page-health";
import {
  formatPageRange,
  parsePageRange,
  togglePage,
} from "@/lib/documents/page-range";
import {
  effectiveStages,
  isStageLocked,
  type ReprocessPageStage,
} from "@/lib/documents/reprocess-stages";
import { cn } from "@/lib/utils";
import { Check, Loader2 } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import type { PartialReprocessPlan } from "@/lib/api/edgequake/pages-health";

export interface ReprocessPagesDialogProps {
  open: boolean;
  documentId: string;
  /** Optional display name (list mode / detail header). */
  documentName?: string;
  pageCount?: number;
  initialPages?: number[];
  onClose: () => void;
  /** Called with track id and the pages that were queued. */
  onQueued?: (trackId: string, pages: number[]) => void;
}

const STAGE_CARDS: {
  id: ReprocessPageStage;
  titleKey: string;
  titleDefault: string;
  descKey: string;
  descDefault: string;
  descShortKey: string;
  descShortDefault: string;
}[] = [
  {
    id: "parse",
    titleKey: "documents.pageHealth.stageParse",
    titleDefault: "Parse (OCR)",
    descKey: "documents.pageHealth.stageParseDesc",
    descDefault:
      "Re-run vision OCR on selected pages. Also re-runs figures and entities.",
    descShortKey: "documents.pageHealth.stageParseDescShort",
    descShortDefault: "OCR + figures + entities",
  },
  {
    id: "figures",
    titleKey: "documents.pageHealth.stageFigures",
    titleDefault: "Figures & charts",
    descKey: "documents.pageHealth.stageFiguresDesc",
    descDefault: "Rebuild page assets. Also re-runs entities.",
    descShortKey: "documents.pageHealth.stageFiguresDescShort",
    descShortDefault: "Assets + entities",
  },
  {
    id: "entities",
    titleKey: "documents.pageHealth.stageEntities",
    titleDefault: "Entities",
    descKey: "documents.pageHealth.stageEntitiesDesc",
    descDefault:
      "Re-extract KG for chunks overlapping selected pages; reuse the rest.",
    descShortKey: "documents.pageHealth.stageEntitiesDescShort",
    descShortDefault: "KG extract on dirty chunks",
  },
];

function StepBadge({ n }: { n: number }) {
  return (
    <span className="flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-primary/10 text-xs font-semibold text-primary">
      {n}
    </span>
  );
}

export function ReprocessPagesDialog({
  open,
  documentId,
  documentName,
  pageCount,
  initialPages,
  onClose,
  onQueued,
}: ReprocessPagesDialogProps) {
  const { t } = useTranslation();
  const health = usePageHealth(documentId, open);
  const mutation = useReprocessPages(documentId);
  const [selected, setSelected] = useState<number[]>(() => initialPages ?? []);
  const [anchor, setAnchor] = useState<number | null>(null);
  const [stage, setStage] = useState<ReprocessPageStage>("parse");
  const [preview, setPreview] = useState<PartialReprocessPlan | null>(null);
  const [previewing, setPreviewing] = useState(false);
  const [rangeText, setRangeText] = useState(() =>
    formatPageRange(initialPages ?? []),
  );
  const [rangeError, setRangeError] = useState<string | null>(null);
  const [pickerFilter, setPickerFilter] = useState<PagePickerFilter>("all");

  const selectionKey = open
    ? `${documentId}:${(initialPages ?? []).join(",")}`
    : "";
  const [lastKey, setLastKey] = useState(selectionKey);
  if (open && selectionKey !== lastKey) {
    setLastKey(selectionKey);
    const seed = initialPages ?? [];
    setSelected(seed);
    setRangeText(formatPageRange(seed));
    setRangeError(null);
    setPreview(null);
    setStage("parse");
    setAnchor(null);
    setPickerFilter("all");
  }

  useEffect(() => {
    if (!open) return;
    setRangeText(formatPageRange(selected));
  }, [selected, open]);

  const pages = health.data?.pages ?? [];
  const count = pageCount ?? health.data?.page_count ?? pages.length;
  const largeDoc = count > 40;
  const closed = useMemo(() => effectiveStages([stage]), [stage]);
  const failedList = useMemo(
    () =>
      failedPageNumbers(
        pages.map((p) => ({
          page_number: p.page_number,
          parse: p.parse.status as "ok",
          figures: p.figures.status as "ok",
          entities: p.entities.status as "ok",
        })),
      ),
    [pages],
  );
  const failedCount = failedList.length;

  const summaryLine = useMemo(() => {
    if (selected.length === 0) return null;
    const stagesLabel = closed
      .map((s) => {
        if (s === "parse")
          return t("documents.pageHealth.meterParse", {
            defaultValue: "Parse",
          });
        if (s === "figures")
          return t("documents.pageHealth.meterFigures", {
            defaultValue: "Figures",
          });
        return t("documents.pageHealth.meterEntities", {
          defaultValue: "Entities",
        });
      })
      .join(" → ");
    return t("documents.pageHealth.readySummary", {
      defaultValue: "{{count}} page(s) · {{stages}}",
      count: selected.length,
      stages: stagesLabel,
    });
  }, [selected.length, closed, t]);

  const onSelect = useCallback(
    (page: number, shiftKey: boolean) => {
      const next = togglePage(selected, page, shiftKey ? anchor : null);
      setSelected(next.pages);
      setAnchor(next.anchor);
      setPreview(null);
      setRangeError(null);
    },
    [selected, anchor],
  );

  const applyRange = () => {
    try {
      const parsed = parsePageRange(rangeText, count || 1);
      setSelected(parsed);
      setRangeText(formatPageRange(parsed));
      setRangeError(null);
      setPreview(null);
      if (parsed.length > 0) setPickerFilter("selected");
    } catch (err) {
      setRangeError(err instanceof Error ? err.message : String(err));
    }
  };

  const runDryRun = async () => {
    if (selected.length === 0) {
      toast.error(
        t("documents.pageHealth.needPages", {
          defaultValue: "Select at least one page",
        }),
      );
      return;
    }
    setPreviewing(true);
    try {
      const res = await mutation.mutateAsync({
        pages: formatPageRange(selected),
        stages: [stage],
        dry_run: true,
      });
      setPreview(res.plan);
    } catch (e) {
      toast.error(
        t("documents.pageHealth.previewFailed", {
          defaultValue: "Could not preview reprocess",
        }),
        { description: e instanceof Error ? e.message : String(e) },
      );
    } finally {
      setPreviewing(false);
    }
  };

  const runEnqueue = async () => {
    if (selected.length === 0) return;
    try {
      const res = await mutation.mutateAsync({
        pages: formatPageRange(selected),
        stages: [stage],
        dry_run: false,
      });
      if (res.track_id) {
        toast.success(
          t("documents.pageHealth.queued", {
            defaultValue: "Page reprocess queued",
          }),
        );
        onQueued?.(res.track_id, selected);
        onClose();
      }
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      toast.error(
        t("documents.pageHealth.enqueueFailed", {
          defaultValue: "Could not start reprocess",
        }),
        { description: msg },
      );
    }
  };

  return (
    <Dialog open={open} onOpenChange={(v) => !v && onClose()}>
      <DialogContent
        className={cn(
          "flex max-h-[min(92vh,820px)] w-full flex-col gap-0 overflow-hidden p-0",
          largeDoc ? "max-w-3xl sm:max-w-3xl" : "max-w-2xl sm:max-w-2xl",
        )}
        data-testid="reprocess-pages-dialog"
        data-large={largeDoc ? "true" : "false"}
      >
        <DialogHeader className="shrink-0 space-y-1.5 border-b px-6 py-4 text-left">
          <DialogTitle>
            {t("documents.pageHealth.dialogTitle", {
              defaultValue: "Reprocess specific pages",
            })}
          </DialogTitle>
          <DialogDescription>
            {documentName ? (
              <span className="block truncate font-medium text-foreground/80">
                {documentName}
                {count > 0 ? (
                  <span className="font-normal text-muted-foreground">
                    {" "}
                    ·{" "}
                    {t("documents.pageHealth.pageCountLabel", {
                      defaultValue: "{{count}} pages",
                      count,
                    })}
                  </span>
                ) : null}
              </span>
            ) : null}
            {t("documents.pageHealth.dialogDesc", {
              defaultValue:
                "Select pages, choose a starting stage, then start. Other pages stay untouched.",
            })}
          </DialogDescription>
        </DialogHeader>

        <div className="min-h-0 flex-1 space-y-5 overflow-y-auto px-6 py-5">
          {health.isLoading ? (
            <div className="flex items-center gap-2 py-8 text-sm text-muted-foreground">
              <Loader2 className="h-4 w-4 animate-spin" />
              {t("documents.pageHealth.loadingHealth", {
                defaultValue: "Loading page health…",
              })}
            </div>
          ) : (
            <>
              {/* Step 1 — Pages */}
              <section
                className="space-y-3"
                aria-labelledby="reprocess-pages-select"
              >
                <div className="flex flex-wrap items-center gap-2">
                  <StepBadge n={1} />
                  <Label
                    id="reprocess-pages-select"
                    className="text-sm font-medium"
                  >
                    {t("documents.pageHealth.pagesLabel", {
                      defaultValue: "Select pages",
                    })}
                  </Label>
                  <span
                    className={cn(
                      "ml-auto rounded-md px-2 py-0.5 text-xs tabular-nums",
                      selected.length > 0
                        ? "bg-primary/10 font-medium text-primary"
                        : "bg-muted text-muted-foreground",
                    )}
                    data-testid="reprocess-selected-count"
                  >
                    {t("documents.pageHealth.selectedOfTotal", {
                      defaultValue: "{{count}} / {{total}} selected",
                      count: selected.length,
                      total: count,
                    })}
                  </span>
                </div>

                {/* Range-first toolbar — primary path for large docs */}
                <div
                  className={cn(
                    "space-y-2 rounded-xl border p-3",
                    largeDoc
                      ? "border-primary/20 bg-primary/3"
                      : "bg-muted/20",
                  )}
                >
                  <div className="space-y-1">
                    <Label
                      htmlFor="reprocess-pages-range-input"
                      className="text-xs text-muted-foreground"
                    >
                      {t("documents.pageHealth.rangeLabel", {
                        defaultValue: "Page range",
                      })}
                    </Label>
                    <div className="flex gap-2">
                      <Input
                        id="reprocess-pages-range-input"
                        data-testid="reprocess-pages-range-input"
                        value={rangeText}
                        placeholder={t(
                          "documents.pageHealth.rangePlaceholder",
                          {
                            defaultValue: "e.g. 1-3,7",
                          },
                        )}
                        className="h-9 min-w-0 flex-1 text-sm"
                        onChange={(e) => {
                          setRangeText(e.target.value);
                          setRangeError(null);
                        }}
                        onKeyDown={(e) => {
                          if (e.key === "Enter") {
                            e.preventDefault();
                            applyRange();
                          }
                        }}
                      />
                      <Button
                        type="button"
                        size="sm"
                        className="h-9 shrink-0"
                        data-testid="reprocess-pages-apply-range"
                        onClick={applyRange}
                      >
                        {t("documents.pageHealth.applyRange", {
                          defaultValue: "Apply",
                        })}
                      </Button>
                    </div>
                  </div>
                  <div className="flex flex-wrap items-center gap-1.5">
                    <Button
                      type="button"
                      size="sm"
                      variant="outline"
                      className="h-8"
                      data-testid="select-failed-pages"
                      disabled={failedCount === 0}
                      onClick={() => {
                        setSelected(failedList);
                        setPreview(null);
                        setRangeError(null);
                        setPickerFilter(
                          failedList.length > 0 ? "selected" : "all",
                        );
                      }}
                    >
                      {t("documents.pageHealth.selectFailed", {
                        defaultValue: "Select failed",
                      })}
                      {failedCount > 0 ? ` (${failedCount})` : ""}
                    </Button>
                    <Button
                      type="button"
                      size="sm"
                      variant="outline"
                      className="h-8"
                      data-testid="select-all-pages"
                      disabled={pages.length === 0}
                      onClick={() => {
                        const all = pages.map((p) => p.page_number);
                        setSelected(all);
                        setPreview(null);
                        setRangeError(null);
                        setPickerFilter("all");
                      }}
                    >
                      {t("documents.pageHealth.selectAll", {
                        defaultValue: "Select all",
                      })}
                    </Button>
                    <Button
                      type="button"
                      size="sm"
                      variant="ghost"
                      className="h-8"
                      data-testid="clear-page-selection"
                      disabled={selected.length === 0}
                      onClick={() => {
                        setSelected([]);
                        setPreview(null);
                        setRangeError(null);
                        setPickerFilter("all");
                      }}
                    >
                      {t("documents.pageHealth.clear", {
                        defaultValue: "Clear",
                      })}
                    </Button>
                  </div>
                </div>
                {rangeError ? (
                  <p
                    className="text-xs text-destructive"
                    data-testid="page-range-error"
                  >
                    {rangeError}
                  </p>
                ) : null}

                <PagePickerGrid
                  pages={pages}
                  selected={selected}
                  filter={pickerFilter}
                  onFilterChange={setPickerFilter}
                  onSelect={onSelect}
                />
              </section>

              {/* Step 2 — Stage */}
              <section
                className="space-y-3"
                aria-labelledby="reprocess-stage-label"
              >
                <div className="flex items-center gap-2">
                  <StepBadge n={2} />
                  <Label
                    id="reprocess-stage-label"
                    className="text-sm font-medium"
                  >
                    {t("documents.pageHealth.stageLabel", {
                      defaultValue: "Choose starting stage",
                    })}
                  </Label>
                </div>
                <div
                  className="grid gap-2 sm:grid-cols-3"
                  data-testid="reprocess-stage-cards"
                  role="radiogroup"
                  aria-labelledby="reprocess-stage-label"
                >
                  {STAGE_CARDS.map((card) => {
                    const locked = isStageLocked(stage, card.id);
                    const active = stage === card.id;
                    const included = closed.includes(card.id);
                    return (
                      <button
                        key={card.id}
                        type="button"
                        role="radio"
                        aria-checked={active}
                        data-testid={`stage-card-${card.id}`}
                        data-locked={locked ? "true" : "false"}
                        className={cn(
                          "relative rounded-xl border text-left transition",
                          largeDoc ? "p-2.5" : "p-3",
                          active
                            ? "border-primary bg-primary/5 shadow-sm"
                            : included
                              ? "border-primary/25 bg-muted/30"
                              : "border-border hover:border-primary/40",
                        )}
                        onClick={() => {
                          setStage(card.id);
                          setPreview(null);
                        }}
                      >
                        <div className="flex items-start justify-between gap-2">
                          <span className="text-sm font-medium leading-tight">
                            {t(card.titleKey, {
                              defaultValue: card.titleDefault,
                            })}
                          </span>
                          {active ? (
                            <Check
                              className="h-3.5 w-3.5 shrink-0 text-primary"
                              aria-hidden
                            />
                          ) : locked ? (
                            <span className="shrink-0 text-xs font-medium uppercase tracking-wide text-muted-foreground">
                              {t("documents.pageHealth.lockedDownstream", {
                                defaultValue: "Included",
                              })}
                            </span>
                          ) : null}
                        </div>
                        <p className="mt-1.5 text-xs leading-snug text-muted-foreground">
                          {t(largeDoc ? card.descShortKey : card.descKey, {
                            defaultValue: largeDoc
                              ? card.descShortDefault
                              : card.descDefault,
                          })}
                        </p>
                      </button>
                    );
                  })}
                </div>
              </section>

              {/* Step 3 — Confirm / impact */}
              <section
                className="space-y-2"
                aria-labelledby="reprocess-confirm-label"
              >
                <div className="flex items-center gap-2">
                  <StepBadge n={3} />
                  <Label
                    id="reprocess-confirm-label"
                    className="text-sm font-medium"
                  >
                    {t("documents.pageHealth.confirmStepLabel", {
                      defaultValue: "Confirm",
                    })}
                  </Label>
                </div>
                {summaryLine ? (
                  <p
                    className="text-sm text-foreground/85"
                    data-testid="reprocess-ready-summary"
                  >
                    {summaryLine}
                    {selected.length > 0 ? (
                      <span className="mt-0.5 block text-xs text-muted-foreground">
                        {t("documents.pageHealth.selectedRangePreview", {
                          defaultValue: "Pages: {{range}}",
                          range: formatPageRange(selected),
                        })}
                      </span>
                    ) : null}
                  </p>
                ) : (
                  <p className="text-xs text-muted-foreground">
                    {t("documents.pageHealth.confirmHint", {
                      defaultValue:
                        "Select at least one page to enable Start reprocess.",
                    })}
                  </p>
                )}
                {preview ? (
                  <div
                    className="rounded-xl border bg-muted/40 p-3 text-xs"
                    data-testid="reprocess-impact-preview"
                  >
                    <p className="font-medium text-foreground/90">
                      {t("documents.pageHealth.impactTitle", {
                        defaultValue: "Impact preview",
                      })}
                    </p>
                    <p className="mt-1 text-muted-foreground">
                      {t("documents.pageHealth.impact", {
                        defaultValue:
                          "Stages: {{stages}} · Dirty chunks: {{dirty}} · Reusable: {{reuse}} · Vision calls ≈ {{vision}}",
                        stages: preview.effective_stages.join(", "),
                        dirty: preview.dirty_chunk_count,
                        reuse: preview.reusable_chunk_count,
                        vision: preview.estimated_vision_calls,
                      })}
                    </p>
                    {preview.warnings.map((w) => (
                      <p
                        key={w}
                        className="mt-1 text-amber-700 dark:text-amber-400"
                      >
                        {w}
                      </p>
                    ))}
                  </div>
                ) : null}
              </section>
            </>
          )}
        </div>

        <DialogFooter className="shrink-0 flex-col gap-2 border-t bg-background px-6 py-3 sm:flex-col">
          <p
            className="w-full text-xs text-muted-foreground"
            data-testid="never-downgrade-note"
          >
            {t("documents.pageHealth.neverDowngrade", {
              defaultValue:
                "If a page fails again, its current markdown is kept (never downgrade).",
            })}
          </p>
          <div className="flex w-full flex-wrap items-center justify-between gap-2">
            <Button type="button" variant="ghost" onClick={onClose}>
              {t("common.cancel", { defaultValue: "Cancel" })}
            </Button>
            <div className="flex flex-wrap items-center justify-end gap-2">
              <Button
                type="button"
                variant="outline"
                data-testid="reprocess-pages-preview"
                disabled={
                  previewing || mutation.isPending || selected.length === 0
                }
                onClick={() => void runDryRun()}
              >
                {previewing ? (
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                ) : null}
                {t("documents.pageHealth.preview", { defaultValue: "Preview" })}
              </Button>
              <Button
                type="button"
                data-testid="reprocess-pages-confirm"
                disabled={mutation.isPending || selected.length === 0}
                onClick={() => void runEnqueue()}
              >
                {mutation.isPending ? (
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                ) : null}
                {t("documents.pageHealth.start", {
                  defaultValue: "Start reprocess",
                })}
              </Button>
            </div>
          </div>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
