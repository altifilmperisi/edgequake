import { describe, expect, it } from "vitest";
import { toSigmaColor } from "./css-token";

describe("toSigmaColor", () => {
  it("passes Sigma-parsable colours through untouched", () => {
    for (const c of ["#6b7280", "#fff", "rgb(1, 2, 3)", "rgba(0,0,0,0.35)", "red"]) {
      expect(toSigmaColor(c)).toBe(c);
    }
  });

  it("returns empty input as-is", () => {
    expect(toSigmaColor("")).toBe("");
  });

  it("keeps oklch() when no canvas is available (node / SSR)", () => {
    // Real conversion needs a browser canvas; covered by the Playwright graph specs.
    expect(toSigmaColor("oklch(0.55 0.02 260)")).toBe("oklch(0.55 0.02 260)");
  });
});
