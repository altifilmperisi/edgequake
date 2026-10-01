import { describe, expect, it } from "vitest";
import {
  FIXTURE_100,
  FIXTURE_EMPTY,
  FIXTURE_PARALLEL,
  FIXTURE_SINGLE,
  generateGraph,
  toKnowledgeGraphResponse,
} from "./index";

describe("SPEC-155 graph fixtures", () => {
  it("empty graph", () => {
    expect(FIXTURE_EMPTY.nodes).toHaveLength(0);
    expect(FIXTURE_EMPTY.total_nodes).toBe(0);
  });

  it("single node", () => {
    expect(FIXTURE_SINGLE.nodes).toHaveLength(1);
  });

  it("100 is deterministic", () => {
    const a = generateGraph(100, { seed: 155 });
    const b = generateGraph(100, { seed: 155 });
    expect(a.nodes[0]?.id).toBe(b.nodes[0]?.id);
    expect(a.edges.length).toBe(b.edges.length);
    expect(FIXTURE_100.nodes).toHaveLength(100);
  });

  it("truncation flag when capped", () => {
    const g = generateGraph(500, { maxNodes: 200 });
    expect(g.nodes).toHaveLength(200);
    expect(g.is_truncated).toBe(true);
    expect(g.total_nodes).toBe(500);
  });

  it("parallel edges produce multiple relation types", () => {
    const types = new Set(FIXTURE_PARALLEL.edges.map((e) => e.relationship_type));
    expect(types.has("WORKS_WITH")).toBe(true);
    expect(types.has("REPORTS_TO")).toBe(true);
  });

  it("toKnowledgeGraphResponse shape", () => {
    const kg = toKnowledgeGraphResponse(FIXTURE_100);
    expect(kg.nodes[0]).toHaveProperty("community_id");
    expect(kg.edges[0]).toHaveProperty("id");
    expect(kg).toHaveProperty("is_truncated");
  });
});
