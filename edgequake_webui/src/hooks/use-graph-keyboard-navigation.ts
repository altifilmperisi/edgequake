"use client";

/**
 * @module use-graph-keyboard-navigation
 * @description Keyboard navigation for Graph Studio (SPEC-155 F-155-G10).
 * Never preventDefault Tab globally. Keys act only when the canvas is focused:
 * arrows = move spatially, 1/2/3 = neighbourhood depth, Enter = fit neighbourhood,
 * Esc = clear, +/-/0 = zoom, f = fullscreen.
 */

import {
  focusCameraOnNode,
  resetCameraToFitGraph,
} from "@/lib/graph/camera-utils";
import {
  pickCentralNode,
  pickNodeInDirection,
  type ArrowDirection,
  type NavPoint,
} from "@/lib/graph/spatial-navigation";
import { useNeighbourhood } from "@/hooks/use-neighbourhood";
import { useGraphStore } from "@/stores/use-graph-store";
import type Sigma from "sigma";
import { useCallback, useEffect } from "react";

export interface GraphKeyboardOptions {
  enabled?: boolean;
  onNodeFocus?: (nodeId: string) => void;
  onDeselect?: () => void;
  /** ContextMenu key / Shift+F10 on the selected node (client coordinates). */
  onOpenContextMenu?: (nodeId: string, clientX: number, clientY: number) => void;
}

function isTypingTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  const tag = target.tagName;
  return (
    tag === "INPUT" ||
    tag === "TEXTAREA" ||
    tag === "SELECT" ||
    target.isContentEditable
  );
}

function revealNode(sigma: Sigma, nodeId: string): void {
  const display = sigma.getNodeDisplayData(nodeId);
  const graph = sigma.getGraph();
  if (!display || !graph.hasNode(nodeId)) return;
  const p = sigma.graphToViewport({
    x: graph.getNodeAttribute(nodeId, "x") as number,
    y: graph.getNodeAttribute(nodeId, "y") as number,
  });
  const { width, height } = sigma.getDimensions();
  const inside =
    p.x >= EDGE_MARGIN_PX &&
    p.x <= width - EDGE_MARGIN_PX &&
    p.y >= EDGE_MARGIN_PX &&
    p.y <= height - EDGE_MARGIN_PX;
  if (!inside) {
    sigma.getCamera().animate({ x: display.x, y: display.y }, { duration: 250 });
  }
}

function isGraphCanvasTarget(target: EventTarget | null): boolean {
  if (!(target instanceof Element)) return false;
  return Boolean(
    target.closest("[data-graph-engine], [data-graph-engine-id]"),
  );
}

const ARROW_DIRECTIONS: Record<string, ArrowDirection> = {
  ArrowUp: "up",
  ArrowDown: "down",
  ArrowLeft: "left",
  ArrowRight: "right",
};

/** Keep the node on screen without recentering the whole view on every step. */
const EDGE_MARGIN_PX = 60;

