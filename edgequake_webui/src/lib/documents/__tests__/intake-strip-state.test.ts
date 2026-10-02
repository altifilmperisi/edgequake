import { describe, expect, test } from "bun:test";
import {
  INTAKE_STRIP_MAX_DVH,
  INTAKE_STRIP_MAX_REM,
  INVENTORY_MIN_PX,
  INVENTORY_MIN_ROWS,
  WORKING_COLLAPSE_STORAGE_KEY,
  inventoryRowsAtViewport,
  intakeStripMaxHeightCss,
  readWorkingCollapsed,
  summarizeWorkingRuns,
  writeWorkingCollapsed,
} from "../intake-strip-state";

function run(
  partial: Partial<Parameters<typeof summarizeWorkingRuns>[0][number]> & {
    filename: string;
    stage: string;
    stageStatus: Parameters<typeof summarizeWorkingRuns>[0][number]["stageStatus"];
  },
) {
  return {
    sourceType: "pdf" as const,
    progress01: undefined,
    ...partial,
  };
}

describe("intake-strip-state", () => {
  test("intakeStripMaxHeightCss is min(40dvh, 24rem)", () => {
    expect(intakeStripMaxHeightCss()).toBe(
      `min(${INTAKE_STRIP_MAX_DVH}dvh, ${INTAKE_STRIP_MAX_REM}rem)`,
    );
  });

  test("at 720px viewport, strip + chrome leave ≥4 inventory rows", () => {
    // chrome ≈ header + search/filters (~150px); strip fully consumes budget
    const rows = inventoryRowsAtViewport({
      viewportPx: 720,
      chromePx: 150,
    });
    expect(rows).toBeGreaterThanOrEqual(INVENTORY_MIN_ROWS);
    // Inventory floor alone guarantees 224/44 = 5 rows
    expect(Math.floor(INVENTORY_MIN_PX / 44)).toBeGreaterThanOrEqual(
      INVENTORY_MIN_ROWS,
    );
  });

  test("summarizeWorkingRuns: single run with pct", () => {
    const summary = summarizeWorkingRuns([
      run({
        filename: "algo_2608.pdf",
        stage: "converting",
        stageStatus: "active",
        progress01: 0.04,
      }),
    ]);
    expect(summary.text).toBe("algo_2608.pdf · Converting PDF 4%");
    expect(summary.avgProgress01).toBeCloseTo(0.04);
    expect(summary.workingCount).toBe(1);
  });

  test("summarizeWorkingRuns: single run without pct", () => {
    const summary = summarizeWorkingRuns([
      run({
        filename: "notes.md",
        stage: "queued",
        stageStatus: "pending",
        sourceType: "markdown",
      }),
    ]);
    expect(summary.text).toBe("notes.md · Queued");
    expect(summary.avgProgress01).toBeNull();
    expect(summary.queuedCount).toBe(1);
  });

  test("summarizeWorkingRuns: multi-run averages", () => {
    const summary = summarizeWorkingRuns([
      run({
        filename: "a.pdf",
        stage: "extracting",
        stageStatus: "active",
        progress01: 0.2,
      }),
      run({
        filename: "b.pdf",
        stage: "extracting",
        stageStatus: "active",
        progress01: 0.4,
      }),
      run({
        filename: "c.pdf",
        stage: "queued",
        stageStatus: "pending",
      }),
      run({
        filename: "d.pdf",
        stage: "queued",
        stageStatus: "pending",
      }),
    ]);
    expect(summary.text).toBe("2 working, 2 queued · avg 30%");
    expect(summary.workingCount).toBe(2);
    expect(summary.queuedCount).toBe(2);
    expect(summary.avgProgress01).toBeCloseTo(0.3);
  });

  test("summarizeWorkingRuns: stopping / cancelled only", () => {
    const summary = summarizeWorkingRuns([
      run({
        filename: "x.pdf",
        stage: "stopping",
        stageStatus: "stopping",
      }),
      run({
        filename: "y.pdf",
        stage: "cancelled",
        stageStatus: "cancelled",
      }),
    ]);
    expect(summary.text).toBe("1 stopping, 1 cancelled");
    expect(summary.avgProgress01).toBeNull();
  });

  test("summarizeWorkingRuns: empty", () => {
    expect(summarizeWorkingRuns([])).toEqual({
      text: "No active runs",
      avgProgress01: null,
      workingCount: 0,
      queuedCount: 0,
      stoppingCount: 0,
      cancelledCount: 0,
    });
  });

  test("collapse persistence: default collapsed; round-trip + bad storage", () => {
    const mem = new Map<string, string>();
    const storage = {
      getItem: (k: string) => mem.get(k) ?? null,
      setItem: (k: string, v: string) => {
        mem.set(k, v);
      },
      removeItem: (k: string) => {
        mem.delete(k);
      },
    } as Storage;

    // Density-first: missing key ⇒ collapsed
    expect(readWorkingCollapsed(storage)).toBe(true);
    writeWorkingCollapsed(false, storage);
    expect(mem.get(WORKING_COLLAPSE_STORAGE_KEY)).toBe("0");
    expect(readWorkingCollapsed(storage)).toBe(false);
    writeWorkingCollapsed(true, storage);
    expect(mem.get(WORKING_COLLAPSE_STORAGE_KEY)).toBe("1");
    expect(readWorkingCollapsed(storage)).toBe(true);

    const throwing = {
      getItem: () => {
        throw new Error("blocked");
      },
      setItem: () => {
        throw new Error("blocked");
      },
      removeItem: () => {
        throw new Error("blocked");
      },
    } as unknown as Storage;
    expect(readWorkingCollapsed(throwing)).toBe(true);
    expect(() => writeWorkingCollapsed(true, throwing)).not.toThrow();
  });
});
