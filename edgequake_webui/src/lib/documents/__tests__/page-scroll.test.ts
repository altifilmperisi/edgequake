/**
 * SPEC-143 — Reading-line math (U-143 scroll helpers).
 */

import { describe, expect, it } from 'bun:test';
import {
  pageAtReadingLine,
  READING_LINE_INSET_PX,
  scrollTopForPage,
} from '../page-scroll';

/** Page starts: page 1@0, 2@800, 3@1600, 4@2400 */
const STARTS: Array<[number, number]> = [
  [1, 0],
  [2, 800],
  [3, 1600],
  [4, 2400],
];

describe('pageAtReadingLine', () => {
  it('returns 1 for empty starts', () => {
    expect(pageAtReadingLine([], 0)).toBe(1);
  });

  it('stays on page 1 at scrollTop 0', () => {
    expect(pageAtReadingLine(STARTS, 0)).toBe(1);
  });

  it('does not advance on a peek smaller than the inset', () => {
    // Next page top is 9px below viewport top → still page 1
    expect(pageAtReadingLine(STARTS, 800 - READING_LINE_INSET_PX - 1)).toBe(1);
  });

  it('advances when the next page reaches the reading line', () => {
    expect(pageAtReadingLine(STARTS, 800 - READING_LINE_INSET_PX)).toBe(2);
  });

  it('selects last page when scrolled past its start', () => {
    expect(pageAtReadingLine(STARTS, 2400)).toBe(4);
    expect(pageAtReadingLine(STARTS, 5000)).toBe(4);
  });

  it('keeps the first page when offsets are duplicated', () => {
    const dup: Array<[number, number]> = [
      [1, 0],
      [2, 100],
      [3, 100],
    ];
    expect(pageAtReadingLine(dup, 100)).toBe(2);
  });

  it('accepts a Map', () => {
    const map = new Map(STARTS);
    expect(pageAtReadingLine(map, 1600)).toBe(3);
  });

  it('selects last page when pinned at maxScroll', () => {
    // Page 4 starts at 2400 but viewport cannot scroll past 2000.
    expect(pageAtReadingLine(STARTS, 2000, READING_LINE_INSET_PX, 2000)).toBe(4);
  });
});

describe('scrollTopForPage', () => {
  it('returns 0 for empty starts', () => {
    expect(scrollTopForPage([], 2)).toBe(0);
  });

  it('returns exact start for known page', () => {
    expect(scrollTopForPage(STARTS, 3)).toBe(1600);
  });

  it('clamps to maxScroll', () => {
    expect(scrollTopForPage(STARTS, 4, 1000)).toBe(1000);
  });

  it('clamps missing high page to last known start (then max)', () => {
    expect(scrollTopForPage(STARTS, 99)).toBe(2400);
  });

  it('clamps page < 1 to first start', () => {
    expect(scrollTopForPage(STARTS, 0)).toBe(0);
  });
});
