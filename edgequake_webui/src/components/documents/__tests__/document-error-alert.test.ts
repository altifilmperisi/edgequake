import { describe, expect, it } from "bun:test";
import { readPathBusyHint } from "../document-error-alert";

describe("GH-400 readPathBusyHint", () => {
  it("maps work_deadline", () => {
    expect(readPathBusyHint("work_deadline")).toBe("list timed out under load");
  });

  it("maps permit_wait", () => {
    expect(readPathBusyHint("permit_wait")).toBe("too many list requests");
  });

  it("maps permit_closed", () => {
    expect(readPathBusyHint("permit_closed")).toBe("read path unavailable");
  });

  it("returns null for unknown / non-string", () => {
    expect(readPathBusyHint("other")).toBeNull();
    expect(readPathBusyHint(undefined)).toBeNull();
    expect(readPathBusyHint(42)).toBeNull();
  });
});
