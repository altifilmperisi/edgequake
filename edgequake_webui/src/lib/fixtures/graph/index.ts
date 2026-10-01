/**
 * Deterministic graph fixtures for SPEC-155 (vitest + Playwright).
 * Seeds are fixed so layout and truncation assertions stay stable.
 */

export type FixtureNode = {
  id: string;
  label: string;
  node_type: string;
  description?: string;
  degree: number;
  community_id?: string;
  created_at?: string;
  properties?: Record<string, unknown>;
};

export type FixtureEdge = {
  id: string;
  source: string;
  target: string;
  relationship_type: string;
  weight: number;
  description?: string;
  keywords?: string[];
};

export type FixtureGraph = {
  nodes: FixtureNode[];
  edges: FixtureEdge[];
  is_truncated: boolean;
  total_nodes: number;
  total_edges: number;
  max_nodes: number;
};

const TYPES = [
  "PERSON",
  "ORGANIZATION",
  "LOCATION",
  "CONCEPT",
  "EVENT",
  "METHOD",
] as const;

function seededRandom(seed: number): () => number {
  let s = seed >>> 0;
  return () => {
    s = (s * 1664525 + 1013904223) >>> 0;
    return s / 0x100000000;
  };
}

export function generateGraph(
  nodeCount: number,
  options: {
    seed?: number;
    maxNodes?: number;
    hub?: boolean;
    parallelEdges?: boolean;
    communities?: number;
  } = {},
): FixtureGraph {
  const seed = options.seed ?? 155;
  const rand = seededRandom(seed);
  const maxNodes = options.maxNodes ?? nodeCount;
  const communityCount = options.communities ?? Math.max(1, Math.min(8, Math.ceil(nodeCount / 25)));

  const nodes: FixtureNode[] = [];
  for (let i = 0; i < nodeCount; i++) {
    const type = TYPES[i % TYPES.length]!;
    nodes.push({
      id: `ws-e2e::NODE_${i}`,
      label: `Node ${i}`,
      node_type: type,
      description: `Fixture entity ${i} (${type})`,
      degree: 0,
      community_id: `c${i % communityCount}`,
      created_at: `2026-01-${String((i % 28) + 1).padStart(2, "0")}T12:00:00Z`,
      properties: { entity_type: type },
    });
  }

  const edges: FixtureEdge[] = [];
  const addEdge = (source: string, target: string, rel: string, idx: number) => {
    edges.push({
      id: `${source}|${rel}|${target}|${idx}`,
      source,
      target,
      relationship_type: rel,
      weight: 1 + (idx % 5),
      description: `${rel} link`,
      keywords: [rel.toLowerCase()],
    });
  };

  if (options.hub && nodeCount > 1) {
    const hub = nodes[0]!;
    for (let i = 1; i < nodeCount; i++) {
      addEdge(hub.id, nodes[i]!.id, "RELATED_TO", i);
    }
  } else {
    for (let i = 0; i < nodeCount; i++) {
      const target = nodes[(i + 1) % nodeCount]!;
      addEdge(nodes[i]!.id, target.id, "RELATED_TO", i);
      if (nodeCount > 3 && rand() > 0.6) {
        const j = Math.floor(rand() * nodeCount);
        if (j !== i) addEdge(nodes[i]!.id, nodes[j]!.id, "MENTIONS", i * 1000 + j);
      }
    }
  }

  if (options.parallelEdges && nodeCount >= 2) {
    addEdge(nodes[0]!.id, nodes[1]!.id, "WORKS_WITH", 900001);
    addEdge(nodes[0]!.id, nodes[1]!.id, "REPORTS_TO", 900002);
  }

  const degreeMap = new Map<string, number>();
  for (const e of edges) {
    degreeMap.set(e.source, (degreeMap.get(e.source) ?? 0) + 1);
    degreeMap.set(e.target, (degreeMap.get(e.target) ?? 0) + 1);
  }
  for (const n of nodes) {
    n.degree = degreeMap.get(n.id) ?? 0;
  }

  const capped = nodes.slice(0, maxNodes);
  const idSet = new Set(capped.map((n) => n.id));
  const cappedEdges = edges.filter((e) => idSet.has(e.source) && idSet.has(e.target));

  return {
    nodes: capped,
    edges: cappedEdges,
    is_truncated: nodeCount > maxNodes || capped.length < nodeCount,
    total_nodes: nodeCount,
    total_edges: edges.length,
    max_nodes: maxNodes,
  };
}

export const FIXTURE_EMPTY = generateGraph(0);
export const FIXTURE_SINGLE = generateGraph(1);
export const FIXTURE_100 = generateGraph(100, { maxNodes: 100 });
export const FIXTURE_500 = generateGraph(500, { maxNodes: 200 });
export const FIXTURE_2K = generateGraph(2000, { maxNodes: 500 });
export const FIXTURE_HUB = generateGraph(200, { hub: true, maxNodes: 200 });
export const FIXTURE_PARALLEL = generateGraph(10, { parallelEdges: true });

export function toKnowledgeGraphResponse(g: FixtureGraph) {
  return {
    nodes: g.nodes.map((n) => ({
      id: n.id,
      label: n.label,
      node_type: n.node_type,
      description: n.description,
      degree: n.degree,
      community_id: n.community_id,
      properties: {
        ...n.properties,
        community_id: n.community_id,
        created_at: n.created_at,
      },
    })),
    edges: g.edges.map((e) => ({
      id: e.id,
      source: e.source,
      target: e.target,
      relationship_type: e.relationship_type,
      weight: e.weight,
      description: e.description,
      keywords: e.keywords,
      properties: {},
    })),
    is_truncated: g.is_truncated,
    total_nodes: g.total_nodes,
    total_edges: g.total_edges,
    max_nodes: g.max_nodes,
  };
}
