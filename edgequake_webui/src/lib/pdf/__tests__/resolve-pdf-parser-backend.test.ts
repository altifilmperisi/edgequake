import { describe, expect, it } from "bun:test";
import {
  formatServerDefaultPdfParserLabel,
  formatWorkspaceDefaultPdfParserLabel,
  getServerDefaultPdfParserBackend,
  pdfParserBackendDisplayName,
  resolvePdfParserBackend,
  resolvesToVisionParser,
  type PdfParserResolutionContext,
} from "@/lib/pdf/resolve-pdf-parser-backend";
import {
  LARGE_PDF_PAGE_THRESHOLD,
  shouldPromptLargePdfParserChoice,
} from "@/lib/pdf/large-pdf-admission";

describe("resolvePdfParserBackend", () => {
  it("upload override wins over workspace and server", () => {
    const ctx: PdfParserResolutionContext = {
      uploadChoice: "edgeparse",
      workspaceBackend: "vision",
      serverBackend: "vision",
    };
    const resolved = resolvePdfParserBackend(ctx);
    expect(resolved.backend).toBe("edgeparse");
    expect(resolved.source).toBe("upload");
    expect(resolved.isExplicit).toBe(true);
    expect(resolved.allowsAutoRoute).toBe(false);
  });

  it("workspace default wins when upload is default", () => {
    const ctx: PdfParserResolutionContext = {
      uploadChoice: "default",
      workspaceBackend: "edgeparse",
      serverBackend: "vision",
    };
    const resolved = resolvePdfParserBackend(ctx);
    expect(resolved.backend).toBe("edgeparse");
    expect(resolved.source).toBe("workspace");
  });

  it("tenant wins when workspace unset", () => {
    const ctx: PdfParserResolutionContext = {
      uploadChoice: "default",
      workspaceBackend: null,
      tenantBackend: "edgeparse",
      serverBackend: "vision",
    };
    const resolved = resolvePdfParserBackend(ctx);
    expect(resolved.backend).toBe("edgeparse");
    expect(resolved.source).toBe("tenant");
  });

  it("server default vision is inviolable (not auto)", () => {
    const ctx: PdfParserResolutionContext = {
      uploadChoice: "default",
      workspaceBackend: undefined,
      serverBackend: "vision",
    };
    const resolved = resolvePdfParserBackend(ctx);
    expect(resolved.backend).toBe("vision");
    expect(resolved.isExplicit).toBe(true);
    expect(resolved.allowsAutoRoute).toBe(false);
  });

  it("explicit auto allows route", () => {
    const resolved = resolvePdfParserBackend({
      uploadChoice: "default",
      workspaceBackend: "auto",
    });
    expect(resolved.backend).toBe("auto");
    expect(resolved.runtimeBackend).toBe("vision");
    expect(resolved.allowsAutoRoute).toBe(true);
    expect(resolved.isExplicit).toBe(false);
  });

  it("falls back to server then vision", () => {
    const ctx: PdfParserResolutionContext = {
      uploadChoice: "default",
      workspaceBackend: undefined,
      serverBackend: "vision",
    };
    expect(resolvePdfParserBackend(ctx).backend).toBe("vision");
    expect(resolvePdfParserBackend(ctx).source).toBe("server");
  });

  it("upload edgeparse-ocr is explicit runtime OCR mode", () => {
    const resolved = resolvePdfParserBackend({
      uploadChoice: "edgeparse-ocr",
      workspaceBackend: "vision",
      serverBackend: "vision",
    });
    expect(resolved.backend).toBe("edgeparse-ocr");
    expect(resolved.runtimeBackend).toBe("edgeparse-ocr");
    expect(resolved.source).toBe("upload");
    expect(resolved.isExplicit).toBe(true);
    expect(resolved.allowsAutoRoute).toBe(false);
    expect(pdfParserBackendDisplayName("edgeparse-ocr")).toBe("EdgeParse + OCR");
  });

  it("getServerDefaultPdfParserBackend defaults to vision", () => {
    expect(getServerDefaultPdfParserBackend()).toBe("vision");
  });

  it("never-silent server default label includes resolved backend", () => {
    expect(pdfParserBackendDisplayName("vision")).toBe("Vision");
    expect(pdfParserBackendDisplayName("edgeparse")).toBe("EdgeParse");
    expect(pdfParserBackendDisplayName("auto")).toBe("Auto");
    const label = formatServerDefaultPdfParserLabel(
      (_key, defaultValue) => defaultValue,
      "vision",
    );
    expect(label).toBe("Server Default (Vision)");
    expect(label).not.toBe("Server Default");
  });

  it("never-silent workspace default label uses workspace backend", () => {
    const t = (_key: string, defaultValue: string) => defaultValue;
    expect(formatWorkspaceDefaultPdfParserLabel(t, "vision")).toBe(
      "Workspace Default (Vision)",
    );
    expect(formatWorkspaceDefaultPdfParserLabel(t, "edgeparse")).toBe(
      "Workspace Default (EdgeParse)",
    );
    expect(formatWorkspaceDefaultPdfParserLabel(t, "auto")).toBe(
      "Workspace Default (Auto)",
    );
  });

  it("workspace default label falls back to server when workspace unset", () => {
    const t = (_key: string, defaultValue: string) => defaultValue;
    expect(
      formatWorkspaceDefaultPdfParserLabel(t, null, "edgeparse"),
    ).toBe("Workspace Default (EdgeParse)");
    expect(
      formatWorkspaceDefaultPdfParserLabel(t, undefined, "vision"),
    ).toBe("Workspace Default (Vision)");
  });
});

describe("shouldPromptLargePdfParserChoice", () => {
  it("prompts for large PDF when Vision is resolved", () => {
    expect(
      shouldPromptLargePdfParserChoice(LARGE_PDF_PAGE_THRESHOLD, {
        uploadChoice: "default",
        workspaceBackend: undefined,
        serverBackend: "vision",
      }),
    ).toBe(true);
  });

  it("skips dialog when upload selects EdgeParse", () => {
    expect(
      shouldPromptLargePdfParserChoice(603, {
        uploadChoice: "edgeparse",
        workspaceBackend: undefined,
        serverBackend: "vision",
      }),
    ).toBe(false);
  });

  it("skips dialog when workspace default is EdgeParse", () => {
    expect(
      shouldPromptLargePdfParserChoice(603, {
        uploadChoice: "default",
        workspaceBackend: "edgeparse",
        serverBackend: "vision",
      }),
    ).toBe(false);
  });

  it("skips dialog below page threshold even with Vision", () => {
    expect(
      shouldPromptLargePdfParserChoice(LARGE_PDF_PAGE_THRESHOLD - 1, {
        uploadChoice: "default",
        serverBackend: "vision",
      }),
    ).toBe(false);
  });

  it("resolvesToVisionParser matches backend chain", () => {
    expect(
      resolvesToVisionParser({
        uploadChoice: "vision",
        workspaceBackend: "edgeparse",
      }),
    ).toBe(true);
  });
});
