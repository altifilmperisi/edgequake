import type { GraphEdge, GraphNode } from "@/types";
import type { GraphLayoutType } from "@/lib/graph/layouts";

export type GraphEngineId = string;

export type FocusMode =
  | "none"
  | "hover"
  | "select"
  | "ego"
  | "path"
  | "answer";

export interface FocusState {
  mode: FocusMode;
  ids: string[];
  depth?: number;
}

export interface GraphTimeRange {
  enabled: boolean;
  start: Date | null;
  end: Date | null;
}

export interface GraphFiltersState {
  types: Set<string>;
  relTypes: Set<string>;
  query: string;
  timeRange: GraphTimeRange;
  documentIds: string[];
}

export interface GraphDelta {
  upsertNodes?: GraphNode[];
  upsertEdges?: GraphEdge[];
  removeNodeIds?: string[];
  removeEdgeIds?: string[];
}

export type LodTier = 0 | 1 | 2;

export interface GraphThemeTokens {
  canvasBg: string;
  edge: string;
  labelFg: string;
  labelBg: string;
  focus: string;
  dim: string;
  hullStroke: string;
  hullFill: string;
  communities: string[];
}

export interface GraphEngineOptions {
  getNodeColor?: (entityType?: string) => string;
  isDark?: boolean;
  showLabels?: boolean;
  showEdgeLabels?: boolean;
  enableNodeDrag?: boolean;
  highlightNeighbors?: boolean;
  hideUnselectedEdges?: boolean;
  nodeSize?: number;
  layout?: GraphLayoutType;
  colorMode?: "entity-type" | "community";
  onNodeClick?: (nodeId: string) => void;
  /** Double-click on a node (default zoom-to-cursor is suppressed). */
  onNodeDoubleClick?: (nodeId: string) => void;
  /** Click on empty canvas (not a pan) — typically clears the selection. */
  onStageClick?: () => void;
  onNodeHover?: (nodeId: string | null) => void;
  onNodeRightClick?: (nodeId: string, x: number, y: number) => void;
  onWebglError?: (error: Error) => void;
}

export interface TruncationInput {
  streamedNodes: number;
  streamedEdges: number;
  totalNodes?: number;
  totalEdges?: number;
  isTruncated?: boolean;
}

export interface TruncationResult {
  isTruncated: boolean;
  totalNodes: number;
  totalEdges: number;
}
