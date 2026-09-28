import { describe, expect, it } from "vitest";
import { formatPageRange, parsePageRange } from "../page-range";
import { effectiveStages, stageClosure } from "../reprocess-stages";
import {
  countFailed,
  failedPageNumbers,
  progressiveReprocessStats,
  stageCompletionRatio,
  worstStatus,
} from "../page-health";
import { resolveDetailLifecycle } from "../detail-lifecycle";

describe("parsePageRange", () => {
  it("parses mixed ranges", () => {
    expect(parsePageRange("1-3,7")).toEqual([1, 2, 3, 7]);
  });
  it("formats compact ranges", () => {
    expect(formatPageRange([1, 2, 3, 7])).toBe("1-3,7");
  });
});

describe("reprocess stages", () => {
  it("parse closes all", () => {
    expect(stageClosure("parse")).toEqual(["parse", "figures", "entities"]);
  });
  it("entities alone", () => {
    expect(effectiveStages(["entities"])).toEqual(["entities"]);
  });
});

describe("page health", () => {
  it("worst is failed", () => {
    expect(worstStatus("ok", "failed", "pending")).toBe("failed");
  });
  it("counts failed pages", () => {
    expect(
      countFailed([
        { page_number: 1, parse: "ok", figures: "ok", entities: "ok" },
        { page_number: 2, parse: "failed", figures: "ok", entities: "ok" },
      ]),
    ).toBe(1);
  });
  it("lists failed page numbers", () => {
    expect(
      failedPageNumbers([
        { page_number: 1, parse: "ok", figures: "ok", entities: "ok" },
        { page_number: 2, parse: "failed", figures: "pending", entities: "ok" },
        { page_number: 3, parse: "ok", figures: "ok", entities: "failed" },
      ]),
    ).toEqual([2, 3]);
  });

  it("stageCompletionRatio tracks done/running", () => {
    const pages = [
      { page_number: 1, parse: "ok" as const, figures: "ok" as const, entities: "ok" as const },
      {
        page_number: 2,
        parse: "running" as const,
        figures: "pending" as const,
        entities: "pending" as const,
      },
    ];
    expect(stageCompletionRatio(pages, "parse")).toEqual({
      done: 1,
      total: 2,
      running: 1,
      failed: 0,
    });
  });

  it("progressiveReprocessStats scopes selection", () => {
    const pages = [
      { page_number: 1, parse: "ok" as const, figures: "ok" as const, entities: "ok" as const },
      {
        page_number: 2,
        parse: "running" as const,
        figures: "pending" as const,
        entities: "pending" as const,
      },
      {
        page_number: 3,
        parse: "pending" as const,
        figures: "pending" as const,
        entities: "pending" as const,
      },
    ];
    const stats = progressiveReprocessStats(pages, [2, 3]);
    expect(stats.total).toBe(2);
    expect(stats.running).toBe(1);
    expect(stats.pending).toBe(1);
    expect(stats.activeStage).toBe("parse");
    expect(stats.ratio).toBe(0);
  });
});

describe("detail lifecycle", () => {
  it("prefers converting progress over bare processing", () => {
    const life = resolveDetailLifecycle({
      status: "processing",
      display_status: "converting",
      current_stage: "converting",
      track_id: "t1",
      progress_counts: { unit: "pages", current: 3, total: 27 },
      page_count: 27,
    });
    expect(life.kind).toBe("converting");
    expect(life.label).toBe("Converting · 3/27");
    expect(life.showSpinner).toBe(true);
    expect(life.canCancel).toBe(true);
    expect(life.canReprocessPages).toBe(false);
    expect(life.emptyMarkdownMode).toBe("in_flight");
  });

  it("ready allows page reprocess", () => {
    const life = resolveDetailLifecycle({
      status: "completed",
      display_status: "completed",
      ui_phase: "terminal",
    });
    expect(life.kind).toBe("ready");
    expect(life.canReprocess).toBe(true);
    expect(life.canReprocessPages).toBe(true);
    expect(life.emptyMarkdownMode).toBe("missing");
  });

  it("failed maps empty markdown to failed mode", () => {
    const life = resolveDetailLifecycle({
      status: "failed",
      ui_phase: "terminal",
    });
    expect(life.kind).toBe("failed");
    expect(life.emptyMarkdownMode).toBe("failed");
    expect(life.canReprocessPages).toBe(true);
  });
});
