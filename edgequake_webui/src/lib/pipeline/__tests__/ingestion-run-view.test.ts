import { describe, expect, it } from "vitest";
import {
  buildIngestionRunView,
  formatQueueChrome,
  formatRunHeadline,
  mapWireStageToPhase,
  normalizeRunStage,
  parseCountsFromMessage,
  PHASE_STRIP_ORDER,
  resolveProgressCounts,
  shouldNestPdfPageMeter,
  shouldShowOverallMeter,
  selectPrimaryRun,
  buildIngestionRunViews,
  SERVER_STAGE_ORDER,
  stageDisplayName,
  stageStatusFor,
} from "@/lib/pipeline/ingestion-run-view";
import { buildStageTimeline } from "@/lib/pipeline/stage-timeline";
import type { Document } from "@/types";

function doc(partial: Partial<Document> & { id: string }): Document {
  return {
    title: partial.title ?? partial.file_name ?? partial.id,
    chunk_count: 0,
    ...partial,
  } as Document;
}

describe("ingestion-run-view", () => {
  it("a delete_failed doc is not an ingest run even with a stale track_id/stage", () => {
    const view = buildIngestionRunView(
      doc({
        id: "d-delete-failed",
        file_name: "half-deleted.pdf",
        status: "delete_failed",
        track_id: "t-old",
        current_stage: "materialize",
        stage_message: "Delete did not finish",
      }),
    );
    expect(view).toBeNull();
  });

  it("terminal status cancelled beats stale display_status extracting", () => {
    const view = buildIngestionRunView(
      doc({
        id: "d-cancel-lag",
        file_name: "ticket.pdf",
        status: "cancelled",
        current_stage: "cancelled",
        display_status: "extracting",
        ui_phase: "running",
        stage_progress: 0.99,
        track_id: "insert-cancelled",
      }),
    );
    expect(view?.stage).toBe("cancelled");
    expect(view?.stageStatus).toBe("cancelled");
    // Cancel honesty: do not invent 0% after cancel; freeze leaves progress unset.
    expect(view?.progress01).toBeUndefined();
  });

  it("places cleaning before queued in SERVER_STAGE_ORDER", () => {
    expect(SERVER_STAGE_ORDER.indexOf("cleaning")).toBe(0);
    expect(SERVER_STAGE_ORDER.indexOf("queued")).toBe(1);
    expect(stageDisplayName("cleaning")).toBe("Cleaning");
  });

  it("normalizes pending → queued and indexing → storing", () => {
    expect(normalizeRunStage("pending", "pending")).toBe("queued");
    expect(normalizeRunStage("indexing", "indexing")).toBe("storing");
  });

  it("builds cleaning admission run view", () => {
    const view = buildIngestionRunView(
      doc({
        id: "d-clean",
        file_name: "paper.pdf",
        status: "processing",
        current_stage: "cleaning",
        stage_message: "Removing prior knowledge graph data…",
        source_type: "pdf",
        track_id: "reprocess_batch",
      }),
    );
    expect(view?.stage).toBe("cleaning");
    expect(view?.stageStatus).toBe("pending");
    expect(stageStatusFor("cleaning", "processing")).toBe("pending");
  });

  it("parses chunk counts preferring chunk unit", () => {
    const c = parseCountsFromMessage("Extracting entities — chunk 42/351");
    expect(c).toEqual({ current: 42, total: 351, unit: "chunks" });
  });

  it("LAW-IS1: resolveProgressCounts prefers structured over message", () => {
    const c = resolveProgressCounts(
      { unit: "pages", current: 4, total: 9 },
      "chunk 1/99",
    );
    expect(c).toEqual({ current: 4, total: 9, unit: "pages" });
  });

  it("LAW-IS1: buildIngestionRunView uses progress_counts without N/M in message", () => {
    const view = buildIngestionRunView(
      doc({
        id: "d-ssot",
        file_name: "ticket.pdf",
        status: "processing",
        current_stage: "converting",
        stage_message: "Converting PDF to Markdown",
        source_type: "pdf",
        track_id: "t-ssot",
        progress_counts: { unit: "pages", current: 4, total: 9 },
      }),
    );
    expect(view?.counts).toEqual({ current: 4, total: 9, unit: "pages" });
    expect(shouldNestPdfPageMeter(view!)).toBe(false);
    expect(shouldShowOverallMeter(view!, false)).toBe(false);
  });

  it.each([
    ["markdown", "notes.md", false],
    ["text", "notes.txt", false],
    ["image", "shot.png", false],
    ["pdf", "paper.pdf", true],
  ] as const)(
    "source_type=%s keeps converting only for pdf",
    (sourceType, fileName, keepConverting) => {
      const view = buildIngestionRunView(
        doc({
          id: `d-${sourceType}`,
          file_name: fileName,
          status: "processing",
          current_stage: "chunking",
          stage_message: "Chunking — 1/3",
          source_type: sourceType,
          track_id: `t-${sourceType}`,
          progress_counts: { unit: "chunks", current: 1, total: 3 },
        }),
      );
      expect(view?.sourceType).toBe(sourceType);
      const timeline = buildStageTimeline(view!);
      const converting = timeline.steps.find((s) => s.id === "converting");
      if (keepConverting) {
        expect(converting).toBeTruthy();
      } else {
        expect(converting).toBeUndefined();
      }
    },
  );

  it("parses figure vision analyze counts", () => {
    const c = parseCountsFromMessage(
      "Analyzing figures with Vision LLM — figure 3/12",
    );
    expect(c).toEqual({ current: 3, total: 12, unit: "figures" });
  });

  it("builds run view for vision figure analyze during converting", () => {
    const view = buildIngestionRunView(
      doc({
        id: "d-vision",
        file_name: "paper.pdf",
        status: "processing",
        current_stage: "converting",
        stage_message: "Analyzing figures with Vision LLM — figure 5/17",
        stage_progress: 0.99,
        source_type: "pdf",
        track_id: "t-vision",
      }),
    );
    expect(view?.stage).toBe("converting");
    expect(view?.counts).toEqual({ current: 5, total: 17, unit: "figures" });
    expect(formatRunHeadline(view!)).toContain("5/17");
  });

  it("builds run view for extracting document", () => {
    const view = buildIngestionRunView(
      doc({
        id: "d1",
        file_name: "areal.pdf",
        status: "processing",
        current_stage: "extracting",
        stage_message: "chunk 10/100",
        stage_progress: 0.1,
        source_type: "pdf",
        track_id: "t1",
      }),
    );
    expect(view?.stage).toBe("extracting");
    expect(view?.counts?.current).toBe(10);
    expect(formatRunHeadline(view!)).toContain("10/100");
  });

  it("treats converting as active even when coarse status is still pending", () => {
    const view = buildIngestionRunView(
      doc({
        id: "d1",
        file_name: "fast_graph.pdf",
        status: "pending",
        current_stage: "converting",
        stage_message: "Converting PDF",
        stage_progress: 0.5,
        source_type: "pdf",
        track_id: "t1",
      }),
    );
    expect(view?.stage).toBe("converting");
    expect(view?.stageStatus).toBe("active");
    expect(stageStatusFor("converting", "pending")).toBe("active");
    expect(stageStatusFor("queued", "pending")).toBe("pending");
  });

  it("selectPrimaryRun prefers active over queued", () => {
    const map = buildIngestionRunViews([
      doc({
        id: "q1",
        status: "pending",
        current_stage: "queued",
        file_name: "q.md",
      }),
      doc({
        id: "a1",
        status: "processing",
        current_stage: "extracting",
        file_name: "a.md",
        stage_message: "working",
      }),
    ]);
    const primary = selectPrimaryRun(map);
    expect(primary?.documentId).toBe("a1");
  });

  it("dedupes bare uuid pin + staging: list row into one ActiveRun", () => {
    const bare = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";
    const track = "insert-same";
    const map = buildIngestionRunViews([
      doc({
        id: bare,
        status: "pending",
        current_stage: "uploading",
        file_name: "wiki.md",
        stage_message: "Queued for processing…",
        track_id: track,
      }),
      doc({
        id: `staging:${bare}`,
        status: "processing",
        current_stage: "extracting",
        file_name: "wiki.md",
        stage_message: "Extracting entities…",
        track_id: track,
      }),
    ]);
    expect(map.size).toBe(1);
    const run = [...map.values()][0];
    expect(run.documentId).toBe(bare);
    expect(run.stage).toBe("extracting");
    expect(run.message).not.toMatch(/\{\{/);
  });

  it("IS2 LAW-IS4: queued run projects position + ETA chrome", () => {
    const view = buildIngestionRunView(
      doc({
        id: "d-q",
        file_name: "wait.pdf",
        status: "pending",
        current_stage: "queued",
        stage_message: "Waiting for a free worker…",
        track_id: "t-q",
        queue_position: 3,
        eta_seconds: 120,
        eta_basis: "measured",
      }),
    );
    expect(view?.stage).toBe("queued");
    expect(view?.queuePosition).toBe(3);
    expect(formatQueueChrome(view!)).toContain("#3");
    expect(formatRunHeadline(view!)).toMatch(/Queued.*#3/);
    expect(formatRunHeadline(view!)).not.toMatch(/Extracting/);
  });

  it("SPEC-120: capacity wait stage_message is not flattened to Queued", () => {
    const view = buildIngestionRunView(
      doc({
        id: "d-cap",
        file_name: "held.pdf",
        status: "pending",
        current_stage: "queued",
        stage_message: "Waiting for capacity",
        track_id: "t-cap",
        queue_position: 1,
      }),
    );
    expect(view?.message).toMatch(/Waiting for capacity/i);
    expect(view?.message).not.toBe("Queued");
  });

  it("IS3: human labels for gleaning / merging", () => {
    expect(stageDisplayName("gleaning")).toBe("Refining entities");
    expect(stageDisplayName("merging")).toBe("Updating knowledge graph");
  });

  it("IS3 IS-AC-06: phase strip maps all PROCESSING_STAGES wire ids", () => {
    const wire = [
      "cleaning",
      "queued",
      "uploading",
      "converting",
      "preprocessing",
      "chunking",
      "extracting",
      "gleaning",
      "merging",
      "summarizing",
      "embedding",
      "storing",
      "completed",
    ];
    for (const stage of wire) {
      expect(PHASE_STRIP_ORDER).toContain(mapWireStageToPhase(stage));
    }
    expect(mapWireStageToPhase("gleaning")).toBe("extract");
    expect(mapWireStageToPhase("queued")).toBe("admit");
    expect(mapWireStageToPhase("embedding")).toBe("materialize");
  });
});
