import { describe, expect, it } from "vitest";
import {
  pickCentralNode,
  pickNodeInDirection,
  type NavPoint,
} from "./spatial-navigation";

const pts: NavPoint[] = [
  { id: "c", x: 100, y: 100 },
  { id: "right-near", x: 160, y: 105 },
  { id: "right-far-linked", x: 300, y: 100 },
  { id: "left", x: 20, y: 100 },
  { id: "up", x: 100, y: 20 },
  { id: "down-diag", x: 140, y: 190 },
];
const current = pts[0]!;

describe("pickNodeInDirection", () => {
  it("prefers a connected node in that direction over a nearer unconnected one", () => {
    const next = pickNodeInDirection(current, "right", new Set(["right-far-linked"]), pts);
    expect(next).toBe("right-far-linked");
  });

  it("falls back to the nearest node in the cone when no neighbour lies that way", () => {
    const next = pickNodeInDirection(current, "right", new Set(["left"]), pts);
    expect(next).toBe("right-near");
  });

  it("maps up/down/left to screen directions (y grows downward)", () => {
    expect(pickNodeInDirection(current, "up", new Set(), pts)).toBe("up");
    expect(pickNodeInDirection(current, "left", new Set(), pts)).toBe("left");
    expect(pickNodeInDirection(current, "down", new Set(), pts)).toBe("down-diag");
  });

  it("returns null when nothing lies in that direction", () => {
    const lonely = [current, { id: "left", x: 20, y: 100 }];
    expect(pickNodeInDirection(current, "right", new Set(), lonely)).toBeNull();
  });
});

describe("pickCentralNode", () => {
  it("picks the node nearest the centre", () => {
    expect(pickCentralNode(pts, { x: 150, y: 110 })).toBe("right-near");
    expect(pickCentralNode([], { x: 0, y: 0 })).toBeNull();
  });
});
