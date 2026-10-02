/**
 * SPEC-155 — a "processing" document the server stopped talking about is
 * Stalled: never a live Active run, never "Working", always cancellable.
 */
import { describe, expect, it } from "vitest";
import { partitionActiveRuns } from "@/lib/pipeline/active-runs-partition";
import { buildIngestionRunView } from "@/lib/pipeline/ingestion-run-view";
import {
  resolvePipelineUiState,
  summarizePipelineDocuments,
} from "@/lib/pipeline/pipeline-document-state";
import type { Document } from "@/types";

const DAY = 24 * 3_600_000;
const ago = (ms: number) => new Date(Date.now() - ms).toISOString();

function doc(over: Partial<Document>): Document {
  return {
    id: "spec129-dual",
    title: "spec129-dual",
    status: "processing",
    current_stage: "preprocessing",
    stage_progress: 0.01,
    track_id: null,
    updated_at: ago(3 * DAY),
    created_at: ago(3 * DAY),
    ...over,
  } as Document;
}

describe("stalled run view", () => {
  it("marks a silent processing doc stalled (no track_id needed)", () => {
    const run = buildIngestionRunView(doc({}));
    expect(run?.stalledForMs).toBeGreaterThanOrEqual(3 * DAY - 5_000);
  });

  it("keeps fresh processing docs live", () => {
    const run = buildIngestionRunView(doc({ updated_at: ago(20_000) }));
    expect(run?.stalledForMs).toBeUndefined();
    expect(run?.stageStatus).toBe("active");
  });

  it("does not stall queued docs (waiting is not silence)", () => {
    const run = buildIngestionRunView(
      doc({ status: "pending", current_stage: "queued", track_id: "t1" }),
    );
    expect(run?.stalledForMs).toBeUndefined();
  });

  it("does not stall cancelled docs", () => {
    const run = buildIngestionRunView(
      doc({ status: "cancelled", current_stage: "cancelled" }),
    );
    expect(run?.stalledForMs).toBeUndefined();
  });
});

describe("stalled partition", () => {
  it("routes stalled runs to Needs attention, not the Active run", () => {
    const stalled = buildIngestionRunView(doc({}))!;
    const live = buildIngestionRunView(
      doc({ id: "live", updated_at: ago(10_000), track_id: "t2" }),
    )!;
    const { working, attention } = partitionActiveRuns([stalled, live]);
    expect(attention.map((r) => r.documentId)).toEqual(["spec129-dual"]);
    expect(working.map((r) => r.documentId)).toEqual(["live"]);
  });
});

describe("stalled pipeline summary", () => {
  it("does not count a stalled doc as Working", () => {
    const summary = summarizePipelineDocuments([doc({})]);
    expect(summary.activeCount).toBe(0);
    expect(summary.stalledDocs.map((d) => d.id)).toEqual(["spec129-dual"]);
  });

  it("surfaces stalled-only as stuck, not as Processing 1", () => {
    const ui = resolvePipelineUiState([doc({})], {
      is_busy: false,
      running_tasks: 0,
      queued_tasks: 0,
    });
    expect(ui.isActivelyProcessing).toBe(false);
    expect(ui.activeDocCount).toBe(0);
    expect(ui.isStuck).toBe(true);
    expect(ui.stuckDocs.map((d) => d.id)).toEqual(["spec129-dual"]);
    expect(ui.alertMode).toBe("stuck");
  });

  it("keeps real work Working next to a stalled doc (mixed)", () => {
    const ui = resolvePipelineUiState(
      [doc({}), doc({ id: "live", updated_at: ago(5_000), track_id: "t" })],
      { is_busy: true, running_tasks: 1, queued_tasks: 0 },
    );
    expect(ui.isActivelyProcessing).toBe(true);
    expect(ui.activeDocCount).toBe(1);
    expect(ui.stuckDocs.map((d) => d.id)).toContain("spec129-dual");
    expect(ui.alertMode).toBe("mixed");
  });
});
