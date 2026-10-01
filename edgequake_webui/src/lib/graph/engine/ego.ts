/**
 * Ego-network helpers: the set of nodes within N hops of a seed node.
 * Shared by the engine (dimming) and the ego-depth control (live counts).
 */
import type Graph from "graphology";

export const MIN_EGO_DEPTH = 1;
export const MAX_EGO_DEPTH = 3;

export function clampEgoDepth(depth: number | undefined): number {
  if (typeof depth !== "number" || !Number.isFinite(depth)) return MIN_EGO_DEPTH;
  return Math.max(MIN_EGO_DEPTH, Math.min(MAX_EGO_DEPTH, Math.round(depth)));
}

/**
 * Hop distance (1..depth) of every node reachable from `seed`, direction-agnostic.
 * The seed itself is excluded. Insertion order is by increasing distance.
 */
export function collectHopDistances(
  graph: Graph,
  seed: string,
  depth: number,
): Map<string, number> {
  const hops = new Map<string, number>();
  if (!graph.hasNode(seed)) return hops;

  let frontier = [seed];
  for (let hop = 1; hop <= clampEgoDepth(depth); hop++) {
    const next: string[] = [];
    for (const node of frontier) {
      graph.forEachNeighbor(node, (neighbor) => {
        if (neighbor === seed || hops.has(neighbor)) return;
        hops.set(neighbor, hop);
        next.push(neighbor);
      });
    }
    if (next.length === 0) break;
    frontier = next;
  }
  return hops;
}

/** BFS neighbours within `depth` hops (direction-agnostic, excludes the seed). */
export function collectNeighborsBfs(
  graph: Graph,
  seed: string,
  depth: number,
): Set<string> {
  return new Set(collectHopDistances(graph, seed, depth).keys());
}

/**
 * Brightness (0..1) of a node `hop` rings out: the first ring is fully bright,
 * outer rings fade so the depth is legible without extra chrome.
 */
export function hopFade(hop: number | undefined): number {
  if (!hop || hop <= 1) return 1;
  return Math.max(0.35, 1 - 0.3 * (hop - 1));
}

/** Cumulative neighbour counts for depth 1..maxDepth, e.g. `[3, 7, 12]`. */
export function countNeighborsByDepth(
  graph: Graph,
  seed: string,
  maxDepth: number = MAX_EGO_DEPTH,
): number[] {
  const counts: number[] = [];
  for (let d = MIN_EGO_DEPTH; d <= clampEgoDepth(maxDepth); d++) {
    counts.push(collectNeighborsBfs(graph, seed, d).size);
  }
  return counts;
}
