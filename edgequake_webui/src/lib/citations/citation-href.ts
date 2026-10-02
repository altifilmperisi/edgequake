import type { CitationChunk } from "@/components/query/markdown/citation-resolver";
import { buildDocumentCitationUrl } from "@/lib/utils/document-url";

/** Deep link to the cited passage in the document viewer (page + chunk + highlight). */
export function buildCitationHref(chunk: CitationChunk): string {
  return buildDocumentCitationUrl({
    documentId: encodeURIComponent(chunk.document_id),
    chunkId: chunk.chunk_id,
    page: chunk.page_start,
    chunkContent: chunk.content,
    startLine: chunk.start_line,
    endLine: chunk.end_line,
  });
}

export type ClickIntent = "same-tab" | "new-tab" | "preview";

/**
 * Decide what a click on a citation chip should do.
 * - modifier / middle click → new tab (standard link behaviour)
 * - touch / no-hover devices → toggle the preview (there is no hover to reveal it)
 * - otherwise → navigate to the source
 */
export function resolveClickIntent(opts: {
  metaKey: boolean;
  ctrlKey: boolean;
  shiftKey: boolean;
  button: number;
  canHover: boolean;
}): ClickIntent {
  if (opts.metaKey || opts.ctrlKey || opts.shiftKey || opts.button === 1) {
    return "new-tab";
  }
  return opts.canHover ? "same-tab" : "preview";
}

/** Percent label clamped to 0–100 (raw retrieval scores may exceed 1). */
export function formatMatchPercent(score: number): number {
  if (!Number.isFinite(score)) return 0;
  return Math.round(Math.min(1, Math.max(0, score)) * 100);
}

/** True for a plain left click (no modifier) — i.e. not "open in new tab". */
export function isPlainActivation(e: {
  metaKey: boolean;
  ctrlKey: boolean;
  shiftKey: boolean;
  altKey?: boolean;
  button: number;
}): boolean {
  return !e.metaKey && !e.ctrlKey && !e.shiftKey && !e.altKey && e.button === 0;
}
