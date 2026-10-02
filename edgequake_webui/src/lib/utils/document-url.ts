/**
 * @module document-url
 * @description Canonical deeplink URL builder for document + page citations.
 *
 * @implements SPEC-033 — Single definition of deeplink URL schema.
 * All citation surfaces (hierarchy tree, query citations) MUST use this helper
 * so the URL schema is defined in exactly one place (DRY principle).
 *
 * URL schema: /documents/{docId}?chunk={chunkId}&page={pageN}
 *
 * @example
 * buildDocumentPageUrl('doc-1', 'chunk-abc', 3)
 * // → '/documents/doc-1?chunk=chunk-abc&page=3'
 *
 * buildDocumentPageUrl('doc-1', undefined, 3)
 * // → '/documents/doc-1?page=3'
 *
 * buildDocumentPageUrl('doc-1')
 * // → '/documents/doc-1'
 */

/**
 * Build a canonical document viewer URL with optional chunk + page params.
 *
 * Rules:
 * - `page` values < 1 are omitted (treated as "no page").
 * - `chunkId` is omitted when undefined or empty.
 * - Parameter order is always chunk first, then page.
 *
 * @param docId     - Document UUID
 * @param chunkId   - Optional chunk UUID for chunk highlight
 * @param page      - Optional 1-indexed PDF page number for viewer navigation
 */
export function buildDocumentPageUrl(
  docId: string,
  chunkId?: string,
  page?: number,
): string {
  const params = new URLSearchParams();
  if (chunkId) params.set('chunk', chunkId);
  if (page !== undefined && page >= 1) params.set('page', String(page));
  const qs = params.toString();
  return `/documents/${docId}${qs ? `?${qs}` : ''}`;
}

/**
 * Citation badge for a chunk page span (SPEC-135).
 * Uses an en-dash between start and end when they differ.
 */
export function formatChunkPageBadge(
  pageStart?: number,
  pageEnd?: number,
): string | null {
  if (pageStart === undefined || pageStart < 1) {
    return null;
  }
  if (pageEnd !== undefined && pageEnd > pageStart) {
    return `p.${pageStart}–${pageEnd}`;
  }
  return `p.${pageStart}`;
}

/**
 * Build a citation deeplink preserving line-range, chunk selection and optional page navigation.
 *
 * Behavior mirrors citation click UX:
 * - line range is added only when both start/end are present
 * - highlight is used only when no explicit line range is available
 */
export function buildDocumentCitationUrl({
  documentId,
  chunkId,
  page,
  chunkContent,
  startLine,
  endLine,
}: {
  documentId: string;
  chunkId?: string;
  page?: number;
  chunkContent?: string;
  startLine?: number;
  endLine?: number;
}): string {
  const baseUrl = buildDocumentPageUrl(documentId, chunkId, page);
  const [path, existingQuery = ''] = baseUrl.split('?');
  const params = new URLSearchParams(existingQuery);

  if (startLine !== undefined && endLine !== undefined) {
    params.set('start_line', startLine.toString());
    params.set('end_line', endLine.toString());
  }

  if (chunkContent && startLine === undefined) {
    params.set('highlight', chunkContent.slice(0, 100));
  }

  const queryString = params.toString();
  return `${path}${queryString ? `?${queryString}` : ''}`;
}

/** True when an href is a document viewer deeplink (`/documents/{id}…`). */
export function isDocumentDeeplink(href: string | undefined | null): boolean {
  if (!href) return false;
  try {
    const path = href.startsWith('http')
      ? new URL(href).pathname
      : href.split('?')[0] ?? '';
    return /^\/documents\//.test(path);
  } catch {
    return href.startsWith('/documents/');
  }
}

/** Path + search for router.push from a citation href. */
export function documentPathFromHref(href: string): string {
  try {
    if (href.startsWith('http')) {
      const u = new URL(href);
      return `${u.pathname}${u.search}`;
    }
  } catch {
    /* fall through */
  }
  return href;
}

/**
 * Parse a 1-indexed page query value. SSOT for `?page=` across the document
 * viewer, citation hrefs and the Query companion pane (SPEC-157 LAW-157-4).
 */
export function parsePageParam(
  value: string | null | undefined,
): number | undefined {
  if (!value) return undefined;
  const n = Number.parseInt(value, 10);
  return Number.isFinite(n) && n >= 1 ? n : undefined;
}

/** Parse 1-indexed page from `?page=N` in a citation href. */
export function parsePageFromHref(href: string): number | undefined {
  try {
    const qs = href.includes('?') ? href.slice(href.indexOf('?') + 1) : '';
    return parsePageParam(new URLSearchParams(qs).get('page'));
  } catch {
    return undefined;
  }
}
