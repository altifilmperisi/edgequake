"use client";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { formatPageRange, parsePageRange } from "@/lib/documents/page-range";
import { useTranslation } from "react-i18next";
import { useEffect, useState } from "react";

export interface PageSelectionBarProps {
  selected: number[];
  pageCount: number;
  onChange: (pages: number[]) => void;
  onSelectFailed?: () => void;
  /** Open reprocess dialog for current selection. */
  onReprocessSelected?: () => void;
  className?: string;
  /** Hide range input (slim detail-page bar). */
  compact?: boolean;
}

export function PageSelectionBar({
  selected,
  pageCount,
  onChange,
  onSelectFailed,
  onReprocessSelected,
  className,
  compact = false,
}: PageSelectionBarProps) {
  const { t } = useTranslation();
  const [range, setRange] = useState(formatPageRange(selected));
  const [error, setError] = useState<string | null>(null);

  // Keep range input in sync when strip selection changes externally.
  useEffect(() => {
    setRange(formatPageRange(selected));
    setError(null);
  }, [selected]);

  return (
    <div
      className={className}
      data-testid="page-selection-bar"
    >
      <div className="flex flex-wrap items-center gap-2">
        {!compact ? (
          <Input
            data-testid="page-range-input"
            value={range}
            placeholder={t("documents.pageHealth.rangePlaceholder", {
              defaultValue: "e.g. 1-3,7",
            })}
            className="h-8 max-w-[180px] text-sm"
            onChange={(e) => {
              setRange(e.target.value);
              setError(null);
            }}
            onBlur={() => {
              try {
                const pages = parsePageRange(range, pageCount);
                onChange(pages);
                setRange(formatPageRange(pages));
              } catch (err) {
                setError(err instanceof Error ? err.message : String(err));
              }
            }}
          />
        ) : null}
        {onSelectFailed ? (
          <Button
            type="button"
            size="sm"
            variant="outline"
            data-testid="select-failed-pages"
            onClick={onSelectFailed}
          >
            {t("documents.pageHealth.selectFailed", {
              defaultValue: "Select failed",
            })}
          </Button>
        ) : null}
        <Button
          type="button"
          size="sm"
          variant="ghost"
          data-testid="clear-page-selection"
          onClick={() => {
            onChange([]);
            setRange("");
          }}
        >
          {t("documents.pageHealth.clear", { defaultValue: "Clear" })}
        </Button>
        {onReprocessSelected && selected.length > 0 ? (
          <Button
            type="button"
            size="sm"
            variant="default"
            data-testid="reprocess-selected-pages"
            onClick={onReprocessSelected}
          >
            {t("documents.pageHealth.reprocessSelected", {
              defaultValue: "Reprocess selected",
            })}
          </Button>
        ) : null}
        <span className="text-xs text-muted-foreground">
          {t("documents.pageHealth.selectedCount", {
            defaultValue: "{{count}} selected",
            count: selected.length,
          })}
        </span>
      </div>
      {error ? (
        <p className="mt-1 text-xs text-destructive" data-testid="page-range-error">
          {error}
        </p>
      ) : null}
    </div>
  );
}
