/**
 * SPEC-143 — Observe markdown `[data-eq-page]` anchors; report active page.
 *
 * Uses reading-line math on pane scrollTop (not IntersectionObserver ratios).
 * Follows PDF/external drives via scrollPaneTo on the explicit scroll root.
 */

'use client';

import {
  beginScrollSuppress,
  collectPageStarts,
  pageAtReadingLine,
  scrollPaneTo,
  scrollTopForPage,
} from '@/lib/documents/page-scroll';
import { useEffect, useRef, useState } from 'react';

export interface UseMarkdownPageObserverOptions {
  /** Element that contains `[data-eq-page]` anchors (e.g. ContentRenderer root). */
  containerRef: React.RefObject<HTMLElement | null>;
  /**
   * Explicit scrollport. When omitted, walks ancestors for overflow-y auto|scroll.
   * Prefer marking `data-testid="md-scroll-container"`.
   */
  scrollRootRef?: React.RefObject<HTMLElement | null>;
  enabled: boolean;
  onPage: (page: number) => void;
  /** When set, scroll this page's `#eq-md-page-N` into the pane. */
  scrollToPage?: number | null;
  /** Skip scroll when driver is markdown itself. */
  skipScroll?: boolean;
  /** User gesture ownership (wheel / pointer on the markdown pane). */
  onGestureStart?: () => void;
  onGestureEnd?: () => void;
}

function resolveScrollRoot(
  el: HTMLElement | null,
  explicit: HTMLElement | null | undefined,
): HTMLElement | null {
  if (explicit) return explicit;
  let p: HTMLElement | null = el?.parentElement ?? null;
  while (p) {
    const style = window.getComputedStyle(p);
    if (/(auto|scroll)/.test(style.overflowY)) return p;
    p = p.parentElement;
  }
  return null;
}

function followPage(
  container: HTMLElement,
  root: HTMLElement,
  page: number,
  suppressRef: { current: boolean },
  lastReported: { current: number | null },
): boolean {
  // Offsets must be relative to the scrollport. Measuring from the content
  // box and assigning that number to the ancestor's scrollTop drifts by
  // whatever sits between them.
  let starts = collectPageStarts(root, '[data-eq-page]', 'data-eq-page');
  if (starts.size === 0 || !starts.has(page)) return false;

  const lastStart = Math.max(...starts.values());
  const needPad = Math.max(0, root.clientHeight - 48);
  if (root.scrollHeight < lastStart + needPad) {
    container.style.paddingBottom = `${needPad}px`;
    starts = collectPageStarts(root, '[data-eq-page]', 'data-eq-page');
    if (!starts.has(page)) return false;
  }

  const start = starts.get(page) ?? 0;
  // Anchor is in the DOM but the prose under it has not laid out yet.
  if (page > 1 && root.scrollHeight < start + 16) return false;

  const maxScroll = Math.max(0, root.scrollHeight - root.clientHeight);
  const top = scrollTopForPage(starts, page, maxScroll);
  const result = scrollPaneTo(root, top);
  if (result.pending) {
    beginScrollSuppress(root, suppressRef);
  }
  lastReported.current = page;
  return true;
}

