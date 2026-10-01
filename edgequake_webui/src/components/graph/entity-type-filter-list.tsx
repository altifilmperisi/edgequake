"use client";

/**
 * Shared entity-type visibility controls for graph sidebar + floating legend (DRY).
 */

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { EntityTypeColorSwatch } from "@/components/graph/entity-type-color-swatch";
import { useEntityTypeColors } from "@/hooks/use-entity-type-colors";
import { cn } from "@/lib/utils";
import { useGraphStore } from "@/stores/use-graph-store";
import { Eye, EyeOff } from "lucide-react";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";

export interface EntityTypeStat {
  type: string;
  count: number;
  color: string;
  label: string;
}

export function useEntityTypeStats(): EntityTypeStat[] {
  const nodes = useGraphStore((s) => s.nodes);
  const { colorFor } = useEntityTypeColors();
  const { t } = useTranslation();

  return useMemo(() => {
    const stats = new Map<string, number>();
    for (const node of nodes) {
      const type = node.node_type || "unknown";
      stats.set(type, (stats.get(type) ?? 0) + 1);
    }

    return Array.from(stats.entries())
      .sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))
      .map(([type, count]) => {
        const normalized = type.toUpperCase();
        return {
          type,
          count,
          color: colorFor(type),
          label: t(
            `graph.nodeTypes.${normalized.toLowerCase()}`,
            type.charAt(0).toUpperCase() + type.slice(1).toLowerCase(),
          ),
        };
      });
  }, [nodes, t, colorFor]);
}

export interface EntityTypeFilterListProps {
  typeQuery?: string;
  className?: string;
  /** Fixed max-height class when not filling parent (e.g. legend). */
  listMaxHeight?: string;
  /** Grow/scroll within a flex parent instead of a fixed max-height. */
  fillHeight?: boolean;
  compact?: boolean;
}

