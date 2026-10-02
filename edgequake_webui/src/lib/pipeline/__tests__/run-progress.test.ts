import { describe, expect, it } from "vitest";
import {
  clampMonotonic,
  findPhase,
  formatPhaseCaption,
  phaseFill01,
  synthesizeFromLegacy,
  type RunProgress,
} from "../run-progress";

function pagesThenFigures(): RunProgress {
  return {
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
  };
}

describe("phaseFill01", () => {
  it("does not collapse Prepare when figures start after pages", () => {
    const ledger = pagesThenFigures();
    const fill = phaseFill01(findPhase(ledger, "prepare"));
    // pages 1.0 + figures ~0.083 → ~0.54
    expect(fill).toBeGreaterThan(0.4);
    expect(fill).toBeLessThan(0.99);
  });

  it("is 1.0 for a done phase", () => {
    const ledger: RunProgress = {
      seq: 1,
      phases: [
        {
          id: "prepare",
          state: "done",
          tasks: [{ id: "pages", unit: "pages", done: 10, total: 10 }],
        },
        { id: "extract", state: "active", tasks: [] },
        { id: "materialize", state: "pending", tasks: [] },
      ],
    };
    expect(phaseFill01(findPhase(ledger, "prepare"))).toBe(1);
  });
});

describe("formatPhaseCaption", () => {
  it("names pages and figures independently", () => {
    expect(formatPhaseCaption(pagesThenFigures())).toBe(
      "Prepare · pages 92/92 · figures 1/12",
    );
  });

  it("uses completed chunks, not last-started index", () => {
    const ledger: RunProgress = {
      seq: 3,
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
    };
    expect(formatPhaseCaption(ledger)).toBe(
      "Extract · 61/92 chunks, 12 in flight",
    );
  });
});

describe("clampMonotonic", () => {
  it("rejects a stale poll that regresses done", () => {
    const prev: RunProgress = {
      seq: 5,
      phases: [
        { id: "prepare", state: "done", tasks: [] },
        {
          id: "extract",
          state: "active",
          tasks: [{ id: "chunks", unit: "chunks", done: 40, total: 100 }],
        },
        { id: "materialize", state: "pending", tasks: [] },
      ],
    };
    const next: RunProgress = {
      seq: 5,
      phases: [
        { id: "prepare", state: "done", tasks: [] },
        {
          id: "extract",
          state: "active",
          tasks: [{ id: "chunks", unit: "chunks", done: 10, total: 100 }],
        },
        { id: "materialize", state: "pending", tasks: [] },
      ],
    };
    const merged = clampMonotonic(prev, next)!;
    expect(findPhase(merged, "extract")!.tasks[0].done).toBe(40);
  });
});

describe("synthesizeFromLegacy", () => {
  it("never invents counters from a free-text message", () => {
    const synth = synthesizeFromLegacy({
      stage: "extracting",
      stageProgress01: 0.5,
      counts: { unit: "chunks", current: 5, total: 10 },
    });
    expect(findPhase(synth!, "extract")!.tasks[0].done).toBe(5);
    expect(findPhase(synth!, "prepare")!.state).toBe("done");
  });
});

describe("clampMonotonic malformed wire data", () => {
  it("does not throw when a ledger lacks phases (regression: reading 'find')", () => {
    const bad = { seq: 3 } as unknown as RunProgress;
    expect(() => clampMonotonic(bad, pagesThenFigures())).not.toThrow();
    expect(clampMonotonic(bad, pagesThenFigures())?.phases).toHaveLength(3);
    expect(clampMonotonic(pagesThenFigures(), bad)?.seq).toBe(2);
    expect(clampMonotonic(bad, bad)).toBeNull();
  });

  it("tolerates phases without tasks arrays", () => {
    const prev = pagesThenFigures();
    const next = {
      seq: 5,
      phases: [{ id: "prepare", state: "active" }],
    } as unknown as RunProgress;
    expect(() => clampMonotonic(prev, next)).not.toThrow();
  });
});
