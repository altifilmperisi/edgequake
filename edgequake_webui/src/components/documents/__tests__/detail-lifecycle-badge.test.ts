import { describe, expect, it } from "vitest";
import { lifecyclePillTone } from "../detail-lifecycle-badge";

describe("lifecyclePillTone", () => {
  it("in-flight work always wins over kind", () => {
    expect(lifecyclePillTone({ kind: "extracting", showSpinner: true })).toBe("working");
    expect(lifecyclePillTone({ kind: "failed", showSpinner: true })).toBe("working");
  });

  it("maps terminal problem states to their own tone", () => {
    expect(lifecyclePillTone({ kind: "partial", showSpinner: false })).toBe("partial");
    expect(lifecyclePillTone({ kind: "failed", showSpinner: false })).toBe("failed");
    expect(lifecyclePillTone({ kind: "cancelled", showSpinner: false })).toBe("cancelled");
  });

  it("renders nothing for a healthy, finished document", () => {
    expect(lifecyclePillTone({ kind: "ready", showSpinner: false })).toBeNull();
  });
});
