import { describe, expect, it } from "vitest";
import { focusModeFromHoverSelect } from "./focus-strategies";

describe("focusModeFromHoverSelect", () => {
  it("selection outranks hover so hovering never drops the neighbourhood", () => {
    expect(focusModeFromHoverSelect("b", "a")).toEqual({ mode: "select", ids: ["a"] });
  });

  it("hover previews only when nothing is selected", () => {
    expect(focusModeFromHoverSelect("b", null)).toEqual({ mode: "hover", ids: ["b"] });
  });

  it("is none without hover or selection", () => {
    expect(focusModeFromHoverSelect(null, null)).toEqual({ mode: "none", ids: [] });
  });
});
