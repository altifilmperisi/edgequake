import { describe, expect, it } from "vitest";
import { resolvePdfId, resolveViewerTarget } from "../viewer-target";

describe("viewer-target", () => {
  it("prefers pdf_id", () => {
    expect(
      resolvePdfId({ id: "d", pdf_id: "p", source_type: "file" }),
    ).toBe("p");
  });
  it("falls back to the document id for pdf source type", () => {
    expect(resolvePdfId({ id: "d", source_type: "pdf" })).toBe("d");
  });
  it("is null for text documents and missing docs", () => {
    expect(resolvePdfId({ id: "d", source_type: "markdown" })).toBeNull();
    expect(resolvePdfId(undefined)).toBeNull();
    expect(resolveViewerTarget(null)).toEqual({ kind: "text" });
    expect(resolveViewerTarget({ id: "d", pdf_id: "p" })).toEqual({
      kind: "pdf",
      pdfId: "p",
    });
  });
});
