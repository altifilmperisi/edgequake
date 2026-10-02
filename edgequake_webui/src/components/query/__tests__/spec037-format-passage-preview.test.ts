import { describe, expect, it } from "bun:test";

import { formatPassagePreview } from "@/lib/citations/passage-text";

describe("SPEC-037 formatPassagePreview", () => {
  it("returns full text when fullChunkContent is true", () => {
    const long =
      "Alpha beta gamma delta ".repeat(20) + "uncertain.";
    const out = formatPassagePreview(long, true);
    expect(out).toContain("uncertain.");
    expect(out.endsWith("…")).toBe(false);
    expect(out.length).toBeGreaterThan(220);
  });

  it("truncates to 220 chars when fullChunkContent is false", () => {
    const long = "word ".repeat(80);
    const out = formatPassagePreview(long, false);
    expect(out.length).toBeLessThanOrEqual(221);
    expect(out.endsWith("…")).toBe(true);
  });
});
