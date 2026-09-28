/** SPEC-151 — page health aggregation helpers. */

export type StageStatus = "pending" | "running" | "ok" | "failed" | "skipped";

export interface PageHealthTile {
  page_number: number;
  parse: StageStatus;
  figures: StageStatus;
  entities: StageStatus;
}

export function worstStatus(...statuses: StageStatus[]): StageStatus {
  const rank: Record<StageStatus, number> = {
    failed: 4,
    running: 3,
    pending: 2,
    skipped: 1,
    ok: 0,
  };
  return statuses.reduce((a, b) => (rank[a] >= rank[b] ? a : b));
}

export function pageWorst(page: PageHealthTile): StageStatus {
  return worstStatus(page.parse, page.figures, page.entities);
}

export function countFailed(pages: PageHealthTile[]): number {
  return pages.filter((p) => pageWorst(p) === "failed").length;
}

export function failedPageNumbers(pages: PageHealthTile[]): number[] {
  return pages
    .filter((p) => pageWorst(p) === "failed")
    .map((p) => p.page_number);
}

export type StageKey = "parse" | "figures" | "entities";

/** Fraction of pages that completed a stage (ok or skipped). */
export function stageCompletionRatio(
  pages: PageHealthTile[],
  stage: StageKey,
): { done: number; total: number; running: number; failed: number } {
  const total = pages.length;
  let done = 0;
  let running = 0;
  let failed = 0;
  for (const p of pages) {
    const s = p[stage];
    if (s === "ok" || s === "skipped") done += 1;
    else if (s === "running") running += 1;
    else if (s === "failed") failed += 1;
  }
  return { done, total, running, failed };
}

/**
 * Progressive reprocess summary for a page set (usually the scoped selection
 * or all in-flight pages).
 */
export function progressiveReprocessStats(
  pages: PageHealthTile[],
  scopePages?: number[],
): {
  scoped: PageHealthTile[];
  running: number;
  done: number;
  failed: number;
  pending: number;
  total: number;
  ratio: number;
  activeStage: StageKey | null;
} {
  const set =
    scopePages && scopePages.length > 0 ? new Set(scopePages) : null;
  const scoped = set
    ? pages.filter((p) => set.has(p.page_number))
    : pages.filter((p) => {
        const w = pageWorst(p);
        return w === "running" || w === "pending" || w === "failed";
      });
  let running = 0;
  let done = 0;
  let failed = 0;
  let pending = 0;
  for (const p of scoped) {
    const w = pageWorst(p);
    if (w === "running") running += 1;
    else if (w === "ok" || w === "skipped") done += 1;
    else if (w === "failed") failed += 1;
    else pending += 1;
  }
  const total = scoped.length;
  const ratio = total === 0 ? 0 : done / total;
  let activeStage: StageKey | null = null;
  for (const stage of ["parse", "figures", "entities"] as StageKey[]) {
    if (scoped.some((p) => p[stage] === "running")) {
      activeStage = stage;
      break;
    }
  }
  return { scoped, running, done, failed, pending, total, ratio, activeStage };
}
