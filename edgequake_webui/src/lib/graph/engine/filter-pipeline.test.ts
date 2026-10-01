import { describe, expect, it } from "vitest";
import type { GraphEdge, GraphNode } from "@/types";
import {
  createEmptyFilters,
  filterGraphData,
  nodeMatchesFilters,
  nodeMatchesTimeFilter,
} from "./filter-pipeline";

const nodes: GraphNode[] = [
  {
    id: "1",
    label: "Alice",
    node_type: "PERSON",
    created_at: "2026-01-10T00:00:00Z",
  },
  {
    id: "2",
    label: "Acme",
    node_type: "ORGANIZATION",
    created_at: "2026-02-10T00:00:00Z",
    description: "Corp",
  },
  {
    id: "3",
    label: "Bob",
    node_type: "PERSON",
    properties: { created_at: "2026-01-15T00:00:00Z" },
  },
];

const edges: GraphEdge[] = [
  {
    id: "e1",
    source: "1",
    target: "2",
    relationship_type: "WORKS_WITH",
    weight: 1,
    source_ids: [],
    created_at: "2026-01-01T00:00:00Z",
  },
  {
    id: "e2",
    source: "1",
    target: "3",
    relationship_type: "KNOWS",
    weight: 1,
    source_ids: [],
    created_at: "2026-01-01T00:00:00Z",
  },
];

describe("filter pipeline", () => {
  it("filters by entity type", () => {
    const filters = createEmptyFilters(["PERSON"], ["WORKS_WITH", "KNOWS"]);
    filters.types = new Set(["PERSON"]);
    const { nodes: out } = filterGraphData(nodes, edges, filters);
    expect(out.map((n) => n.id)).toEqual(["1", "3"]);
  });

  it("filters by search query", () => {
    const filters = createEmptyFilters(
      ["PERSON", "ORGANIZATION"],
      ["WORKS_WITH", "KNOWS"],
    );
    filters.query = "acme";
    expect(nodeMatchesFilters(nodes[1]!, filters)).toBe(true);
    expect(nodeMatchesFilters(nodes[0]!, filters)).toBe(false);
  });

  it("applies time range (G06) including properties.created_at", () => {
    const filters = createEmptyFilters(
      ["PERSON", "ORGANIZATION"],
      ["WORKS_WITH", "KNOWS"],
    );
    filters.timeRange = {
      enabled: true,
      start: new Date("2026-01-12T00:00:00Z"),
      end: new Date("2026-01-20T00:00:00Z"),
    };
    expect(nodeMatchesTimeFilter(nodes[0]!, filters.timeRange)).toBe(false);
    expect(nodeMatchesTimeFilter(nodes[2]!, filters.timeRange)).toBe(true);
    const { nodes: out, edges: eOut } = filterGraphData(nodes, edges, filters);
    expect(out.map((n) => n.id)).toEqual(["3"]);
    expect(eOut).toHaveLength(0);
  });

  it("filters edges by relationship type and visible endpoints", () => {
    const filters = createEmptyFilters(
      ["PERSON", "ORGANIZATION"],
      ["WORKS_WITH"],
    );
    filters.relTypes = new Set(["WORKS_WITH"]);
    const { edges: out } = filterGraphData(nodes, edges, filters);
    expect(out.map((e) => e.id)).toEqual(["e1"]);
  });
});
