import { describe, expect, it } from "vitest";
import {
  resolveDropzoneFillLayout,
  DROPZONE_ROW_MAX_HEIGHT_PX,
  DROPZONE_STACK_MAX_WIDTH_PX,
} from "../dropzone-fill-layout";

describe("resolveDropzoneFillLayout", () => {
  it("uses row for short wide panels", () => {
    expect(resolveDropzoneFillLayout(800, DROPZONE_ROW_MAX_HEIGHT_PX - 1)).toBe(
      "row",
    );
  });

  it("uses stack for tall narrow panels", () => {
    expect(
      resolveDropzoneFillLayout(DROPZONE_STACK_MAX_WIDTH_PX - 1, 400),
    ).toBe("stack");
  });

  it("uses hero for roomy panels", () => {
    expect(resolveDropzoneFillLayout(600, 240)).toBe("hero");
  });

  it("defaults to hero when size unknown", () => {
    expect(resolveDropzoneFillLayout(0, 0)).toBe("hero");
  });
});
