/**
 * SPEC-155 — Documents intake strip: height budget, Working-section summary,
 * and collapse persistence.
 *
 * Dropzone + Active runs share one capped strip so the inventory table always
 * keeps a usable floor (header + ~4 rows).
 */

import { stageDisplayName } from "@/lib/pipeline/ingestion-run-view";
import type { IngestionRunView } from "@/lib/pipeline/ingestion-run-view";

/** Shared max height for the intake strip (dropzone + feedback). */
export const INTAKE_STRIP_MAX_DVH = 40;
/** Absolute px floor at large viewports (24rem). */
export const INTAKE_STRIP_MAX_REM = 24;
/** Inventory section min height: header + ~4 rows (~14rem). */
export const INVENTORY_MIN_PX = 224;
/** Row height used for budget math / E2E floor checks. */
export const INVENTORY_ROW_PX = 44;
/** Rows we guarantee visible at 720px viewport. */
export const INVENTORY_MIN_ROWS = 4;

/**
 * Tailwind-friendly max-height: `min(40dvh, 24rem)`.
 * Applied as inline style so the budget is one SSOT constant.
 */
export function intakeStripMaxHeightCss(): string {
  return `min(${INTAKE_STRIP_MAX_DVH}dvh, ${INTAKE_STRIP_MAX_REM}rem)`;
}

/**
 * At a given viewport height, how many inventory rows remain after chrome +
 * a fully-expanded intake strip? Used by unit tests / layout contracts.
 */
export function inventoryRowsAtViewport(opts: {
  viewportPx: number;
  chromePx: number;
  /** Cap the strip at the CSS budget (dvh / rem). */
  stripConsumedPx?: number;
}): number {
  const stripBudgetPx = Math.min(
    (INTAKE_STRIP_MAX_DVH / 100) * opts.viewportPx,
    INTAKE_STRIP_MAX_REM * 16,
  );
  const stripPx = Math.min(
    opts.stripConsumedPx ?? stripBudgetPx,
    stripBudgetPx,
  );
  const remaining = opts.viewportPx - opts.chromePx - stripPx;
  const usable = Math.max(0, remaining);
  // Inventory also has its own min-height floor.
  const inventoryH = Math.max(usable, INVENTORY_MIN_PX);
  return Math.floor(inventoryH / INVENTORY_ROW_PX);
}

export const WORKING_COLLAPSE_STORAGE_KEY =
  "edgequake.documents.intakeWorkingCollapsed";

/**
 * Density-first: missing key ⇒ collapsed (Documents keep the viewport).
 * Explicit `"0"` ⇒ expanded; `"1"` ⇒ collapsed.
 */
export function readWorkingCollapsed(storage?: Storage | null): boolean {
  try {
    const store = storage ?? (typeof localStorage !== "undefined" ? localStorage : null);
    if (!store) return true;
    const raw = store.getItem(WORKING_COLLAPSE_STORAGE_KEY);
    if (raw === "0") return false;
    return true;
  } catch {
    return true;
  }
}

export function writeWorkingCollapsed(
  collapsed: boolean,
  storage?: Storage | null,
): void {
  try {
    const store = storage ?? (typeof localStorage !== "undefined" ? localStorage : null);
    if (!store) return;
    store.setItem(WORKING_COLLAPSE_STORAGE_KEY, collapsed ? "1" : "0");
  } catch {
    // ignore quota / private-mode failures
  }
}

export interface WorkingRunsSummary {
  /** One-line text for the collapse toggle. */
  text: string;
  /** Average progress 0–1 across runs that report progress01; null if none. */
  avgProgress01: number | null;
  workingCount: number;
  queuedCount: number;
  stoppingCount: number;
  cancelledCount: number;
}

type RunLike = Pick<
  IngestionRunView,
  "filename" | "stage" | "stageStatus" | "progress01" | "sourceType"
>;

function isCancelled(run: RunLike): boolean {
  return run.stage === "cancelled" || run.stageStatus === "cancelled";
}

function isStopping(run: RunLike): boolean {
  return (
    !isCancelled(run) &&
    (run.stage === "stopping" || run.stageStatus === "stopping")
  );
}

function isQueued(run: RunLike): boolean {
  return (
    !isCancelled(run) &&
    !isStopping(run) &&
    (run.stageStatus === "pending" ||
      run.stage === "queued" ||
      run.stage === "cleaning")
  );
}

function isWorking(run: RunLike): boolean {
  return (
    !isCancelled(run) &&
    !isStopping(run) &&
    !isQueued(run) &&
    run.stageStatus === "active"
  );
}

function pctLabel(progress01: number | undefined): string | null {
  if (typeof progress01 !== "number" || Number.isNaN(progress01)) return null;
  return `${Math.round(Math.min(1, Math.max(0, progress01)) * 100)}%`;
}

/**
 * One-line summary for the Working section collapse toggle.
 *
 * - 1 run: `file.pdf · Prepare 4%`
 * - N runs: `3 working, 2 queued · avg 31%`
 * - Stopping / cancelled only: matching section title language
 */
export function summarizeWorkingRuns(runs: RunLike[]): WorkingRunsSummary {
  const workingCount = runs.filter(isWorking).length;
  const queuedCount = runs.filter(isQueued).length;
  const stoppingCount = runs.filter(isStopping).length;
  const cancelledCount = runs.filter(isCancelled).length;

  const withPct = runs
    .map((r) => r.progress01)
    .filter((p): p is number => typeof p === "number" && !Number.isNaN(p));
  const avgProgress01 =
    withPct.length > 0
      ? withPct.reduce((a, b) => a + b, 0) / withPct.length
      : null;

  if (runs.length === 0) {
    return {
      text: "No active runs",
      avgProgress01: null,
      workingCount: 0,
      queuedCount: 0,
      stoppingCount: 0,
      cancelledCount: 0,
    };
  }

  if (runs.length === 1) {
    const run = runs[0]!;
    const stage = stageDisplayName(run.stage, run.sourceType);
    const pct = pctLabel(run.progress01);
    const text = pct
      ? `${run.filename} · ${stage} ${pct}`
      : `${run.filename} · ${stage}`;
    return {
      text,
      avgProgress01,
      workingCount,
      queuedCount,
      stoppingCount,
      cancelledCount,
    };
  }

  // Multi-run: prefer live counts; fall back to stopping/cancelled language.
  const parts: string[] = [];
  if (workingCount > 0) {
    parts.push(`${workingCount} working`);
  }
  if (queuedCount > 0) {
    parts.push(`${queuedCount} queued`);
  }
  if (stoppingCount > 0) {
    parts.push(`${stoppingCount} stopping`);
  }
  if (cancelledCount > 0) {
    parts.push(`${cancelledCount} cancelled`);
  }
  if (parts.length === 0) {
    parts.push(`${runs.length} runs`);
  }
  const avg =
    avgProgress01 != null
      ? ` · avg ${Math.round(avgProgress01 * 100)}%`
      : "";
  return {
    text: `${parts.join(", ")}${avg}`,
    avgProgress01,
    workingCount,
    queuedCount,
    stoppingCount,
    cancelledCount,
  };
}
