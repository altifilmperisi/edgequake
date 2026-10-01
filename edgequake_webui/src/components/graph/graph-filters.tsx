/**
 * @module GraphFilters
 * @description Entity type filter panel for knowledge graph sidebar.
 * Search is debounced before hitting the store (SPEC-155 F-155-G02).
 */

"use client";

import { Input } from "@/components/ui/input";
import { EntityTypeFilterList } from "@/components/graph/entity-type-filter-list";
import { useEntityTypeStats } from "@/components/graph/entity-type-filter-list";
import { useGraphStore } from "@/stores/use-graph-store";
import { cn } from "@/lib/utils";
import { FileText, Filter, Search, X } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

const SEARCH_DEBOUNCE_MS = 200;

export interface GraphFiltersProps {
  className?: string;
  /** When true, list grows to fill parent height (sidebar layout). */
  fillHeight?: boolean;
}

export function GraphFilters({
  className,
  fillHeight = false,
}: GraphFiltersProps) {
  const { t } = useTranslation();
  const nodes = useGraphStore((s) => s.nodes);
  const documentFilterId = useGraphStore((s) => s.documentFilterId);
  const searchQuery = useGraphStore((s) => s.searchQuery);
  const setSearchQuery = useGraphStore((s) => s.setSearchQuery);
  const typeStats = useEntityTypeStats();

  // Local input for snappy typing; store updates are debounced (G02)
  const [localQuery, setLocalQuery] = useState(searchQuery);
  useEffect(() => {
    setLocalQuery(searchQuery);
  }, [searchQuery]);

  useEffect(() => {
    if (localQuery === searchQuery) return;
    const handle = window.setTimeout(() => {
      setSearchQuery(localQuery);
    }, SEARCH_DEBOUNCE_MS);
    return () => window.clearTimeout(handle);
  }, [localQuery, searchQuery, setSearchQuery]);

  // SPEC-100: keep filters panel mounted with empty state (never null→tall CLS)
  if (nodes.length === 0) {
    return (
      <div
        className={cn(
          "flex flex-col gap-3 min-h-[8rem]",
          fillHeight && "h-full",
          className,
        )}
        data-testid="spec100-graph-filters-empty"
      >
        <div className="flex items-center gap-1.5 shrink-0">
          <Filter className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
          <h4 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
            {t("graph.filters.title", "Filters")}
          </h4>
        </div>
        <p className="text-xs text-muted-foreground">
          {t("graph.filters.empty", "Filters appear when the graph has entities.")}
        </p>
      </div>
    );
  }

  return (
    <div
      className={cn(
        "flex flex-col gap-3 min-h-0",
        fillHeight && "h-full",
        className,
      )}
      data-testid="spec100-graph-filters"
    >
      <div className="flex items-center justify-between gap-2 shrink-0">
        <div className="flex items-center gap-1.5 min-w-0">
          <Filter className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
          <h4 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
            {t("graph.filters.title", "Filters")}
          </h4>
        </div>
        {documentFilterId && (
          <span
            className="inline-flex items-center gap-1 max-w-[140px] rounded-full bg-primary/10 px-2 py-0.5 text-xs font-medium text-primary truncate"
            title={documentFilterId}
          >
            <FileText className="h-3 w-3 shrink-0" aria-hidden />
            <span className="truncate">
              {t("graph.filters.documentScoped", "Document view")}
            </span>
          </span>
        )}
      </div>

      <div className="relative shrink-0">
        <Search
          className="absolute left-2.5 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-muted-foreground pointer-events-none"
          aria-hidden
        />
        <Input
          placeholder={t(
            "graph.filters.searchPlaceholder",
            "Search entities or types…",
          )}
          value={localQuery}
          onChange={(e) => setLocalQuery(e.target.value)}
          className="h-9 pl-8 pr-8 text-xs bg-background border-border/60 focus-visible:ring-primary/30"
          aria-label={t(
            "graph.filters.searchPlaceholder",
            "Search entities or types",
          )}
          data-testid="graph-filter-search"
        />
        {localQuery && (
          <button
            type="button"
            onClick={() => {
              setLocalQuery("");
              setSearchQuery("");
            }}
            className="absolute right-2 top-1/2 -translate-y-1/2 rounded-sm p-0.5 text-muted-foreground hover:text-foreground"
            aria-label={t("graph.filters.clearSearch", "Clear search")}
          >
            <X className="h-3.5 w-3.5" />
          </button>
        )}
      </div>

      <div className={cn("flex flex-col min-h-0 gap-2", fillHeight && "flex-1")}>
        <div className="flex items-center justify-between gap-2 shrink-0 px-0.5">
          <h5 className="text-xs font-semibold text-muted-foreground uppercase tracking-wider">
            {t("graph.filters.entityTypes", "Entity types")}
          </h5>
          <span className="text-xs tabular-nums text-muted-foreground">
            {t("graph.filters.typeCount", "{{count}} types", {
              count: typeStats.length,
            })}
          </span>
        </div>
        <EntityTypeFilterList
          typeQuery={localQuery}
          fillHeight={fillHeight}
          listMaxHeight={fillHeight ? undefined : "max-h-72"}
          className={fillHeight ? "flex-1 min-h-0" : undefined}
        />
      </div>
    </div>
  );
}

export default GraphFilters;
