import { MultiGraph } from "graphology";
import { describe, expect, it } from "vitest";
import {
  clampEgoDepth,
  collectNeighborsBfs,
  countNeighborsByDepth,
} from "./ego";
import { isEdgeDimmed, isNodeDimmed, type FocusContext } from "./focus-strategies";
import { createEmptyFilters } from "./filter-pipeline";

// a - b - c - d - e   (chain) plus a - x
function chain(): MultiGraph {
  const g = new MultiGraph();
  ["a", "b", "c", "d", "e", "x"].forEach((id) => g.addNode(id));
  [["a", "b"], ["b", "c"], ["c", "d"], ["d", "e"], ["a", "x"]].forEach(([s, t], i) =>
    g.addEdgeWithKey(`e${i}`, s, t),
  );
  return g;
}

describe("collectNeighborsBfs", () => {
  it("returns nodes within N hops and never the seed", () => {
    const g = chain();
    expect([...collectNeighborsBfs(g, "a", 1)].sort()).toEqual(["b", "x"]);
    expect([...collectNeighborsBfs(g, "a", 2)].sort()).toEqual(["b", "c", "x"]);
    expect([...collectNeighborsBfs(g, "a", 3)].sort()).toEqual(["b", "c", "d", "x"]);
  });

  it("ignores edge direction and survives cycles / unknown seeds", () => {
    const g = new MultiGraph({ type: "directed" });
    ["a", "b", "c"].forEach((id) => g.addNode(id));
    g.addEdge("b", "a");
    g.addEdge("a", "c");
    g.addEdge("c", "b");
    expect([...collectNeighborsBfs(g, "a", 3)].sort()).toEqual(["b", "c"]);
    expect(collectNeighborsBfs(g, "nope", 2).size).toBe(0);
  });

  it("clamps depth to 1..3", () => {
    expect(clampEgoDepth(0)).toBe(1);
    expect(clampEgoDepth(9)).toBe(3);
    expect(clampEgoDepth(Number.NaN)).toBe(1);
    expect(clampEgoDepth(undefined)).toBe(1);
    expect(collectNeighborsBfs(chain(), "a", 99).size).toBe(4);
  });

  it("counts cumulatively per depth", () => {
    expect(countNeighborsByDepth(chain(), "a")).toEqual([2, 3, 4]);
  });
});

describe("select focus honours depth", () => {
  const all = new Set(["a", "b", "c", "d", "e", "x"]);
  const ctx = (depth: number): FocusContext => {
    const g = chain();
    return {
      focus: { mode: "select", ids: ["a"] },
      filters: createEmptyFilters(),
      matchingNodeIds: all,
      neighborIds: collectNeighborsBfs(g, "a", depth),
      highlightNeighbors: true,
      focusDepth: depth,
    };
  };

  it("depth 1: only direct neighbours bright, edges must touch the seed", () => {
    const c = ctx(1);
    expect(isNodeDimmed("b", c)).toBe(false);
    expect(isNodeDimmed("c", c)).toBe(true);
    expect(isEdgeDimmed("e0", "a", "b", c)).toBe(false);
    expect(isEdgeDimmed("e1", "b", "c", c)).toBe(true);
  });

  it("depth 2: second ring bright and the edges inside the neighbourhood lit", () => {
    const c = ctx(2);
    expect(isNodeDimmed("c", c)).toBe(false);
    expect(isNodeDimmed("d", c)).toBe(true);
    expect(isEdgeDimmed("e1", "b", "c", c)).toBe(false);
    expect(isEdgeDimmed("e2", "c", "d", c)).toBe(true);
  });
});
