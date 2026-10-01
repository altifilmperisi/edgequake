"use client";

import { DocumentPickerPopover } from "@/components/query/document-picker-popover";
import { useScopeDocumentLabel } from "@/hooks/use-scope-document-label";
import { cn } from "@/lib/utils";
import { ChevronDown, FileText, Filter, X } from "lucide-react";
import { useTranslation } from "react-i18next";

export interface GraphDocumentFilterBarProps {
  documentId: string | null;
  onDocumentChange: (documentId: string | null) => void;
  disabled?: boolean;
}

function DocumentScopePill({
  documentId,
  onClear,
}: {
  documentId: string;
  onClear: () => void;
}) {
  const { t } = useTranslation();
  const { label, isLoading } = useScopeDocumentLabel(documentId);
  // Never show a raw GUID — loading placeholder until the name resolves.
  const display = label ?? (isLoading ? "…" : t("graph.documentFilter.unknown", "Unknown document"));

  return (
    <span
      data-testid="graph-document-filter-pill"
      className="inline-flex items-center gap-1 max-w-[220px] rounded-md bg-secondary/80 px-2 py-0.5 text-xs font-medium ring-1 ring-border/60"
    >
      <FileText className="h-3 w-3 shrink-0 text-muted-foreground" aria-hidden />
      <span className="truncate" title={label ?? undefined}>
        {display}
      </span>
      <button
        type="button"
        onClick={onClear}
        className="ml-0.5 rounded-sm p-0.5 hover:bg-muted"
        aria-label={t("graph.documentFilter.clear", "Clear document filter")}
      >
        <X className="h-3 w-3" />
      </button>
    </span>
  );
}

/**
 * Always-visible document scope control for the graph explorer.
 * Loads a document-scoped subgraph via `/lineage/documents/:id`.
 */
export function GraphDocumentFilterBar({
  documentId,
  onDocumentChange,
  disabled = false,
}: GraphDocumentFilterBarProps) {
  const { t } = useTranslation();
  const hasFilter = !!documentId;

  const handleSelectionChange = (ids: string[]) => {
    onDocumentChange(ids.length > 0 ? ids[ids.length - 1]! : null);
  };

  return (
    <div
      role="region"
      aria-label={t("graph.documentFilter.region", "Graph document filter")}
      className={cn(
        "flex items-center gap-2 px-2 sm:px-4 py-1.5 border-b shrink-0 min-h-9",
        "overflow-x-auto scrollbar-none whitespace-nowrap",
        "transition-colors duration-150",
        hasFilter && "bg-primary/5 ring-1 ring-inset ring-primary/10",
        disabled && "opacity-60 pointer-events-none",
      )}
    >
      <Filter
        className="h-3.5 w-3.5 text-muted-foreground shrink-0"
        aria-hidden
      />
      <span className="text-xs font-medium uppercase tracking-wide text-muted-foreground shrink-0">
        {t("graph.documentFilter.label", "Document")}
      </span>

      {hasFilter && documentId ? (
        <>
          <DocumentScopePill
            documentId={documentId}
            onClear={() => onDocumentChange(null)}
          />
          <DocumentPickerPopover
            selectedIds={[documentId]}
            onSelectionChange={handleSelectionChange}
            disabled={disabled}
            trigger={
              <button
                type="button"
                className="text-xs text-muted-foreground hover:text-foreground underline-offset-2 hover:underline shrink-0"
              >
                {t("graph.documentFilter.change", "Change")}
              </button>
            }
          />
        </>
      ) : (
        <DocumentPickerPopover
          selectedIds={[]}
          onSelectionChange={handleSelectionChange}
          disabled={disabled}
          trigger={
            <button
              type="button"
              className={cn(
                "inline-flex items-center gap-1 rounded-md px-2 py-0.5 text-xs shrink-0",
                "text-muted-foreground hover:text-foreground hover:bg-muted/60 transition-colors",
              )}
            >
              <span>{t("graph.documentFilter.allEntities", "All entities")}</span>
              <ChevronDown className="h-3 w-3 opacity-70" />
            </button>
          }
        />
      )}

      {hasFilter && (
        <button
          type="button"
          onClick={() => onDocumentChange(null)}
          className="ml-auto text-xs text-muted-foreground hover:text-foreground shrink-0"
        >
          {t("graph.documentFilter.showAll", "Show full graph")}
        </button>
      )}
    </div>
  );
}
