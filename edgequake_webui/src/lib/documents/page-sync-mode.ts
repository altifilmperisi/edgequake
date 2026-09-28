/**
 * SPEC-143 — Explicit page-sync direction (not a bidirectional toggle).
 */

export const PAGE_SYNC_MODE_STORAGE_KEY = 'eq-page-sync-mode';

export type PageSyncMode = 'none' | 'pdf-to-md' | 'md-to-pdf';

export const DEFAULT_PAGE_SYNC_MODE: PageSyncMode = 'pdf-to-md';

export const PAGE_SYNC_MODES: readonly PageSyncMode[] = [
  'none',
  'pdf-to-md',
  'md-to-pdf',
] as const;

export function isPageSyncMode(value: unknown): value is PageSyncMode {
  return value === 'none' || value === 'pdf-to-md' || value === 'md-to-pdf';
}

/** Markdown follows the PDF. */
export function followMarkdown(mode: PageSyncMode): boolean {
  return mode === 'pdf-to-md';
}

/** PDF follows the markdown. */
export function followPdf(mode: PageSyncMode): boolean {
  return mode === 'md-to-pdf';
}

/** PDF scroll/toolbar publishes the shared active page. */
export function publishesFromPdf(mode: PageSyncMode): boolean {
  return mode === 'pdf-to-md';
}

/** Markdown scroll publishes the shared active page. */
export function publishesFromMd(mode: PageSyncMode): boolean {
  return mode === 'md-to-pdf';
}

/**
 * Controlled PDF page when sync is directional.
 * - pdf-to-md / md-to-pdf: PDF is driven by activePage (deeplink + stick-to-top).
 * - none: omit currentPage so the PDF owns its own page.
 */
export function pdfCurrentPageForMode(
  mode: PageSyncMode,
  activePage: number,
): number | undefined {
  if (mode === 'none') return undefined;
  return activePage >= 1 ? activePage : undefined;
}

export function readStoredPageSyncMode(
  storage: Pick<Storage, 'getItem'> | null | undefined = typeof window !== 'undefined'
    ? window.localStorage
    : null,
): PageSyncMode {
  if (!storage) return DEFAULT_PAGE_SYNC_MODE;
  try {
    const raw = storage.getItem(PAGE_SYNC_MODE_STORAGE_KEY);
    if (isPageSyncMode(raw)) return raw;
  } catch {
    // private mode / blocked storage
  }
  return DEFAULT_PAGE_SYNC_MODE;
}

export function writeStoredPageSyncMode(
  mode: PageSyncMode,
  storage: Pick<Storage, 'setItem'> | null | undefined = typeof window !== 'undefined'
    ? window.localStorage
    : null,
): void {
  if (!storage) return;
  try {
    storage.setItem(PAGE_SYNC_MODE_STORAGE_KEY, mode);
  } catch {
    // private mode / blocked storage
  }
}
