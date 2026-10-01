import type { GraphEdge, GraphNode } from "@/types";
import type { GraphFiltersState, GraphTimeRange } from "./types";

export function createEmptyFilters(
  allTypes: Iterable<string> = [],
  allRelTypes: Iterable<string> = [],
): GraphFiltersState {
  return {
    types: new Set(allTypes),
    relTypes: new Set(allRelTypes),
    query: "",
    timeRange: { enabled: false, start: null, end: null },
    documentIds: [],
  };
}

export function nodeMatchesTimeFilter(
  node: GraphNode,
  timeRange: GraphTimeRange,
): boolean {
  if (!timeRange.enabled) return true;
  const raw =
    node.created_at ??
    (typeof node.properties?.created_at === "string"
      ? node.properties.created_at
      : undefined);
  if (!raw) return true; // no timestamp → keep (fail-open)
  const nodeDate = new Date(raw);
  if (Number.isNaN(nodeDate.getTime())) return true;
  if (timeRange.start && nodeDate < timeRange.start) return false;
  if (timeRange.end && nodeDate > timeRange.end) return false;
  return true;
}

export function nodeMatchesSearch(node: GraphNode, query: string): boolean {
  const q = query.trim().toLowerCase();
  if (!q) return true;
  return (
    node.label.toLowerCase().includes(q) ||
    (node.description?.toLowerCase().includes(q) ?? false) ||
    node.id.toLowerCase().includes(q)
  );
}

export function nodeMatchesFilters(
  node: GraphNode,
  filters: GraphFiltersState,
): boolean {
  // Visibility set semantics: empty types ⇒ nothing visible (store behaviour)
  if (!filters.types.has(node.node_type)) return false;
  if (!nodeMatchesTimeFilter(node, filters.timeRange)) return false;
  if (!nodeMatchesSearch(node, filters.query)) return false;
  return true;
}

export function edgeMatchesFilters(
  edge: GraphEdge,
  filters: GraphFiltersState,
  visibleNodeIds: Set<string>,
): boolean {
  if (!filters.relTypes.has(edge.relationship_type)) return false;
  return visibleNodeIds.has(edge.source) && visibleNodeIds.has(edge.target);
}

export function filterGraphData(
  nodes: GraphNode[],
  edges: GraphEdge[],
  filters: GraphFiltersState,
): { nodes: GraphNode[]; edges: GraphEdge[]; nodeIds: Set<string> } {
  const filteredNodes = nodes.filter((n) => nodeMatchesFilters(n, filters));
  const nodeIds = new Set(filteredNodes.map((n) => n.id));
  const filteredEdges = edges.filter((e) =>
    edgeMatchesFilters(e, filters, nodeIds),
  );
  return { nodes: filteredNodes, edges: filteredEdges, nodeIds };
}
