import { MultiGraph } from "graphology";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { GraphLayoutType } from "@/lib/graph/layouts";
import { applyDelta } from "./apply-delta";
import {
  LayoutScheduler,
  PLACED_ATTR,
  seedUnplacedNodes,
} from "./layout-scheduler";

const CTX = {
  borderColor: "#fff",
  edgeColor: "#999",
  nodeSize: 10,
  showEdgeLabels: false,
  getNodeColor: () => "#123456",
};

function node(id: string) {
  return { id, label: id, node_type: "CONCEPT" };
}

function edge(source: string, target: string) {
  return { id: `${source}->${target}`, source, target, relationship_type: "REL" };
}

function setup(layout: GraphLayoutType = "force") {
  const graph = new MultiGraph({ type: "directed" });
  const onApplied = vi.fn();
  const scheduler = new LayoutScheduler({
    graph,
    getLayout: () => layout,
    onApplied,
    debounceMs: 100,
  });
  const add = (nodes: string[], edges: [string, string][] = []) => {
    const before = graph.order;
    const result = applyDelta(
      graph,
      {
        upsertNodes: nodes.map(node),
        upsertEdges: edges.map(([s, t]) => edge(s, t)),
      },
      CTX,
    );
    scheduler.notify(result, before);
  };
  return { graph, scheduler, onApplied, add };
}

function ids(prefix: string, n: number): string[] {
  return Array.from({ length: n }, (_, i) => `${prefix}${i}`);
}

describe("LayoutScheduler", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it("lays out the first batch synchronously and marks nodes placed", () => {
    const { graph, onApplied, add } = setup("force");
    add(ids("a", 12), ids("a", 12).slice(1).map((id) => ["a0", id]));

    expect(onApplied).toHaveBeenCalledTimes(1);
    graph.forEachNode((_, attrs) => expect(attrs[PLACED_ATTR]).toBe(true));
  });

  it("applies the selected layout (not the provisional ring) to streamed batches", () => {
    const { graph, onApplied, add } = setup("circular");
    add(ids("a", 4));
    add(ids("b", 4));

    // Not yet — batch is debounced.
    expect(onApplied).toHaveBeenCalledTimes(1);
    expect(graph.getNodeAttribute("b0", PLACED_ATTR)).toBe(false);

    vi.advanceTimersByTime(100);
    expect(onApplied).toHaveBeenCalledTimes(2);

    // graphology circular layout: every node on one circle (same radius).
    const radii = new Set<number>();
    graph.forEachNode((_, a) => radii.add(Math.round(Math.hypot(a.x, a.y) * 1000)));
    expect(radii.size).toBe(1);
    graph.forEachNode((_, a) => expect(a[PLACED_ATTR]).toBe(true));
  });

  it("coalesces rapid batches into one pass", () => {
    const { onApplied, add } = setup("circular");
    add(ids("a", 3));
    add(ids("b", 3));
    vi.advanceTimersByTime(50);
    add(ids("c", 3));
    vi.advanceTimersByTime(99);
    expect(onApplied).toHaveBeenCalledTimes(1);
    vi.advanceTimersByTime(1);
    expect(onApplied).toHaveBeenCalledTimes(2);
  });

  it("re-runs force layouts when only edges arrive (stream: nodes then edges)", () => {
    const { onApplied, add } = setup("force");
    add(ids("a", 8));
    expect(onApplied).toHaveBeenCalledTimes(1);

    add([], [["a0", "a1"], ["a0", "a2"]]);
    vi.advanceTimersByTime(100);
    expect(onApplied).toHaveBeenCalledTimes(2);
  });

  it("ignores edge-only changes for edge-independent layouts", () => {
    const { onApplied, add } = setup("circular");
    add(ids("a", 8));
    add([], [["a0", "a1"]]);
    vi.advanceTimersByTime(500);
    expect(onApplied).toHaveBeenCalledTimes(1);
  });

  it("cancel() drops pending work", () => {
    const { scheduler, onApplied, add } = setup("circular");
    add(ids("a", 3));
    add(ids("b", 3));
    scheduler.cancel();
    vi.advanceTimersByTime(500);
    expect(onApplied).toHaveBeenCalledTimes(1);
  });

  it("does nothing for empty / no-op deltas", () => {
    const { onApplied, add } = setup("force");
    add([]);
    vi.advanceTimersByTime(500);
    expect(onApplied).not.toHaveBeenCalled();
  });
});

describe("seedUnplacedNodes", () => {
  it("moves new nodes next to their placed neighbours", () => {
    const graph = new MultiGraph();
    graph.addNode("old", { x: 500, y: -300, [PLACED_ATTR]: true });
    graph.addNode("new", { x: 0, y: 0, [PLACED_ATTR]: false });
    graph.addEdgeWithKey("e", "old", "new");

    expect(seedUnplacedNodes(graph)).toBe(1);
    const { x, y } = graph.getNodeAttributes("new");
    expect(Math.hypot(x - 500, y + 300)).toBeLessThan(40);
  });

  it("leaves nodes without the placed flag and isolated nodes alone", () => {
    const graph = new MultiGraph();
    graph.addNode("legacy", { x: 1, y: 2 });
    graph.addNode("lonely", { x: 3, y: 4, [PLACED_ATTR]: false });
    graph.addNode("old", { x: 9, y: 9, [PLACED_ATTR]: true });
    graph.addEdgeWithKey("e", "old", "legacy");

    expect(seedUnplacedNodes(graph)).toBe(0);
    expect(graph.getNodeAttribute("legacy", "x")).toBe(1);
    expect(graph.getNodeAttribute("lonely", "x")).toBe(3);
  });
});