export function useGraphKeyboardNavigation(options: GraphKeyboardOptions = {}) {
  const { enabled = true, onNodeFocus, onDeselect, onOpenContextMenu } = options;

  const sigmaInstance = useGraphStore((s) => s.sigmaInstance);
  const selectedNodeId = useGraphStore((s) => s.selectedNodeId);
  const selectNode = useGraphStore((s) => s.selectNode);
  const { depth, setDepth, fit: fitNeighbourhood } = useNeighbourhood();

  /** Arrow key = screen direction, walking along connections when possible. */
  const navigateInDirection = useCallback(
    (direction: ArrowDirection) => {
      if (!sigmaInstance) return;
      const graph = sigmaInstance.getGraph();
      const points: NavPoint[] = [];
      graph.forEachNode((id, attrs) => {
        if (attrs.hidden) return;
        const p = sigmaInstance.graphToViewport({ x: attrs.x, y: attrs.y });
        points.push({ id, x: p.x, y: p.y });
      });

      const current = selectedNodeId
        ? points.find((p) => p.id === selectedNodeId)
        : undefined;
      let nextId: string | null;
      if (current) {
        nextId = pickNodeInDirection(
          current,
          direction,
          new Set(graph.neighbors(current.id)),
          points,
        );
      } else {
        const { width, height } = sigmaInstance.getDimensions();
        nextId = pickCentralNode(points, { x: width / 2, y: height / 2 });
      }
      if (!nextId) return;

      selectNode(nextId);
      onNodeFocus?.(nextId);
      revealNode(sigmaInstance, nextId);
    },
    [sigmaInstance, selectedNodeId, selectNode, onNodeFocus],
  );

  /** Keyboard twin of right-click: open the menu beside the selected node. */
  const openContextMenuForSelection = useCallback(() => {
    if (!sigmaInstance || !selectedNodeId || !onOpenContextMenu) return;
    const graph = sigmaInstance.getGraph();
    if (!graph.hasNode(selectedNodeId)) return;
    const p = sigmaInstance.graphToViewport({
      x: graph.getNodeAttribute(selectedNodeId, "x") as number,
      y: graph.getNodeAttribute(selectedNodeId, "y") as number,
    });
    const rect = sigmaInstance.getContainer().getBoundingClientRect();
    onOpenContextMenu(selectedNodeId, rect.left + p.x, rect.top + p.y);
  }, [sigmaInstance, selectedNodeId, onOpenContextMenu]);

  const handleZoomIn = useCallback(() => {
    sigmaInstance?.getCamera().animatedZoom({ duration: 200, factor: 1.5 });
  }, [sigmaInstance]);

  const handleZoomOut = useCallback(() => {
    sigmaInstance?.getCamera().animatedUnzoom({ duration: 200, factor: 1.5 });
  }, [sigmaInstance]);

  const handleResetZoom = useCallback(() => {
    if (sigmaInstance) resetCameraToFitGraph(sigmaInstance, 500);
  }, [sigmaInstance]);

  /** Enter: frame the selected node together with its neighbourhood. */
  const handleFocusSelected = useCallback(() => {
    if (!selectedNodeId) return;
    if (!fitNeighbourhood(selectedNodeId, depth) && sigmaInstance) {
      focusCameraOnNode(sigmaInstance, selectedNodeId, { duration: 500 });
    }
  }, [sigmaInstance, selectedNodeId, fitNeighbourhood, depth]);

  /** Esc: clear selection and close the details panel. */
  const handleDeselect = useCallback(() => {
    selectNode(null);
    onDeselect?.();
  }, [selectNode, onDeselect]);

  const handleFullscreen = useCallback(() => {
    const container = document.querySelector("[data-graph-container]");
    if (!container) return;
    if (!document.fullscreenElement) {
      const isDark = document.documentElement.classList.contains("dark");
      if (isDark) container.classList.add("dark");
      container.requestFullscreen?.();
    } else {
      container.classList.remove("dark");
      document.exitFullscreen?.();
    }
  }, []);

  useEffect(() => {
    if (!enabled) return;

    const handleKeyDown = (event: KeyboardEvent) => {
      if (isTypingTarget(event.target)) return;

      const canvasFocused = isGraphCanvasTarget(event.target);

      switch (event.key) {
        case "Tab":
          // G10: never preventDefault Tab — let focus leave the canvas
          return;

        case "ArrowRight":
        case "ArrowDown":
        case "ArrowLeft":
        case "ArrowUp": {
          if (!canvasFocused || event.ctrlKey || event.metaKey || event.altKey) return;
          event.preventDefault();
          navigateInDirection(ARROW_DIRECTIONS[event.key]!);
          break;
        }

        case "1":
        case "2":
        case "3":
          if (!canvasFocused || event.ctrlKey || event.metaKey || event.altKey) return;
          event.preventDefault();
          setDepth(Number(event.key));
          break;

        case "ContextMenu":
          if (!canvasFocused || !selectedNodeId) return;
          event.preventDefault();
          openContextMenuForSelection();
          break;

        case "F10":
          if (!canvasFocused || !event.shiftKey || !selectedNodeId) return;
          event.preventDefault();
          openContextMenuForSelection();
          break;

        case "Enter":
          if (!canvasFocused || !selectedNodeId) return;
          event.preventDefault();
          handleFocusSelected();
          break;

        case "Escape":
          if (!canvasFocused || !selectedNodeId) return;
          event.preventDefault();
          handleDeselect();
          break;

        case "+":
        case "=":
          if (!canvasFocused) return;
          if (!event.ctrlKey && !event.metaKey) {
            event.preventDefault();
            handleZoomIn();
          }
          break;

        case "-":
        case "_":
          if (!canvasFocused) return;
          if (!event.ctrlKey && !event.metaKey) {
            event.preventDefault();
            handleZoomOut();
          }
          break;

        case "0":
          if (!canvasFocused) return;
          if (!event.ctrlKey && !event.metaKey) {
            event.preventDefault();
            handleResetZoom();
          }
          break;

        case "f":
        case "F":
          if (!canvasFocused) return;
          if (!event.ctrlKey && !event.metaKey) {
            event.preventDefault();
            handleFullscreen();
          }
          break;
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [
    enabled,
    navigateInDirection,
    setDepth,
    selectedNodeId,
    handleFocusSelected,
    handleDeselect,
    openContextMenuForSelection,
    handleZoomIn,
    handleZoomOut,
    handleResetZoom,
    handleFullscreen,
  ]);

  return {
    navigateInDirection,
    handleZoomIn,
    handleZoomOut,
    handleResetZoom,
    handleFocusSelected,
    handleDeselect,
    handleFullscreen,
  };
}

export default useGraphKeyboardNavigation;
