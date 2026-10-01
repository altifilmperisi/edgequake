/**
 * Community legend from GET /graph/communities (SPEC-155 W5).
 */
"use client";

import { getGraphCommunities, type GraphCommunityItem } from "@/lib/api/edgequake/graph";
import { resolveGraphTheme } from "@/lib/graph/engine";
import { formatNumber } from "@/lib/format";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { useMemo } from "react";

function colorForCommunity(id: string, palette: string[]): string {
  let h = 0;
  for (let i = 0; i < id.length; i++) {
    h = (h * 31 + id.charCodeAt(i)) >>> 0;
  }
  return palette[h % palette.length] ?? "#64748b";
}

export function CommunityLegend({ className }: { className?: string }) {
  const { t } = useTranslation();
  const { data, isLoading, isError } = useQuery({
    queryKey: ["graph-communities"],
    queryFn: getGraphCommunities,
    staleTime: 60_000,
  });

  const palette = useMemo(() => {
    if (typeof document === "undefined") return ["#64748b"];
    return resolveGraphTheme(document.documentElement).communities;
  }, []);

  const items: GraphCommunityItem[] = data?.items ?? [];

  if (isLoading) {
    return (
      <p className="text-xs text-muted-foreground p-2" data-testid="community-legend">
        {t("graph.legend.communitiesLoading", "Loading communities…")}
      </p>
    );
  }

  if (isError || items.length === 0) {
    return (
      <p className="text-xs text-muted-foreground p-2" data-testid="community-legend">
        {t("graph.legend.communitiesEmpty", "No communities available")}
      </p>
    );
  }

  return (
    <ul
      className={className}
      data-testid="community-legend"
      aria-label={t("graph.legend.communities", "Communities")}
    >
      {items.map((c) => (
        <li key={c.id} className="flex items-center gap-2 py-1 text-sm">
          <span
            className="h-3 w-3 rounded-sm shrink-0 ring-1 ring-border"
            style={{ backgroundColor: colorForCommunity(c.id, palette) }}
            aria-hidden
          />
          <span className="truncate flex-1">{c.label || c.id}</span>
          <span className="text-xs text-muted-foreground tabular-nums">
            {formatNumber(c.size)}
          </span>
        </li>
      ))}
    </ul>
  );
}
