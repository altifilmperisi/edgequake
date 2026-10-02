/**
 * Authenticated PDF URL resolution (PDF viewer 401 fix).
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  buildAuthenticatedPdfSource,
  PDF_LOAD_OPTIONS,
  extractPdfSourceUrl,
  fetchAuthenticatedPdfBlobUrl,
  fetchAuthenticatedPdfData,
  isApiProtectedPdfUrl,
} from "../resolve-authenticated-pdf-source";

vi.mock("@/lib/api/client", () => ({
  buildHeaders: () => {
    const h = new Headers();
    h.set("Authorization", "Bearer test-token-149");
    h.set("Content-Type", "application/json");
    h.set("X-Workspace-ID", "ws-1");
    return h;
  },
}));

describe("resolve-authenticated-pdf-source", () => {
  beforeEach(() => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => {
        const bytes = new Uint8Array([0x25, 0x50, 0x44, 0x46]); // %PDF
        return new Response(bytes, {
          status: 200,
          headers: { "Content-Type": "application/pdf" },
        });
      }),
    );
    vi.stubGlobal("URL", {
      ...URL,
      createObjectURL: vi.fn(() => "blob:mock-pdf-url"),
      revokeObjectURL: vi.fn(),
    });
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("detects protected download URLs", () => {
    expect(
      isApiProtectedPdfUrl(
        "https://demo.edgequake.com/api/v1/documents/pdf/abc/download",
      ),
    ).toBe(true);
    expect(
      isApiProtectedPdfUrl("/api/v1/documents/01a0/download/original"),
    ).toBe(true);
    expect(isApiProtectedPdfUrl("https://cdn.example/file.pdf")).toBe(false);
    expect(isApiProtectedPdfUrl("blob:http://localhost/x")).toBe(false);
  });

  it("extracts url from string or {url} sources", () => {
    expect(extractPdfSourceUrl("https://x/api/v1/documents/pdf/a/download")).toBe(
      "https://x/api/v1/documents/pdf/a/download",
    );
    expect(
      extractPdfSourceUrl({ url: "/api/v1/documents/pdf/a/download" }),
    ).toBe("/api/v1/documents/pdf/a/download");
    expect(extractPdfSourceUrl({ data: new Uint8Array([1]) })).toBeNull();
    expect(extractPdfSourceUrl(null)).toBeNull();
  });

  it("fetches with Authorization and returns PDF bytes", async () => {
    const result = await fetchAuthenticatedPdfData(
      "https://demo.edgequake.com/api/v1/documents/pdf/ead6/download",
    );

    expect(result.data[0]).toBe(0x25); // %
    expect(result.data[1]).toBe(0x50); // P
    expect(fetch).toHaveBeenCalledTimes(1);
    const [url, init] = vi.mocked(fetch).mock.calls[0]!;
    expect(url).toContain("/documents/pdf/ead6/download");
    expect(init?.method).toBe("GET");
    const headers = init?.headers as Headers;
    expect(headers.get("Authorization")).toBe("Bearer test-token-149");
    expect(headers.get("Content-Type")).toBeNull();
    expect(headers.get("X-Workspace-ID")).toBe("ws-1");
  });

  it("maps non-OK responses to ResponseException", async () => {
    vi.mocked(fetch).mockResolvedValueOnce(
      new Response("nope", { status: 401 }),
    );
    await expect(
      fetchAuthenticatedPdfData("/api/v1/documents/pdf/x/download"),
    ).rejects.toThrow("ResponseException: Unexpected server response (401)");
  });

  it("legacy blob helper still attaches Authorization", async () => {
    const blobUrl = await fetchAuthenticatedPdfBlobUrl(
      "/api/v1/documents/pdf/x/download",
    );
    expect(blobUrl).toBe("blob:mock-pdf-url");
    const headers = vi.mocked(fetch).mock.calls[0]![1]?.headers as Headers;
    expect(headers.get("Authorization")).toBe("Bearer test-token-149");
  });

  it("describes a protected PDF for pdf.js without downloading the body", () => {
    const fetchMock = vi.mocked(fetch);
    const src = buildAuthenticatedPdfSource(
      "http://api/api/v1/documents/pdf/abc/download",
    );
    expect(src.url).toBe("http://api/api/v1/documents/pdf/abc/download");
    expect(src.httpHeaders.authorization).toBe("Bearer test-token-149");
    expect(src.httpHeaders["x-workspace-id"]).toBe("ws-1");
    // JSON content-type would make no sense on a binary GET.
    expect(src.httpHeaders["content-type"]).toBeUndefined();
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("loads on demand: no background prefetch, 128 KiB range chunks", () => {
    expect(PDF_LOAD_OPTIONS.disableAutoFetch).toBe(true);
    expect(PDF_LOAD_OPTIONS.disableStream).toBe(true);
    expect(PDF_LOAD_OPTIONS.rangeChunkSize).toBe(131072);
  });
});
