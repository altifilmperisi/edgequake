import { describe, expect, it } from "vitest";
import {
  MIN_VISIBLE_FILL_PCT,
  buildPhaseSegments,
  phaseSegmentStatus,
  resolveCaptionProgress,
  visibleFillPct,
} from "../phase-segments";

const run = (stage: string, stageStatus = "active") =>
  ({ stage, stageStatus }) as Parameters<typeof buildPhaseSegments>[0];

describe("visibleFillPct", () => {
  it("keeps a visible sliver for tiny real progress", () => {
    expect(visibleFillPct(1)).toBe(MIN_VISIBLE_FILL_PCT);
  });
  it("is 0 for zero / invalid and clamps at 100", () => {
    expect(visibleFillPct(0)).toBe(0);
    expect(visibleFillPct(Number.NaN)).toBe(0);
    expect(visibleFillPct(250)).toBe(100);
  });
  it("passes ordinary values through", () => {
    expect(visibleFillPct(42)).toBe(42);
  });
});

describe("buildPhaseSegments", () => {
  it("marks earlier phases done, the active one filled, later ones empty", () => {
    const segs = buildPhaseSegments(run("extracting"), 1);
    expect(segs.map((s) => s.status)).toEqual([
      "done",
      "done",
      "active",
      "pending",
    ]);
    expect(segs.map((s) => s.fillPct)).toEqual([100, 100, MIN_VISIBLE_FILL_PCT, 0]);
    expect(segs.some((s) => s.indeterminate)).toBe(false);
  });

  it("is indeterminate (not a fake number) without a determinate stage pct", () => {
    const active = buildPhaseSegments(run("extracting"), undefined).find(
      (s) => s.status === "active",
    );
    expect(active).toMatchObject({ indeterminate: true, fillPct: 0 });
  });

  it("paints the failed phase solid", () => {
    const segs = buildPhaseSegments(run("extracting", "failed"), 40);
    expect(segs[2]).toMatchObject({ status: "failed", fillPct: 100 });
  });

  it("keeps Prepare fill high when ledger has pages done + figures mid-flight", () => {
    const segs = buildPhaseSegments(
      {
        stage: "converting",
        stageStatus: "active",
        runProgress: {
          seq: 2,
          phases: [
            {
              id: "prepare",
              state: "active",
              tasks: [
                { id: "pages", unit: "pages", done: 92, total: 92 },
                { id: "figures", unit: "figures", done: 1, total: 12 },
              ],
            },
            { id: "extract", state: "pending", tasks: [] },
            { id: "materialize", state: "pending", tasks: [] },
          ],
        },
      },
      8, // legacy figure-only pct that used to collapse Prepare
    );
    const prepare = segs.find((s) => s.phase === "prepare")!;
    expect(prepare.status).toBe("active");
    expect(prepare.fillPct).toBeGreaterThan(40);
    expect(prepare.summary).toContain("92 pages");
    expect(prepare.summary).toContain("1 figures");
  });

  it("uses completed chunks from the ledger, not a 92/92 last-started pct", () => {
    const segs = buildPhaseSegments(
      {
        stage: "extracting",
        stageStatus: "active",
        runProgress: {
          seq: 4,
          phases: [
            { id: "prepare", state: "done", tasks: [] },
            {
              id: "extract",
              state: "active",
              tasks: [
                {
                  id: "chunks",
                  unit: "chunks",
                  done: 61,
                  total: 92,
                  in_flight: 12,
                },
              ],
            },
            { id: "materialize", state: "pending", tasks: [] },
          ],
        },
      },
      99, // legacy fake from chunk_idx
    );
    const extract = segs.find((s) => s.phase === "extract")!;
    expect(extract.fillPct).toBeGreaterThan(60);
    expect(extract.fillPct).toBeLessThan(75);
  });
});

describe("phaseSegmentStatus", () => {
  it("orders phases relative to the active one", () => {
    expect(phaseSegmentStatus("admit", "prepare", false)).toBe("done");
    expect(phaseSegmentStatus("prepare", "prepare", false)).toBe("active");
    expect(phaseSegmentStatus("materialize", "prepare", false)).toBe("pending");
  });
});

describe("resolveCaptionProgress", () => {
  it("prefers the determinate stage percentage", () => {
    expect(resolveCaptionProgress({ stagePct: 1, overallPct: 30 })).toEqual({
      pct: 1,
      estimated: false,
    });
  });
  it("falls back to the overall estimate, flagged as estimated", () => {
    expect(resolveCaptionProgress({ stagePct: undefined, overallPct: 30 })).toEqual({
      pct: 30,
      estimated: true,
    });
  });
});
