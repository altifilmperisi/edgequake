/**
 * SPEC-157 — Answer subgraph: SSOT for turning a chat answer's retrieval
 * context into (a) Graph Studio focus ids and (b) a self-contained graph the
 * Query companion pane can render without touching the workspace graph store
 * (LAW-157-8/9/10).
 *
 * Pure: no React, no stores, no network.
 */
import type { QueryContext } from "@/types/query";
import type { GraphEdge, GraphNode } from "@/types/graph";
import type {
  ContextEntityApi,
  SubgraphBundle,
} from "@/lib/utils/subgraph-types";

/** Hard ceiling for the embedded view; the rest is reachable in Studio. */
export const ANSWER_GRAPH_MAX_NODES = 60;
/** Extra neighbours added per explicit "Expand" action. */
export const ANSWER_GRAPH_EXPAND_CAP = 25;

type ContextForGraph = Pick<QueryContext, "entities" | "relationships"> & {
  subgraph?: SubgraphBundle;
};

// --- Subgraph from a message ------------------------------------------------

/**
 * The subgraph behind an answer. Prefer the server bundle; fall back to the
 * flat `entities`/`relationships` lists persisted on older messages.
 */
export function subgraphFromContext(
  context: ContextForGraph | null | undefined,
): SubgraphBundle {
  if (context?.subgraph) return context.subgraph;
  return {
    entities: (context?.entities ?? []).map((e) => ({
      id: e.id,
      name: e.label,
      entity_type: e.entity_type ?? "OTHER",
      description: "",
      score: e.relevance ?? 0,
      degree: e.degree ?? 0,
      graph_node_id: e.id,
    })),
    relationships: (context?.relationships ?? []).map((r) => ({
      id: `${r.source}-${r.target}`,
      source: r.source,
      target: r.target,
      source_label: r.source_label,
      target_label: r.target_label,
      relation_type: r.type,
      description: "",
      score: r.relevance ?? 0,
    })),
  };
}

export function hasAnswerGraph(
  context: ContextForGraph | null | undefined,
): boolean {
  return (
    (context?.entities?.length ?? 0) > 0 ||
    (context?.subgraph?.entities?.length ?? 0) > 0
  );
}

// --- Graph Studio focus -----------------------------------------------------

/** Map API subgraph entities → graph node ids + name fallbacks. */
export function mapSubgraphToAnswerFocus(subgraph: SubgraphBundle): {
  nodeIds: string[];
  entityNames: string[];
} {
  const nodeIds: string[] = [];
  const entityNames: string[] = [];
  for (const e of subgraph.entities ?? []) {
    const graphId = e.graph_node_id || e.node_id || "";
    if (graphId) nodeIds.push(graphId);
    if (e.name) entityNames.push(e.name);
    if (e.id && !graphId) entityNames.push(e.id);
  }
  return {
    nodeIds: [...new Set(nodeIds)],
    entityNames: [...new Set(entityNames)],
  };
}

// --- Embedded graph model ---------------------------------------------------

export type AnswerGraphModel = {
  nodes: GraphNode[];
  edges: GraphEdge[];
  /** Ids of the entities the answer actually used (vs. context neighbours). */
  answerNodeIds: string[];
  /** Entities dropped by the display cap — reachable in Graph Studio. */
  hiddenCount: number;
};

/** `Foo Bar` / `foo-bar` / `FOO_BAR` → `FOO_BAR` (entity-name SSOT shape). */
export function normalizeEntityKey(value: string): string {
  return value
    .trim()
    .replace(/[\s-]+/g, "_")
    .toUpperCase();
}

function stripPrefix(value: string): string {
  const afterScope = value.includes("::")
    ? value.slice(value.lastIndexOf("::") + 2)
    : value;
  return afterScope.replace(/^ent:/i, "");
}

function aliasesOf(value: string | undefined): string[] {
  if (!value) return [];
  const bare = stripPrefix(value);
  return [value, bare, normalizeEntityKey(bare)];
}

function entityAliases(e: ContextEntityApi): string[] {
  return [
    ...aliasesOf(e.graph_node_id),
    ...aliasesOf(e.node_id),
    ...aliasesOf(e.id),
    ...aliasesOf(e.name),
  ];
}

function nodeIdOf(e: ContextEntityApi): string {
  return (
    e.graph_node_id || e.node_id || e.id || normalizeEntityKey(e.name || "")
  );
}

function toNode(e: ContextEntityApi): GraphNode {
  return {
    id: nodeIdOf(e),
    label: e.name || stripPrefix(nodeIdOf(e)),
    node_type: e.entity_type || "OTHER",
    description: e.description || undefined,
    degree: e.degree ?? 0,
  };
}