export function EntityTypeFilterList({
  typeQuery = "",
  className,
  listMaxHeight = "max-h-52",
  fillHeight = false,
  compact = false,
}: EntityTypeFilterListProps) {
  const { t } = useTranslation();
  const nodes = useGraphStore((s) => s.nodes);
  const visibleEntityTypes = useGraphStore((s) => s.visibleEntityTypes);
  const toggleEntityType = useGraphStore((s) => s.toggleEntityType);
  const setVisibleEntityTypes = useGraphStore((s) => s.setVisibleEntityTypes);

  const typeStats = useEntityTypeStats();
  const { colors, setTypeColor, resetTypeColor } = useEntityTypeColors();
  const normalizedQuery = typeQuery.trim().toLowerCase();

  const filteredStats = useMemo(() => {
    if (!normalizedQuery) return typeStats;
    return typeStats.filter(
      (item) =>
        item.label.toLowerCase().includes(normalizedQuery) ||
        item.type.toLowerCase().includes(normalizedQuery),
    );
  }, [typeStats, normalizedQuery]);

  const allTypes = useMemo(() => typeStats.map((s) => s.type), [typeStats]);

  const visibleNodeCount = useMemo(
    () => nodes.filter((n) => visibleEntityTypes.has(n.node_type)).length,
    [nodes, visibleEntityTypes],
  );

  const hiddenTypeCount = allTypes.filter(
    (type) => !visibleEntityTypes.has(type),
  ).length;

  const allVisible =
    allTypes.length > 0 && allTypes.every((ty) => visibleEntityTypes.has(ty));
  const noneVisible = allTypes.length > 0 && visibleNodeCount === 0;

  if (typeStats.length === 0) {
    return (
      <p className="text-xs text-muted-foreground px-1 py-2">
        {t("graph.filters.noTypes", "No entity types in the current graph.")}
      </p>
    );
  }

  return (
    <div
      className={cn(
        "flex flex-col gap-2 min-h-0",
        fillHeight && "h-full",
        className,
      )}
    >
      <div className="flex items-center justify-between gap-2 px-0.5 shrink-0">
        <p
          className={cn(
            "text-xs tabular-nums leading-snug",
            noneVisible
              ? "text-amber-600 dark:text-amber-400 font-medium"
              : "text-muted-foreground",
          )}
        >
          {t("graph.filters.visibleSummary", "{{visible}} of {{total}} nodes visible", {
            visible: visibleNodeCount,
            total: nodes.length,
          })}
        </p>
        <div className="flex items-center gap-1 shrink-0">
          {!allVisible && (
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="h-7 px-2 text-xs font-medium"
              onClick={() => setVisibleEntityTypes(allTypes)}
            >
              {t("graph.showAll", "Show All")}
            </Button>
          )}
          {!noneVisible && hiddenTypeCount < allTypes.length && (
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="h-7 px-2 text-xs font-medium text-muted-foreground"
              onClick={() => setVisibleEntityTypes([])}
            >
              {t("graph.filters.hideAll", "Hide all")}
            </Button>
          )}
        </div>
      </div>

      {noneVisible && (
        <div className="rounded-md border border-amber-500/30 bg-amber-500/10 px-2.5 py-2 text-xs text-amber-800 dark:text-amber-200 shrink-0">
          {t(
            "graph.filters.allHiddenHint",
            "All types are hidden. Show at least one category to see the graph.",
          )}
        </div>
      )}

      <ScrollArea
        className={cn(
          "rounded-lg border border-border/50 bg-muted/20 min-h-0",
          fillHeight ? "flex-1 h-full" : listMaxHeight,
        )}
        showShadows
      >
        <div
          className={cn("p-1.5 space-y-0.5", compact && "p-1 space-y-0")}
          role="list"
          aria-label={t(
            "graph.legend.typeList",
            "Entity type visibility controls",
          )}
        >
          {filteredStats.length === 0 ? (
            <p className="text-xs text-muted-foreground px-2 py-3 text-center">
              {t("graph.filters.noMatchingTypes", "No matching types")}
            </p>
          ) : (
            filteredStats.map(({ type, count, color, label }) => {
              const isVisible = visibleEntityTypes.has(type);
              // WHY: row is a div — swatch is its own <button>; nesting buttons
              // inside buttons is invalid HTML and breaks hydration (SPEC-102).
              return (
                <div
                  key={type}
                  role="listitem"
                  className={cn(
                    "w-full flex items-center gap-2.5 rounded-md",
                    "hover:bg-background/80",
                    compact ? "px-2 py-1.5" : "px-2.5 py-2",
                    !isVisible && "opacity-45",
                  )}
                >
                  <EntityTypeColorSwatch
                    entityType={type}
                    color={color}
                    overrides={colors}
                    onChange={(hex) => void setTypeColor(type, hex)}
                    onReset={() => void resetTypeColor(type)}
                  />
                  <button
                    type="button"
                    className={cn(
                      "flex-1 flex items-center gap-2.5 min-w-0 text-left rounded-md",
                      "focus:outline-none focus-visible:ring-2 focus-visible:ring-primary/40",
                    )}
                    onClick={() => toggleEntityType(type)}
                    aria-pressed={isVisible}
                    aria-label={`${label}: ${count}. ${isVisible ? t("graph.legend.clickToHide", "Click to hide") : t("graph.legend.clickToShow", "Click to show")}`}
                  >
                    <span
                      className={cn(
                        "flex-1 truncate font-medium min-w-0",
                        compact ? "text-xs" : "text-xs",
                      )}
                    >
                      {label}
                    </span>
                    <Badge
                      variant={isVisible ? "secondary" : "outline"}
                      className="h-5 min-w-7 px-1.5 text-xs font-semibold tabular-nums shrink-0 justify-center"
                    >
                      {count}
                    </Badge>
                    {isVisible ? (
                      <Eye
                        className="h-3.5 w-3.5 text-primary/70 shrink-0"
                        aria-hidden
                      />
                    ) : (
                      <EyeOff
                        className="h-3.5 w-3.5 text-muted-foreground shrink-0"
                        aria-hidden
                      />
                    )}
                  </button>
                </div>
              );
            })
          )}
        </div>
      </ScrollArea>
    </div>
  );
}
