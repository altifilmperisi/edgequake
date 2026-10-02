import { describe, expect, it } from "vitest";
import type { SubgraphBundle } from "@/lib/utils/subgraph-types";
import {
  ANSWER_GRAPH_EXPAND_CAP,
  buildAnswerGraphModel,
  hasAnswerGraph,
  mapSubgraphToAnswerFocus,
  mergeNeighborhood,
  normalizeEntityKey,
  subgraphFromContext,
} from "../answer-graph";

const ent = (
  name: string,
  extra: Partial<SubgraphBundle["entities"][number]> = {},
) => ({
  id: `ent:${normalizeEntityKey(name)}`,
  name,
  entity_type: "ORGANIZATION",
  description: "",
  score: 1,
  degree: 1,
  ...extra,
});

const rel = (
  source: string,
  target: string,
  extra: Partial<SubgraphBundle["relationships"][number]> = {},
) => ({
  id: `${source}-${target}`,
  source,
  target,
  relation_type: "USES",
  description: "",
  score: 0.5,
  ...extra,
});

describe("answer-graph model (LAW-157-10)", () => {
  it("resolves relationship endpoints by id, name or normalised name", () => {
    const m = buildAnswerGraphModel({
      entities: [ent("Light Rag", { graph_node_id: "n1" }), ent("Neo4j")],
      relationships: [
        rel("n1", "ent:NEO4J"),
        rel("LIGHT_RAG", "Neo4j", { relation_type: "CITES" }),
      ],
    });
    expect(m.nodes.map((n) => n.id).sort()).toEqual(["ent:NEO4J", "n1"]);
    expect(m.edges).toHaveLength(2);
    expect(m.answerNodeIds).toHaveLength(2);
  });

  it("works with name-only entities from a reloaded conversation (EC-157-18)", () => {
    const m = buildAnswerGraphModel({
      entities: [
        { id: "", name: "Alpha", entity_type: "X", description: "", score: 1, degree: 0 },
        { id: "", name: "Beta", entity_type: "X", description: "", score: 1, degree: 0 },
      ],
      relationships: [rel("ALPHA", "BETA")],
    });
    expect(m.nodes).toHaveLength(2);
    expect(m.edges).toHaveLength(1);
    expect(m.edges[0].source).toBe("ALPHA");
  });

  it("adds typed-UNKNOWN placeholders for dangling endpoints, not as answer nodes", () => {
    const m = buildAnswerGraphModel({
      entities: [ent("Alpha")],
      relationships: [rel("ent:ALPHA", "Gamma", { target_label: "Gamma Corp" })],
    });
    const gamma = m.nodes.find((n) => n.id === "Gamma");
    expect(gamma).toMatchObject({ node_type: "UNKNOWN", label: "Gamma Corp" });
    expect(m.answerNodeIds).toEqual(["ent:ALPHA"]);
  });

  it("drops self-loops, duplicates and empty endpoints", () => {
    const m = buildAnswerGraphModel({
      entities: [ent("A"), ent("B")],
      relationships: [
        rel("A", "A"),
        rel("A", "B"),
        rel("A", "B"),
        rel("", "B"),
      ],
    });
    expect(m.edges).toHaveLength(1);
  });

  it("caps entities by score and reports the hidden count", () => {
    const entities = Array.from({ length: 10 }, (_, i) =>
      ent(`E${i}`, { score: i }),
    );
    const m = buildAnswerGraphModel({ entities, relationships: [] }, 4);
    expect(m.nodes).toHaveLength(4);
    expect(m.hiddenCount).toBe(6);
    expect(m.nodes.map((n) => n.label)).toContain("E9");
  });

  it("merges duplicate entities that share an alias", () => {
    const m = buildAnswerGraphModel({
      entities: [ent("Foo Bar"), { ...ent("foo-bar"), id: "other" }],
      relationships: [],
    });
    expect(m.nodes).toHaveLength(1);
  });

  it("is empty-safe", () => {
    const m = buildAnswerGraphModel({ entities: [], relationships: [] });
    expect(m).toEqual({ nodes: [], edges: [], answerNodeIds: [], hiddenCount: 0 });
  });
});

describe("subgraphFromContext / focus", () => {
  it("prefers the server bundle", () => {
    const sg = { entities: [ent("A")], relationships: [] };
    expect(subgraphFromContext({ entities: [], relationships: [], subgraph: sg })).toBe(sg);
  });

  it("falls back to flat context lists", () => {
    const sg = subgraphFromContext({
      entities: [{ id: "n1", label: "A", relevance: 0.4, degree: 2 }],
      relationships: [{ source: "n1", target: "n2", type: "USES", relevance: 0.2 }],
    });
    expect(sg.entities[0]).toMatchObject({ name: "A", graph_node_id: "n1" });
    expect(sg.relationships[0].relation_type).toBe("USES");
    expect(subgraphFromContext(null).entities).toEqual([]);
  });

  it("hasAnswerGraph", () => {
    expect(hasAnswerGraph(null)).toBe(false);
    expect(
      hasAnswerGraph({ entities: [{ id: "a", label: "A", relevance: 1 }], relationships: [] }),
    ).toBe(true);
  });

  it("mapSubgraphToAnswerFocus dedups ids and keeps name fallbacks", () => {
    const f = mapSubgraphToAnswerFocus({
      entities: [ent("A", { graph_node_id: "n1" }), ent("B", { id: "ent:B" })],
      relationships: [],
    });
    expect(f.nodeIds).toEqual(["n1"]);
    expect(f.entityNames).toEqual(["A", "B", "ent:B"]);
  });
});

describe("mergeNeighborhood (EC-157-19)", () => {
  const model = buildAnswerGraphModel({
    entities: [ent("A")],
    relationships: [],
  });
  const many = Array.from({ length: 80 }, (_, i) => ({
    id: `x${i}`,
    label: "",
    node_type: "",
    entity_type: "PERSON",
  }));

  it("caps additions and keeps only edges between known nodes", () => {
    const merged = mergeNeighborhood(model, {
      nodes: many,
      edges: [
        { id: "e1", source: "ent:A", target: "x0", relationship_type: "R", weight: 1, source_ids: [], created_at: "" },
        { id: "e2", source: "ent:A", target: "x79", relationship_type: "R", weight: 1, source_ids: [], created_at: "" },
      ],
    });
    expect(merged.nodes).toHaveLength(1 + ANSWER_GRAPH_EXPAND_CAP);
    expect(merged.edges.map((e) => e.id)).toEqual(["e1"]);
    expect(merged.nodes[1].node_type).toBe("PERSON");
    expect(merged.nodes[1].label).toBe("x0");
  });

  it("is idempotent and returns the same model when nothing is new", () => {
    const once = mergeNeighborhood(model, { nodes: many.slice(0, 3), edges: [] });
    const twice = mergeNeighborhood(once, { nodes: many.slice(0, 3), edges: [] });
    expect(twice).toBe(once);
  });
});