export function useMarkdownPageObserver({
  containerRef,
  scrollRootRef,
  enabled,
  onPage,
  scrollToPage,
  skipScroll = false,
  onGestureStart,
  onGestureEnd,
}: UseMarkdownPageObserverOptions): void {
  const onPageRef = useRef(onPage);
  onPageRef.current = onPage;
  const onGestureStartRef = useRef(onGestureStart);
  onGestureStartRef.current = onGestureStart;
  const onGestureEndRef = useRef(onGestureEnd);
  onGestureEndRef.current = onGestureEnd;
  const lastReported = useRef<number | null>(null);
  const suppressRef = useRef(false);
  const rafRef = useRef<number | null>(null);
  const mdGestureRef = useRef(false);
  /** Re-run follow after a user gesture so a late layout shift can align. */
  const [gestureSettle, setGestureSettle] = useState(0);

  // User scroll → active page (reading line). Only emit during a user gesture
  // so mount / PDF-driven follow cannot overwrite a deeplink target with page 1.
  useEffect(() => {
    if (!enabled) return;
    const container = containerRef.current;
    if (!container) return;
    const root = resolveScrollRoot(container, scrollRootRef?.current);
    if (!root) return;

    const pick = () => {
      if (suppressRef.current) return;
      if (!mdGestureRef.current) return;
      const starts = collectPageStarts(root, '[data-eq-page]', 'data-eq-page');
      if (starts.size === 0) return;
      const maxScroll = Math.max(0, root.scrollHeight - root.clientHeight);
      const page = pageAtReadingLine(starts, root.scrollTop, undefined, maxScroll);
      if (page >= 1 && page !== lastReported.current) {
        lastReported.current = page;
        onPageRef.current(page);
      }
    };

    const onScroll = () => {
      if (rafRef.current != null) cancelAnimationFrame(rafRef.current);
      rafRef.current = requestAnimationFrame(() => {
        rafRef.current = null;
        pick();
      });
    };

    let endTimer: number | null = null;
    const markGesture = () => {
      if (endTimer != null) {
        window.clearTimeout(endTimer);
        endTimer = null;
      }
      mdGestureRef.current = true;
      onGestureStartRef.current?.();
    };
    const endGesture = () => {
      // Delay clear so rAF scroll pick can run after scrollend.
      if (endTimer != null) window.clearTimeout(endTimer);
      endTimer = window.setTimeout(() => {
        const wasUser = mdGestureRef.current;
        if (wasUser) pick();
        mdGestureRef.current = false;
        onGestureEndRef.current?.();
        endTimer = null;
        if (wasUser) setGestureSettle((n) => n + 1);
      }, 120);
    };

    root.addEventListener('scroll', onScroll, { passive: true });
    root.addEventListener('wheel', markGesture, { passive: true });
    root.addEventListener('pointerdown', markGesture, { passive: true });
    root.addEventListener('scrollend', endGesture);
    root.addEventListener('pointerup', endGesture, { passive: true });

    return () => {
      root.removeEventListener('scroll', onScroll);
      root.removeEventListener('wheel', markGesture);
      root.removeEventListener('pointerdown', markGesture);
      root.removeEventListener('scrollend', endGesture);
      root.removeEventListener('pointerup', endGesture);
      if (rafRef.current != null) cancelAnimationFrame(rafRef.current);
    };
  }, [containerRef, scrollRootRef, enabled]);

  // Programmatic scroll when PDF / external drives — retry until anchors mount.
  useEffect(() => {
    if (!enabled || skipScroll) return;
    if (scrollToPage == null || scrollToPage < 1) return;
    const container = containerRef.current;
    if (!container) return;
    const root = resolveScrollRoot(container, scrollRootRef?.current);
    if (!root) return;

    const align = () => {
      if (mdGestureRef.current) return false;
      return followPage(container, root, scrollToPage, suppressRef, lastReported);
    };

    let mo: MutationObserver | null = null;
    if (!align()) {
      mo = new MutationObserver(() => {
        if (align()) mo?.disconnect();
      });
      mo.observe(container, { childList: true, subtree: true });
    }
    // Images and lazy sections change height after the first align.
    let frame = 0;
    const ro = new ResizeObserver(() => {
      if (frame) cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        frame = 0;
        align();
      });
    });
    ro.observe(container);
    const giveUp = window.setTimeout(() => mo?.disconnect(), 5000);
    return () => {
      mo?.disconnect();
      ro.disconnect();
      if (frame) cancelAnimationFrame(frame);
      window.clearTimeout(giveUp);
    };
  }, [containerRef, scrollRootRef, enabled, scrollToPage, skipScroll, gestureSettle]);
}
