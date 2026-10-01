"use client";

/**
 * Single place for neighbourhood ("ego") actions so every entry point — the
 * panel, canvas keys 1/2/3, Enter, double-click — behaves identically.
 */
import { fitCameraToNodes } from "@/lib/graph/camera-utils";
import {
  clampEgoDepth,
  collectNeighborsBfs,
} from "@/lib/graph/engine/ego";
import { useGraphStore } from "@/stores/use-graph-store";
import { useCallback } from "react";

/**
 * Frame `nodeId` (default: selection) and its neighbours within `hops`.
 * Reads the store at call time, so it is safe in long-lived callbacks (the
 * engine captures its option callbacks once at creation).
 */
export function fitNeighbourhood(
  nodeId?: string | null,
  hops?: number,
): boolean {
  const { sigmaInstance, selectedNodeId, egoDepth } = useGraphStore.getState();
  const id = nodeId ?? selectedNodeId;
  if (!sigmaInstance || !id) return false;
  const graph = sigmaInstance.getGraph();
  if (!graph.hasNode(id)) return false;
  const ids = [id, ...collectNeighborsBfs(graph, id, hops ?? egoDepth)];
  return fitCameraToNodes(sigmaInstance, ids);
}

export function useNeighbourhood() {
  const selectedNodeId = useGraphStore((s) => s.selectedNodeId);
  const depth = useGraphStore((s) => s.egoDepth);
  const setEgoDepth = useGraphStore((s) => s.setEgoDepth);
  const selectNode = useGraphStore((s) => s.selectNode);

  const fit = useCallback(
    (nodeId?: string | null, hops?: number) => fitNeighbourhood(nodeId, hops),
    [],
  );

  /** Change the depth; with a selection, bring the new ring into view. */
  const setDepth = useCallback(
    (next: number) => {
      const clamped = clampEgoDepth(next);
      setEgoDepth(clamped);
      if (selectedNodeId) fitNeighbourhood(selectedNodeId, clamped);
    },
    [setEgoDepth, selectedNodeId],
  );

  const clear = useCallback(() => selectNode(null), [selectNode]);

  return { depth, hasSelection: Boolean(selectedNodeId), fit, setDepth, clear };
}
