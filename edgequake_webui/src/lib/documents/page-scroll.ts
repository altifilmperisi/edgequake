/**
 * SPEC-143 — Pure page reading-line math + pane-local scroll helpers.
 *
 * Sync unit is the page. Both panes align the active page to the top of their
 * own scrollport. No scrollIntoView (that moves ancestors).
 */

/** Default inset: peek of the next page must not advance the active page. */
export const READING_LINE_INSET_PX = 8;

/**
 * Greatest page whose start offset is <= scrollTop + inset.
 * Returns 1 when starts is empty or no page has started yet.
 *
 * When `maxScroll` is provided and the pane is pinned at the bottom,
 * returns the last page (last page often cannot reach the reading line).
 */
export function pageAtReadingLine(
  starts: ReadonlyMap<number, number> | ReadonlyArray<readonly [number, number]>,
  scrollTop: number,
  insetPx: number = READING_LINE_INSET_PX,
  maxScroll?: number,
): number {
  const entries = normalizeStarts(starts);
  if (entries.length === 0) return 1;
  if (
    maxScroll != null &&
    Number.isFinite(maxScroll) &&
    maxScroll > 0 &&
    scrollTop >= maxScroll - 1
  ) {
    return entries[entries.length - 1]![0];
  }
  const line = scrollTop + insetPx;
  let best = entries[0]![0];
  let bestStart = entries[0]![1];
  for (const [page, start] of entries) {
    if (start > line) break;
    // Strictly later start wins; duplicate offsets keep the first (lower) page.
    if (start > bestStart) {
      best = page;
      bestStart = start;
    }
  }
  return best;
}

/**
 * scrollTop that places `page` at the top of the pane, clamped to [0, maxScroll].
 */
export function scrollTopForPage(
  starts: ReadonlyMap<number, number> | ReadonlyArray<readonly [number, number]>,
  page: number,
  maxScroll: number = Number.POSITIVE_INFINITY,
): number {
  const entries = normalizeStarts(starts);
  if (entries.length === 0) return 0;
  const target = Math.max(1, Math.floor(page));
  let start = entries[0]![1];
  for (const [p, s] of entries) {
    if (p === target) {
      start = s;
      break;
    }
    if (p < target) start = s;
  }
  const max = Number.isFinite(maxScroll) ? Math.max(0, maxScroll) : start;
  return Math.min(Math.max(0, start), max);
}

function normalizeStarts(
  starts: ReadonlyMap<number, number> | ReadonlyArray<readonly [number, number]>,
): Array<[number, number]> {
  const raw: Array<[number, number]> = Array.isArray(starts)
    ? starts.map(([p, s]) => [p, s] as [number, number])
    : Array.from(starts.entries());
  // Ascending by start offset; for duplicate offsets keep lower page first.
  raw.sort((a, b) => a[1] - b[1] || a[0] - b[0]);
  return raw;
}

/**
 * Offset of `el` relative to `root`'s content top (stable under nested layout).
 */
export function offsetTopWithin(root: HTMLElement, el: HTMLElement): number {
  const rootRect = root.getBoundingClientRect();
  const elRect = el.getBoundingClientRect();
  return elRect.top - rootRect.top + root.scrollTop;
}

/**
 * Collect page → content offset for `[data-eq-page]` or `[data-page]` sheets.
 */
export function collectPageStarts(
  root: HTMLElement,
  selector: string,
  attr: string = 'data-eq-page',
): Map<number, number> {
  const map = new Map<number, number>();
  root.querySelectorAll<HTMLElement>(selector).forEach((el) => {
    const raw = el.getAttribute(attr);
    const page = raw ? parseInt(raw, 10) : NaN;
    if (!Number.isFinite(page) || page < 1) return;
    if (map.has(page)) return; // first wins
    map.set(page, offsetTopWithin(root, el));
  });
  return map;
}

export interface ScrollPaneResult {
  scrolled: boolean;
  /** True when suppress should be held until scrollend / fallback. */
  pending: boolean;
}

/**
 * Set scrollTop on `root` only. Forces instant behavior for this write.
 * Returns whether a scroll was applied.
 */
export function scrollPaneTo(
  root: HTMLElement,
  top: number,
  epsilonPx: number = 2,
): ScrollPaneResult {
  const maxScroll = Math.max(0, root.scrollHeight - root.clientHeight);
  const target = Math.min(Math.max(0, top), maxScroll);
  if (Math.abs(root.scrollTop - target) <= epsilonPx) {
    return { scrolled: false, pending: false };
  }
  const prevBehavior = root.style.scrollBehavior;
  root.style.scrollBehavior = 'auto';
  root.scrollTop = target;
  // Restore on next frame so CSS classes are not permanently overridden.
  requestAnimationFrame(() => {
    root.style.scrollBehavior = prevBehavior;
  });
  return { scrolled: true, pending: true };
}

const SUPPRESS_FALLBACK_MS = 150;

/**
 * Hold a suppress flag across programmatic scroll; clear on scrollend or fallback.
 * Prefer native `scrollend` (baseline); debounce scroll events as fallback.
 */
export function beginScrollSuppress(
  root: HTMLElement,
  flag: { current: boolean },
): void {
  flag.current = true;
  let cleared = false;
  let timer = 0;

  const clear = () => {
    if (cleared) return;
    cleared = true;
    flag.current = false;
    root.removeEventListener('scrollend', onEnd);
    root.removeEventListener('scroll', onScroll);
    if (timer) window.clearTimeout(timer);
  };

  const armFallback = () => {
    if (timer) window.clearTimeout(timer);
    timer = window.setTimeout(clear, SUPPRESS_FALLBACK_MS);
  };

  const onEnd = () => clear();
  const onScroll = () => armFallback();

  root.addEventListener('scrollend', onEnd, { once: true });
  root.addEventListener('scroll', onScroll, { passive: true });
  armFallback();
}
