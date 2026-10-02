/**
 * SPEC-157 — What a document should be viewed as (PDF vs text).
 *
 * SSOT shared by the full-page viewer (`/documents/[id]`) and the Query
 * companion pane, so both agree on when a PDF binary exists (LAW-157-5).
 */
import type { Document } from "@/types";

export type PdfIdSource = Pick<Document, "id" | "pdf_id" | "source_type">;

/**
 * PDF binary id for a document, or `null` for text-like documents.
 * Older rows may lack `pdf_id`; a `source_type` of "pdf" then means the
 * document id doubles as the PDF id.
 */
export function resolvePdfId(
  doc: PdfIdSource | null | undefined,
): string | null {
  if (!doc) return null;
  if (doc.pdf_id) return doc.pdf_id;
  return doc.source_type === "pdf" ? doc.id : null;
}

export type ViewerTarget =
  | { kind: "pdf"; pdfId: string }
  | { kind: "text" };

export function resolveViewerTarget(
  doc: PdfIdSource | null | undefined,
): ViewerTarget {
  const pdfId = resolvePdfId(doc);
  return pdfId ? { kind: "pdf", pdfId } : { kind: "text" };
}

/**
 * Merge a PDF's extracted markdown (stored separately) into the document so
 * `ContentRenderer` can show it without special PDF handling.
 */
export function withPdfMarkdown(
  doc: Document | null | undefined,
  pdfId: string | null,
  pdfMarkdown: string | null | undefined,
): Document | null {
  if (!doc) return null;
  const markdown = (pdfMarkdown?.trim() || doc.content?.trim() || "") as string;
  if (pdfId && markdown) {
    return {
      ...doc,
      content: markdown,
      // PDF mime routes to the plain-text path; markdown needs text/markdown.
      mime_type: "text/markdown",
      source_type: "pdf" as const,
    };
  }
  return doc;
}