function placeholderNode(id: string, label?: string): GraphNode {
  return {
    id,
    label: label || stripPrefix(id),
    node_type: "UNKNOWN",
    degree: 0,
  };
}

/**
 * Build the embedded graph: the entities the answer used (top-N by score),
 * the relationships among them, and context-only neighbours for dangling
 * relationship endpoints (honest: typed UNKNOWN, not in `answerNodeIds`).
 */
export function buildAnswerGraphModel(
  subgraph: SubgraphBundle,
  maxNodes: number = ANSWER_GRAPH_MAX_NODES,
): AnswerGraphModel {
  const ranked = [...(subgraph.entities ?? [])]
    .filter((e) => e.name || e.id || e.graph_node_id || e.node_id)
    .sort((a, b) => (b.score ?? 0) - (a.score ?? 0));

  const alias = new Map<string, string>();
  const nodes = new Map<string, GraphNode>();
  const kept = ranked.slice(0, Math.max(0, maxNodes));

  for (const e of kept) {
    const node = toNode(e);
    if (!node.id) continue;
    const existing = entityAliases(e)
      .map((a) => alias.get(a))
      .find(Boolean);
    const id = existing ?? node.id;
    if (!existing) nodes.set(id, node);
    for (const a of entityAliases(e)) if (!alias.has(a)) alias.set(a, id);
  }
  const answerNodeIds = [...nodes.keys()];

  const resolve = (endpoint: string, label?: string): string | null => {
    const hit = aliasesOf(endpoint)
      .map((a) => alias.get(a))
      .find(Boolean);
    if (hit) return hit;
    if (!endpoint.trim()) return null;
    if (nodes.size >= maxNodes + ANSWER_GRAPH_EXPAND_CAP) return null;
    const id = endpoint;
    nodes.set(id, placeholderNode(id, label));
    for (const a of aliasesOf(endpoint)) alias.set(a, id);
    return id;
  };

  const edges = new Map<string, GraphEdge>();
  for (const r of subgraph.relationships ?? []) {
    const source = resolve(r.source, r.source_label);
    const target = resolve(r.target, r.target_label);
    if (!source || !target || source === target) continue;
    const key = `${source}\u0000${target}\u0000${r.relation_type}`;
    if (edges.has(key)) continue;
    edges.set(key, {
      id: r.id || key,
      source,
      target,
      relationship_type: r.relation_type || "RELATED_TO",
      weight: r.score && r.score > 0 ? r.score : 1,
      description: r.description || undefined,
      source_ids: [],
      created_at: "",
    });
  }

  return {
    nodes: [...nodes.values()],
    edges: [...edges.values()],
    answerNodeIds,
    hiddenCount: Math.max(0, ranked.length - kept.length),
  };
}

/** The neighbourhood API says `entity_type`; the graph model says `node_type`. */
function normalizeApiNode(
  n: GraphNode & { entity_type?: string },
): GraphNode {
  return {
    ...n,
    label: n.label || stripPrefix(n.id),
    node_type: n.node_type || n.entity_type || "UNKNOWN",
  };
}

/**
 * Merge a neighbourhood fetched for one node into the model, capped so a
 * hub entity can never flood the pane (EC-157-19). Pure + idempotent.
 */
export function mergeNeighborhood(
  model: AnswerGraphModel,
  incoming: { nodes: GraphNode[]; edges: GraphEdge[] },
  cap: number = ANSWER_GRAPH_EXPAND_CAP,
): AnswerGraphModel {
  const known = new Set(model.nodes.map((n) => n.id));
  const fresh = incoming.nodes.filter((n) => n.id && !known.has(n.id));
  const accepted = fresh.slice(0, Math.max(0, cap)).map(normalizeApiNode);
  const all = new Set([...known, ...accepted.map((n) => n.id)]);

  const edgeKeys = new Set(
    model.edges.map((e) => `${e.source}\u0000${e.target}\u0000${e.relationship_type}`),
  );
  const newEdges = incoming.edges.filter((e) => {
    const key = `${e.source}\u0000${e.target}\u0000${e.relationship_type}`;
    if (!all.has(e.source) || !all.has(e.target) || edgeKeys.has(key)) {
      return false;
    }
    edgeKeys.add(key);
    return true;
  });

  if (accepted.length === 0 && newEdges.length === 0) return model;
  return {
    ...model,
    nodes: [...model.nodes, ...accepted],
    edges: [...model.edges, ...newEdges],
  };
}
