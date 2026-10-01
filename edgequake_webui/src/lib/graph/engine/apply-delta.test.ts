import { describe, expect, it } from "vitest";
import { MultiGraph } from "graphology";
import type { GraphEdge, GraphNode } from "@/types";
import {
  applyDelta,
  diffToDelta,
  resolveEdgeKey,
} from "./apply-delta";

const ctx = {
  borderColor: "#fff",
  edgeColor: "#999",
  nodeSize: 10,
  showEdgeLabels: false,
  getNodeColor: () => "#64748b",
};

function node(id: string, label = id): GraphNode {
  return { id, label, node_type: "PERSON", degree: 1 };
}

function edge(
  id: string,
  source: string,
  target: string,
  relationship_type = "RELATED_TO",
): GraphEdge {
  return {
    id,
    source,
    target,
    relationship_type,
    weight: 1,
    source_ids: [],
    created_at: "2026-01-01T00:00:00Z",
  };
}

describe("applyDelta", () => {
  it("upserts nodes and edges on a MultiGraph without dropping parallels", () => {
    const graph = new MultiGraph({ type: "directed" });
    applyDelta(
      graph,
      {
        upsertNodes: [node("a"), node("b")],
        upsertEdges: [
          edge("e1", "a", "b", "WORKS_WITH"),
          edge("e2", "a", "b", "REPORTS_TO"),
        ],
      },
      ctx,
    );
    expect(graph.order).toBe(2);
    expect(graph.size).toBe(2);
    expect(graph.hasEdge("e1")).toBe(true);
    expect(graph.hasEdge("e2")).toBe(true);
  });

  it("preserves positions when upserting existing nodes", () => {
    const graph = new MultiGraph({ type: "directed" });
    applyDelta(graph, { upsertNodes: [node("a")] }, ctx);
    graph.setNodeAttribute("a", "x", 42);
    graph.setNodeAttribute("a", "y", 7);
    applyDelta(graph, { upsertNodes: [node("a", "A2")] }, ctx);
    expect(graph.getNodeAttribute("a", "x")).toBe(42);
    expect(graph.getNodeAttribute("a", "y")).toBe(7);
    expect(graph.getNodeAttribute("a", "label")).toBe("A2");
  });

  it("removes nodes and edges by id", () => {
    const graph = new MultiGraph({ type: "directed" });
    applyDelta(
      graph,
      {
        upsertNodes: [node("a"), node("b")],
        upsertEdges: [edge("e1", "a", "b")],
      },
      ctx,
    );
    applyDelta(
      graph,
      { removeEdgeIds: ["e1"], removeNodeIds: ["b"] },
      ctx,
    );
    expect(graph.hasEdge("e1")).toBe(false);
    expect(graph.hasNode("b")).toBe(false);
    expect(graph.hasNode("a")).toBe(true);
  });

  it("diffToDelta removes stale ids and upserts desired set", () => {
    const graph = new MultiGraph({ type: "directed" });
    applyDelta(
      graph,
      {
        upsertNodes: [node("a"), node("b")],
        upsertEdges: [edge("e1", "a", "b")],
      },
      ctx,
    );
    const delta = diffToDelta(graph, [node("a")], []);
    expect(delta.removeNodeIds).toContain("b");
    expect(delta.removeEdgeIds).toContain("e1");
    expect(delta.upsertNodes).toHaveLength(1);
  });
});

describe("resolveEdgeKey", () => {
  it("prefers explicit edge id", () => {
    expect(
      resolveEdgeKey({
        id: "custom",
        source: "a",
        target: "b",
        relationship_type: "X",
      }),
    ).toBe("custom");
  });
});

describe("edge programs", () => {
  it("draws lone edges straight and bends only parallel / reverse edges", () => {
    const graph = new MultiGraph({ type: "directed" });
    applyDelta(
      graph,
      {
        upsertNodes: [node("a"), node("b"), node("c")],
        upsertEdges: [
          edge("e1", "a", "b"),
          edge("e2", "b", "a", "REVERSE"),
          edge("e3", "a", "c"),
        ],
      },
      ctx,
    );

    // @sigma/edge-curve renders nothing at curvature 0 → lone edges use "arrow".
    expect(graph.getEdgeAttribute("e3", "type")).toBe("arrow");
    expect(graph.hasEdgeAttribute("e3", "curvature")).toBe(false);
    expect(graph.getEdgeAttribute("e1", "type")).toBe("curvedArrow");
    expect(graph.getEdgeAttribute("e2", "type")).toBe("curvedArrow");
    expect(graph.getEdgeAttribute("e1", "curvature")).not.toBe(
      graph.getEdgeAttribute("e2", "curvature"),
    );
  });

  it("marks new nodes as unplaced until a layout pass runs", () => {
    const graph = new MultiGraph();
    applyDelta(graph, { upsertNodes: [node("a")] }, ctx);
    expect(graph.getNodeAttribute("a", "placed")).toBe(false);
  });
});
