/**
 * SPEC-143 — Controller lock + directional publish unit tests (U-143-02).
 */

import { describe, expect, it } from 'bun:test';
import { shouldAcceptPageUpdate } from '../use-page-sync-controller';
import {
  followMarkdown,
  followPdf,
  publishesFromMd,
  publishesFromPdf,
  readStoredPageSyncMode,
  writeStoredPageSyncMode,
} from '@/lib/documents/page-sync-mode';

describe('page sync lock (U-143-02)', () => {
  it('accepts same-driver updates while locked', () => {
    expect(
      shouldAcceptPageUpdate({
        now: 100,
        lockUntil: 300,
        currentDriver: 'pdf',
        source: 'pdf',
        currentPage: 1,
        nextPage: 2,
        gestureDriver: 'none',
      }),
    ).toBe(true);
  });

  it('rejects cross-driver updates while locked', () => {
    expect(
      shouldAcceptPageUpdate({
        now: 100,
        lockUntil: 300,
        currentDriver: 'pdf',
        source: 'md',
        currentPage: 1,
        nextPage: 2,
        gestureDriver: 'none',
      }),
    ).toBe(false);
  });

  it('accepts cross-driver after settle', () => {
    expect(
      shouldAcceptPageUpdate({
        now: 400,
        lockUntil: 300,
        currentDriver: 'pdf',
        source: 'md',
        currentPage: 1,
        nextPage: 2,
        gestureDriver: 'none',
      }),
    ).toBe(true);
  });

  it('rejects cross-driver while gesture holds pdf', () => {
    expect(
      shouldAcceptPageUpdate({
        now: 500,
        lockUntil: 0,
        currentDriver: 'pdf',
        source: 'md',
        currentPage: 2,
        nextPage: 3,
        gestureDriver: 'pdf',
      }),
    ).toBe(false);
  });

  it('accepts same gesture driver while held', () => {
    expect(
      shouldAcceptPageUpdate({
        now: 500,
        lockUntil: 0,
        currentDriver: 'pdf',
        source: 'pdf',
        currentPage: 2,
        nextPage: 3,
        gestureDriver: 'pdf',
      }),
    ).toBe(true);
  });

  it('rejects same-page updates (no driver reset)', () => {
    expect(
      shouldAcceptPageUpdate({
        now: 100,
        lockUntil: 0,
        currentDriver: 'pdf',
        source: 'md',
        currentPage: 2,
        nextPage: 2,
        gestureDriver: 'none',
      }),
    ).toBe(false);
  });

  it('rejects same-page external echo', () => {
    expect(
      shouldAcceptPageUpdate({
        now: 100,
        lockUntil: 0,
        currentDriver: 'pdf',
        source: 'external',
        currentPage: 4,
        nextPage: 4,
        gestureDriver: 'none',
      }),
    ).toBe(false);
  });
});

describe('directional publish gates (U-143-mode)', () => {
  it('pdf-to-md publishes from PDF only', () => {
    expect(followMarkdown('pdf-to-md')).toBe(true);
    expect(followPdf('pdf-to-md')).toBe(false);
    expect(publishesFromPdf('pdf-to-md')).toBe(true);
    expect(publishesFromMd('pdf-to-md')).toBe(false);
  });

  it('md-to-pdf publishes from markdown only', () => {
    expect(followMarkdown('md-to-pdf')).toBe(false);
    expect(followPdf('md-to-pdf')).toBe(true);
    expect(publishesFromPdf('md-to-pdf')).toBe(false);
    expect(publishesFromMd('md-to-pdf')).toBe(true);
  });

  it('none publishes from neither pane', () => {
    expect(followMarkdown('none')).toBe(false);
    expect(followPdf('none')).toBe(false);
    expect(publishesFromPdf('none')).toBe(false);
    expect(publishesFromMd('none')).toBe(false);
  });

  it('persists last mode and rejects garbage', () => {
    const map = new Map<string, string>();
    const storage = {
      getItem: (k: string) => (map.has(k) ? map.get(k)! : null),
      setItem: (k: string, v: string) => {
        map.set(k, v);
      },
    };
    writeStoredPageSyncMode('md-to-pdf', storage);
    expect(readStoredPageSyncMode(storage)).toBe('md-to-pdf');
    map.set('eq-page-sync-mode', 'both');
    expect(readStoredPageSyncMode(storage)).toBe('pdf-to-md');
  });
});
