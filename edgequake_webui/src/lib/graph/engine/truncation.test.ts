import { describe, expect, it } from "vitest";
import { resolveTruncationInfo } from "./truncation";

describe("resolveTruncationInfo", () => {
  it("honours server is_truncated + totals (FIXTURE_500 shape)", () => {
    const r = resolveTruncationInfo({
      streamedNodes: 200,
      streamedEdges: 250,
      totalNodes: 500,
      totalEdges: 800,
      isTruncated: true,
    });
    expect(r.isTruncated).toBe(true);
    expect(r.totalNodes).toBe(500);
    expect(r.totalEdges).toBe(800);
  });

  it("does not invert small graphs as truncated", () => {
    const r = resolveTruncationInfo({
      streamedNodes: 50,
      streamedEdges: 40,
      totalNodes: 50,
      totalEdges: 40,
    });
    expect(r.isTruncated).toBe(false);
    expect(r.totalNodes).toBe(50);
  });

  it("infers truncation when totals exceed streamed counts", () => {
    const r = resolveTruncationInfo({
      streamedNodes: 200,
      streamedEdges: 100,
      totalNodes: 500,
      totalEdges: 900,
    });
    expect(r.isTruncated).toBe(true);
    expect(r.totalNodes).toBe(500);
  });

  it("falls back to streamed counts when totals absent", () => {
    const r = resolveTruncationInfo({
      streamedNodes: 10,
      streamedEdges: 5,
    });
    expect(r.isTruncated).toBe(false);
    expect(r.totalNodes).toBe(10);
    expect(r.totalEdges).toBe(5);
  });
});
