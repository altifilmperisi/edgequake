/**
 * SPEC-143 — Page sync mode storage + helpers.
 */

import { describe, expect, it } from 'bun:test';
import {
  DEFAULT_PAGE_SYNC_MODE,
  followMarkdown,
  followPdf,
  isPageSyncMode,
  pdfCurrentPageForMode,
  PAGE_SYNC_MODE_STORAGE_KEY,
  publishesFromMd,
  publishesFromPdf,
  readStoredPageSyncMode,
  writeStoredPageSyncMode,
} from '../page-sync-mode';

function memoryStorage(seed: Record<string, string> = {}): Storage {
  const map = new Map(Object.entries(seed));
  return {
    get length() {
      return map.size;
    },
    clear() {
      map.clear();
    },
    getItem(key: string) {
      return map.has(key) ? map.get(key)! : null;
    },
    key(index: number) {
      return [...map.keys()][index] ?? null;
    },
    removeItem(key: string) {
      map.delete(key);
    },
    setItem(key: string, value: string) {
      map.set(key, String(value));
    },
  };
}

describe('page-sync-mode', () => {
  it('defaults to pdf-to-md', () => {
    expect(DEFAULT_PAGE_SYNC_MODE).toBe('pdf-to-md');
    expect(readStoredPageSyncMode(null)).toBe('pdf-to-md');
  });

  it('round-trips a stored mode', () => {
    const storage = memoryStorage();
    writeStoredPageSyncMode('md-to-pdf', storage);
    expect(storage.getItem(PAGE_SYNC_MODE_STORAGE_KEY)).toBe('md-to-pdf');
    expect(readStoredPageSyncMode(storage)).toBe('md-to-pdf');
  });

  it('falls back when storage holds garbage', () => {
    const storage = memoryStorage({ [PAGE_SYNC_MODE_STORAGE_KEY]: 'bidirectional' });
    expect(readStoredPageSyncMode(storage)).toBe('pdf-to-md');
  });

  it('gates follow helpers by mode', () => {
    expect(followMarkdown('pdf-to-md')).toBe(true);
    expect(followMarkdown('md-to-pdf')).toBe(false);
    expect(followMarkdown('none')).toBe(false);
    expect(followPdf('md-to-pdf')).toBe(true);
    expect(followPdf('pdf-to-md')).toBe(false);
    expect(followPdf('none')).toBe(false);
  });

  it('gates publish helpers by mode', () => {
    expect(publishesFromPdf('pdf-to-md')).toBe(true);
    expect(publishesFromPdf('md-to-pdf')).toBe(false);
    expect(publishesFromPdf('none')).toBe(false);
    expect(publishesFromMd('md-to-pdf')).toBe(true);
    expect(publishesFromMd('pdf-to-md')).toBe(false);
    expect(publishesFromMd('none')).toBe(false);
  });

  it('omits controlled PDF page only in none mode', () => {
    expect(pdfCurrentPageForMode('none', 4)).toBeUndefined();
    expect(pdfCurrentPageForMode('pdf-to-md', 4)).toBe(4);
    expect(pdfCurrentPageForMode('md-to-pdf', 4)).toBe(4);
  });

  it('validates mode strings', () => {
    expect(isPageSyncMode('none')).toBe(true);
    expect(isPageSyncMode('pdf-to-md')).toBe(true);
    expect(isPageSyncMode('md-to-pdf')).toBe(true);
    expect(isPageSyncMode('on')).toBe(false);
  });
});
