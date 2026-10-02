/**
 * Resolve PDF sources that require API auth into in-memory bytes.
 *
 * react-pdf / pdf.js cannot attach Authorization on a bare download URL.
 * Demo (auth_enabled) returns 401 for HEAD/GET without Bearer — same class of
 * bug as AuthenticatedMarkdownImage.
 *
 * We return `{ data: Uint8Array }` (not a blob: URL) so pdf.js never re-fetches
 * and we avoid revoke races with React effect cleanup.
 */

import { buildHeaders } from "@/lib/api/client";

export type PdfFileSource =
  | string
  | { url: string; httpHeaders?: Record<string, string> }
  | { data: ArrayBuffer | Uint8Array }
  | null;

/**
 * pdf.js loader options shared by every viewer (must be a stable reference —
 * react-pdf reloads the document when `options` changes identity).
 *
 * - `disableAutoFetch`: never download the rest of the file in the background;
 *   fetch only the byte ranges backing pages that are actually rendered.
 * - `disableStream`: pdf.js opens with one plain GET to learn the size. With
 *   streaming ON it keeps reading that response to the end (whole file, in the
 *   background, defeating range loading). With streaming OFF it cancels that
 *   probe as soon as the headers prove `Accept-Ranges: bytes`. A server without
 *   range support still works: the probe simply completes (one full download).
 * - `rangeChunkSize`: 128 KiB chunks keep first-paint small without a request
 *   storm on long documents.
 */
export const PDF_LOAD_OPTIONS = {
  disableAutoFetch: true,
  disableStream: true,
  rangeChunkSize: 131072,
};

/** API paths that serve binary PDF bytes behind auth middleware. */
export function isApiProtectedPdfUrl(url: string): boolean {
  if (!url.includes("/api/v1/documents/")) return false;
  return (
    url.includes("/download") ||
    /\/documents\/pdf\/[^/]+\/download/.test(url) ||
    /\/documents\/[^/]+\/download\/original/.test(url)
  );
}

export function extractPdfSourceUrl(file: PdfFileSource): string | null {
  if (typeof file === "string") return file;
  if (file && typeof file === "object" && "url" in file && typeof file.url === "string") {
    return file.url;
  }
  return null;
}

/**
 * Describe a protected PDF URL for pdf.js: URL + session headers, no body fetch.
 *
 * pdf.js then issues `Range` requests itself (the API answers `206`), so only
 * the first chunk is needed to paint page 1. Servers that ignore `Range`
 * degrade to a single streamed `200`, never to a failure.
 */
export function buildAuthenticatedPdfSource(url: string): {
  url: string;
  httpHeaders: Record<string, string>;
} {
  const headers = buildHeaders();
  headers.delete("Content-Type");
  const httpHeaders: Record<string, string> = {};
  headers.forEach((value, key) => {
    httpHeaders[key] = value;
  });
  return { url, httpHeaders };
}

/**
 * Fetch a protected PDF URL with session headers and return bytes for react-pdf.
 * Whole-file download: prefer {@link buildAuthenticatedPdfSource} for viewing.
 */
export async function fetchAuthenticatedPdfData(
  url: string,
  signal?: AbortSignal,
): Promise<{ data: Uint8Array }> {
  const headers = buildHeaders();
  headers.delete("Content-Type");
  const res = await fetch(url, { headers, method: "GET", signal });
  if (!res.ok) {
    throw new Error(
      `ResponseException: Unexpected server response (${res.status})`,
    );
  }
  const buffer = await res.arrayBuffer();
  return { data: new Uint8Array(buffer) };
}

/** @deprecated Prefer {@link fetchAuthenticatedPdfData} — blob URLs race with revoke. */
export async function fetchAuthenticatedPdfBlobUrl(
  url: string,
  signal?: AbortSignal,
): Promise<string> {
  const { data } = await fetchAuthenticatedPdfData(url, signal);
  const ab = data.buffer.slice(
    data.byteOffset,
    data.byteOffset + data.byteLength,
  ) as ArrayBuffer;
  return URL.createObjectURL(new Blob([ab], { type: "application/pdf" }));
}
