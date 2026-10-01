/**
 * Neighbourhood depth control (a.k.a. "ego depth").
 *
 * PURPOSE: after you select a node, keep only its N-hop neighbourhood bright and
 * dim the rest of the canvas — N is chosen here (1–3). The engine reads
 * `egoDepth` from the graph store (GraphRenderer → engine.setEgoDepth), so this
 * control only edits a preference and never overwrites other canvas focus
 * (answer / path) the way the old stub did.
 *
 * UX rules: always interactive (picking a depth before selecting is valid),
 * shows what it will do / did ("12 nodes within 2 hops"), explains when
 * neighbour highlighting is switched off, and offers a one-click way out.
 */
"use client";

import { Button } from "@/components/ui/button";
import { useNeighbourhood } from "@/hooks/use-neighbourhood";
import {
  clampEgoDepth,
  collectNeighborsBfs,
  MAX_EGO_DEPTH,
  MIN_EGO_DEPTH,
} from "@/lib/graph/engine/ego";
import { cn } from "@/lib/utils";
import { useGraphStore } from "@/stores/use-graph-store";
import { useSettingsStore } from "@/stores/use-settings-store";
import { Maximize2, Network, X } from "lucide-react";
import { useCallback, useId, useMemo, type KeyboardEvent } from "react";
import { useTranslation } from "react-i18next";

const DEPTHS = Array.from(
  { length: MAX_EGO_DEPTH - MIN_EGO_DEPTH + 1 },
  (_, i) => MIN_EGO_DEPTH + i,
);

interface EgoDepthSliderProps {
  className?: string;
}

export function EgoDepthSlider({ className }: EgoDepthSliderProps) {
  const { t } = useTranslation();
  const groupLabelId = useId();
  const selectedNodeId = useGraphStore((s) => s.selectedNodeId);
  const { depth: egoDepth, setDepth: setEgoDepth, fit, clear } = useNeighbourhood();
  const sigma = useGraphStore((s) => s.sigmaInstance);
  const nodeCount = useGraphStore((s) => s.nodes.length);
  const edgeCount = useGraphStore((s) => s.edges.length);
  const highlightNeighbors = useSettingsStore(
    (s) => s.graphSettings.highlightNeighbors ?? true,
  );
  const setGraphSettings = useSettingsStore((s) => s.setGraphSettings);

  // Live size of the neighbourhood the canvas is currently keeping bright.
  const reach = useMemo(() => {
    void nodeCount;
    void edgeCount; // recompute when streamed data changes the topology
    const graph = sigma?.getGraph();
    if (!graph || !selectedNodeId || !graph.hasNode(selectedNodeId)) return null;
    return {
      label: String(graph.getNodeAttribute(selectedNodeId, "label") ?? selectedNodeId),
      count: collectNeighborsBfs(graph, selectedNodeId, egoDepth).size,
    };
  }, [sigma, selectedNodeId, egoDepth, nodeCount, edgeCount]);

  const onKeyDown = useCallback(
    (event: KeyboardEvent<HTMLDivElement>) => {
      const step =
        event.key === "ArrowRight" || event.key === "ArrowUp"
          ? 1
          : event.key === "ArrowLeft" || event.key === "ArrowDown"
            ? -1
            : 0;
      if (step === 0) return;
      event.preventDefault();
      setEgoDepth(clampEgoDepth(egoDepth + step));
    },
    [egoDepth, setEgoDepth],
  );

  return (
    <div
      className={cn("flex flex-col gap-2", className)}
      data-testid="ego-depth-slider"
    >
      <div className="flex items-center justify-between gap-2">
        <div
          id={groupLabelId}
          className="flex items-center gap-1.5 text-xs font-medium text-foreground"
          title={t(
            "graph.ego.hint",
            "Keep only nodes within N hops of the selected node bright; dim the rest",
          )}
        >
          <Network className="h-3.5 w-3.5 text-muted-foreground" aria-hidden />
          {t("graph.ego.title", "Neighbourhood")}
        </div>
        {selectedNodeId ? (
          <div className="-mr-1 flex items-center">
            <Button
              type="button"
              variant="ghost"
              size="icon"
              className="h-6 w-6"
              onClick={() => fit()}
              aria-label={t("graph.ego.fit", "Fit neighbourhood in view")}
              title={t("graph.ego.fit", "Fit neighbourhood in view")}
              data-testid="ego-fit"
            >
              <Maximize2 className="h-3.5 w-3.5" aria-hidden />
            </Button>
            <Button
              type="button"
              variant="ghost"
              size="icon"
              className="h-6 w-6"
              onClick={clear}
              aria-label={t("graph.ego.clear", "Clear selection")}
              title={t("graph.ego.clear", "Clear selection")}
              data-testid="ego-clear"
            >
              <X className="h-3.5 w-3.5" aria-hidden />
            </Button>
          </div>
        ) : null}
      </div>

      <div
        role="radiogroup"
        aria-labelledby={groupLabelId}
        onKeyDown={onKeyDown}
        className="grid grid-cols-3 gap-0.5 rounded-md bg-muted p-0.5"
      >
        {DEPTHS.map((depth) => {
          const active = depth === egoDepth;
          return (
            <button
              key={depth}
              type="button"
              role="radio"
              aria-checked={active}
              tabIndex={active ? 0 : -1}
              data-testid={`ego-depth-${depth}`}
              onClick={() => setEgoDepth(depth)}
              className={cn(
                "h-7 rounded-[5px] text-xs font-medium tabular-nums transition-colors",
                "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
                active
                  ? "bg-background text-foreground shadow-sm"
                  : "text-muted-foreground hover:text-foreground",
              )}
            >
              {t("graph.ego.hops", { count: depth, defaultValue_one: "{{count}} hop", defaultValue_other: "{{count}} hops" })}
            </button>
          );
        })}
      </div>

      <p
        className="min-h-8 line-clamp-2 text-xs leading-4 text-muted-foreground"
        title={reach?.label}
        aria-live="polite"
        data-testid="ego-status"
      >
        {!highlightNeighbors
          ? t("graph.ego.highlightOff", "Neighbour highlighting is off.")
          : reach
            ? t("graph.ego.summary", {
                count: reach.count,
                defaultValue_one: "{{count}} node in reach",
                defaultValue_other: "{{count}} nodes in reach",
              })
            : t("graph.ego.selectPrompt", "Click a node to focus on its neighbourhood")}
      </p>

      {!highlightNeighbors ? (
        <Button
          type="button"
          variant="outline"
          size="sm"
          className="h-7 text-xs"
          onClick={() => setGraphSettings({ highlightNeighbors: true })}
        >
          {t("graph.ego.enable", "Turn on")}
        </Button>
      ) : null}
    </div>
  );
}
