"use client";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { CommunityLegend } from "@/components/graph/community-legend";
import { EntityTypeFilterList } from "@/components/graph/entity-type-filter-list";
import { useGraphStore } from "@/stores/use-graph-store";
import { EyeOff, Palette } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

interface GraphLegendProps {
  className?: string;
  collapsed?: boolean;
}

export function GraphLegend({ className, collapsed = true }: GraphLegendProps) {
  const { t } = useTranslation();
  const nodes = useGraphStore((s) => s.nodes);
  const colorMode = useGraphStore((s) => s.colorMode);
  const [isCollapsed, setIsCollapsed] = useState(collapsed);

  // SPEC-100: always mount legend control (empty graph keeps toolbar geometry)
  if (nodes.length === 0 || isCollapsed) {
    return (
      <Button
        variant="outline"
        size="icon"
        className={`h-9 w-9 graph-overlay-surface hover:bg-accent transition-shadow ${className}`}
        onClick={() => nodes.length > 0 && setIsCollapsed(false)}
        disabled={nodes.length === 0}
        aria-label={t("graph.legend.showLegend", "Show entity type legend")}
        title={t("graph.legend.showLegend", "Show Legend")}
        data-testid="spec100-graph-legend-slot"
      >
        <Palette className="h-4 w-4" aria-hidden="true" />
      </Button>
    );
  }

  const isCommunity = colorMode === "community";

  return (
    <Card
      className={`graph-overlay-surface w-80 shadow-md flex flex-col max-h-[calc(100vh-8rem)] ${className}`}
      role="region"
      aria-label={
        isCommunity
          ? t("graph.legend.communities", "Communities")
          : t("graph.legend.title", "Entity Types")
      }
    >
      <CardHeader className="py-3 px-4 shrink-0 border-b">
        <div className="flex items-center justify-between gap-2">
          <CardTitle className="text-sm font-semibold flex items-center gap-2.5">
            <Palette className="h-4 w-4 text-muted-foreground" aria-hidden="true" />
            <span>
              {isCommunity
                ? t("graph.legend.communities", "Communities")
                : t("graph.legend.title", "Entity Types")}
            </span>
            <Badge variant="secondary" className="h-5 px-1.5 text-xs tabular-nums">
              {nodes.length}
            </Badge>
          </CardTitle>
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7 hover:bg-muted -mr-1 shrink-0"
            onClick={() => setIsCollapsed(true)}
            aria-label={t("graph.legend.collapse", "Collapse legend")}
            title={t("graph.collapseLegend", "Collapse")}
          >
            <EyeOff className="h-4 w-4" aria-hidden="true" />
          </Button>
        </div>
      </CardHeader>
      <CardContent className="p-3 flex-1 min-h-0 overflow-hidden">
        {isCommunity ? (
          <CommunityLegend className="space-y-0.5 overflow-auto max-h-[min(24rem,50vh)]" />
        ) : (
          <EntityTypeFilterList listMaxHeight="max-h-[min(24rem,50vh)]" compact />
        )}
      </CardContent>
    </Card>
  );
}

export default GraphLegend;
