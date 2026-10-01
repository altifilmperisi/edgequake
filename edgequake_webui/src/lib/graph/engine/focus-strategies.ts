/**
 * Focus strategies — dim non-matches (LAW-155-4). Never hide:true for filters.
 * W5 may extend ego/path/answer; W4 ships dim for hover/select/filter.
 */

import type { FocusMode, FocusState, GraphFiltersState } from "./types";

export const DIM_NODE_OPACITY = 0.18;
export const DIM_EDGE_OPACITY = 0.12;
export const FOCUS_NODE_OPACITY = 1;
export const FOCUS_EDGE_OPACITY = 1;

export interface FocusContext {
  focus: FocusState;
  filters: GraphFiltersState;
  /** Node ids that pass the filter pipeline (dim others). */
  matchingNodeIds: Set<string>;
  /** Neighbor ids of hovered/selected focus node. */
  neighborIds: Set<string>;
  highlightNeighbors: boolean;
  /**
   * Hops covered by `neighborIds` for select focus (default 1). Above 1 an edge
   * stays lit when *both* ends are inside the neighbourhood, not only when it
   * touches the selected node.
   */
  focusDepth?: number;
}

export function isNodeDimmed(
  nodeId: string,
  ctx: FocusContext,
): boolean {
  if (!ctx.matchingNodeIds.has(nodeId)) return true;

  const focusId = ctx.focus.ids[0];
  if (
    ctx.highlightNeighbors &&
    (ctx.focus.mode === "hover" || ctx.focus.mode === "select") &&
    focusId
  ) {
    if (nodeId === focusId) return false;
    if (ctx.neighborIds.has(nodeId)) return false;
    return true;
  }

  if (ctx.focus.mode === "answer" || ctx.focus.mode === "path") {
    if (ctx.focus.ids.length === 0) return false;
    return !ctx.focus.ids.includes(nodeId);
  }

  // Ego: seed + BFS neighbours within depth stay bright (W5).
  if (ctx.focus.mode === "ego") {
    const seed = ctx.focus.ids[0];
    if (!seed) return false;
    if (nodeId === seed) return false;
    return !ctx.neighborIds.has(nodeId);
  }

  return false;
}

export function isEdgeDimmed(
  edgeId: string,
  source: string,
  target: string,
  ctx: FocusContext,
): boolean {
  if (!ctx.matchingNodeIds.has(source) || !ctx.matchingNodeIds.has(target)) {
    return true;
  }

  const focusId = ctx.focus.ids[0];
  if (
    ctx.highlightNeighbors &&
    (ctx.focus.mode === "hover" || ctx.focus.mode === "select") &&
    focusId
  ) {
    if (ctx.focus.mode === "select" && (ctx.focusDepth ?? 1) > 1) {
      const inHood = (id: string) => id === focusId || ctx.neighborIds.has(id);
      return !(inHood(source) && inHood(target));
    }
    return source !== focusId && target !== focusId;
  }

  if (ctx.focus.mode === "answer" || ctx.focus.mode === "path") {
    if (ctx.focus.ids.length === 0) return false;
    return !(
      ctx.focus.ids.includes(source) && ctx.focus.ids.includes(target)
    );
  }

  if (ctx.focus.mode === "ego") {
    const seed = ctx.focus.ids[0];
    if (!seed) return false;
    const inEgo = (id: string) => id === seed || ctx.neighborIds.has(id);
    return !(inEgo(source) && inEgo(target));
  }

  return false;
}

/**
 * Selection outranks hover: hover previews only while nothing is selected, so
 * moving the pointer across the canvas never discards the selected node's focus.
 */
export function focusModeFromHoverSelect(
  hoveredId: string | null,
  selectedId: string | null,
): FocusState {
  if (selectedId) {
    return { mode: "select" as FocusMode, ids: [selectedId] };
  }
  if (hoveredId) {
    return { mode: "hover" as FocusMode, ids: [hoveredId] };
  }
  return { mode: "none", ids: [] };
}

export function nodeReducerAttrs(
  attrs: Record<string, unknown>,
  dimmed: boolean,
  emphasized: boolean,
  focusColor: string,
  /** 1 = full strength; <1 softens outer neighbourhood rings (see hopFade). */
  fade = 1,
): Record<string, unknown> {
  if (emphasized) {
    return {
      ...attrs,
      // dim via opacity — never hidden
      hidden: false,
      color: typeof attrs.color === "string" ? attrs.color : focusColor,
      // Ring in the focus colour = unmistakable "this one" marker.
      borderColor: focusColor,
      borderSize: 0.3,
      zIndex: 100,
      forceLabel: true,
      // Sigma uses size / color; opacity via custom attr read by reducer consumers
      _dimmed: false,
      _opacity: FOCUS_NODE_OPACITY,
    };
  }
  if (dimmed) {
    return {
      ...attrs,
      hidden: false,
      _dimmed: true,
      _opacity: DIM_NODE_OPACITY,
      forceLabel: false,
      // Soften colour toward muted without removing the node from the mental map
      color: softenColor(
        typeof attrs.color === "string" ? attrs.color : "#94a3b8",
        DIM_NODE_OPACITY,
      ),
    };
  }
  if (fade < 1) {
    return {
      ...attrs,
      hidden: false,
      _dimmed: false,
      _opacity: fade,
      color: softenColor(
        typeof attrs.color === "string" ? attrs.color : "#94a3b8",
        fade,
      ),
    };
  }
  return {
    ...attrs,
    hidden: false,
    _dimmed: false,
    _opacity: FOCUS_NODE_OPACITY,
  };
}

export interface EdgeReducerOptions {
  /** Draw the relationship label on an emphasised edge (default true). */
  showLabel?: boolean;
  /** <1 softens edges in the outer neighbourhood rings. */
  fade?: number;
}

export function edgeReducerAttrs(
  attrs: Record<string, unknown>,
  dimmed: boolean,
  emphasized: boolean,
  focusColor: string,
  defaultEdgeColor: string,
  { showLabel = true, fade = 1 }: EdgeReducerOptions = {},
): Record<string, unknown> {
  if (emphasized) {
    return {
      ...attrs,
      hidden: false,
      color: focusColor,
      size: (typeof attrs.size === "number" ? attrs.size : 2) * 1.6,
      forceLabel: showLabel && !!attrs.label,
      _dimmed: false,
    };
  }
  if (!dimmed && fade < 1) {
    return {
      ...attrs,
      hidden: false,
      color: softenColor(defaultEdgeColor, fade),
      _dimmed: false,
    };
  }
  if (dimmed) {
    return {
      ...attrs,
      hidden: false,
      color: softenColor(defaultEdgeColor, DIM_EDGE_OPACITY),
      forceLabel: false,
      _dimmed: true,
    };
  }
  return { ...attrs, hidden: false, _dimmed: false };
}

/** Approximate alpha blend onto a light canvas (no true WebGL opacity in node programs). */
function softenColor(hex: string, opacity: number): string {
  const m = /^#?([0-9a-f]{6})$/i.exec(hex.trim());
  if (!m) return hex;
  const n = parseInt(m[1]!, 16);
  const r = (n >> 16) & 255;
  const g = (n >> 8) & 255;
  const b = n & 255;
  const blend = (c: number) => Math.round(c * opacity + 250 * (1 - opacity));
  const toHex = (c: number) => c.toString(16).padStart(2, "0");
  return `#${toHex(blend(r))}${toHex(blend(g))}${toHex(blend(b))}`;
}
