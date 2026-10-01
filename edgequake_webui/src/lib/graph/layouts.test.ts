import Graph from "graphology";
import { describe, expect, it } from "vitest";

import {
  applyLayoutToGraph,
  calculateLayoutPositions,
  getGraphPerformanceProfile,
} from "./layouts";

function buildTestGraph(): Graph {
  const graph = new Graph();

  graph.addNode("a", { x: 0, y: 0, entityType: "PERSON" });
  graph.addNode("b", { x: 1, y: 0, entityType: "PERSON" });
  graph.addNode("c", { x: 0, y: 1, entityType: "ORG" });
  graph.addEdgeWithKey("a-b", "a", "b");
  graph.addEdgeWithKey("a-c", "a", "c");

  return graph;
}

describe("graph layouts", () => {
  it("computes performance thresholds consistently", () => {
    expect(getGraphPerformanceProfile(50, 80)).toMatchObject({
      isLargeGraph: false,
      isVeryLargeGraph: false,
      disableEdgeEvents: false,
      labelGridCellSize: 80,
    });

    expect(getGraphPerformanceProfile(250, 300)).toMatchObject({
      isLargeGraph: true,
      isVeryLargeGraph: false,
      disableEdgeEvents: false,
      labelGridCellSize: 120,
    });

    expect(getGraphPerformanceProfile(600, 1200)).toMatchObject({
      isLargeGraph: true,
      isVeryLargeGraph: true,
      disableEdgeEvents: true,
      labelGridCellSize: 160,
    });
  });

  it("returns positions for each node", () => {
    const graph = buildTestGraph();
    const positions = calculateLayoutPositions(graph, "force", "interactive");

    expect(Object.keys(positions).sort()).toEqual(["a", "b", "c"]);
    expect(Number.isFinite(positions.a.x)).toBe(true);
    expect(Number.isFinite(positions.a.y)).toBe(true);
  });

  it("applies hierarchical layout by grouping entity types into levels", () => {
    const graph = buildTestGraph();
    applyLayoutToGraph(graph, "hierarchical");

    expect(graph.getNodeAttribute("a", "y")).toBe(graph.getNodeAttribute("b", "y"));
    expect(graph.getNodeAttribute("a", "y")).not.toBe(graph.getNodeAttribute("c", "y"));
  });

  it("spreads a force layout so node discs do not pile up", () => {
    const graph = new Graph();
    const n = 80;
    for (let i = 0; i < n; i++) {
      const a = (2 * Math.PI * i) / n;
      graph.addNode(`n${i}`, { x: Math.cos(a) * 100, y: Math.sin(a) * 100, size: 10 });
    }
    for (let i = 1; i < n; i++) graph.addEdgeWithKey(`h${i}`, "n0", `n${i}`);
    for (let i = 2; i < n; i += 2) graph.addEdgeWithKey(`c${i}`, `n${i}`, `n${i - 1}`);

    applyLayoutToGraph(graph, "force", "initial");

    const pts: { x: number; y: number }[] = [];
    graph.forEachNode((_, a) => pts.push({ x: a.x, y: a.y }));
    const xs = pts.map((p) => p.x);
    const ys = pts.map((p) => p.y);
    const extent = Math.max(Math.max(...xs) - Math.min(...xs), Math.max(...ys) - Math.min(...ys));
    expect(extent).toBeGreaterThanOrEqual(500);

    // On a 700px canvas a node (radius 10px) must not overlap its neighbours.
    const scale = 700 / extent;
    let overlaps = 0;
    for (let i = 0; i < pts.length; i++) {
      for (let j = i + 1; j < pts.length; j++) {
        if (Math.hypot(pts[i].x - pts[j].x, pts[i].y - pts[j].y) * scale < 18) overlaps++;
      }
    }
    expect(overlaps).toBeLessThanOrEqual(3);
  });
});
