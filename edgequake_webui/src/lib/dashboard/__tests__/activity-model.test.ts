import { describe, expect, it } from "vitest";
import {
  buildActivityItem,
  hasStatusCounts,
  isAllSettled,
  progressPct,
  summarizeAttention,
} from "@/lib/dashboard/activity-model";
import type { Document } from "@/types";

const doc = (p: Partial<Document> & { id: string }): Document =>
  ({ title: p.id, chunk_count: 0, ...p }) as Document;

const MIN = 60_000;
const ago = (ms: number) => new Date(Date.now() - ms).toISOString();

describe("buildActivityItem", () => {
  it("never reports a half-deleted document as Completed", () => {
    const item = buildActivityItem(doc({ id: "a", status: "delete_failed" }));
    expect(item.status).toBe("delete_failed");
    expect(item.detail).toMatchObject({ kind: "note", tone: "error" });
  });

  it("completed documents carry no detail line", () => {
    expect(buildActivityItem(doc({ id: "a", status: "completed" })).detail).toBeNull();
  });

  it("live processing exposes a real fraction and the stage message", () => {
    const item = buildActivityItem(
      doc({
        id: "a",
        status: "processing",
        current_stage: "extracting",
        stage_message: "Extracting Entities",
        stage_progress: 0.426,
        updated_at: ago(MIN),
      }),
    );
    expect(item.detail).toEqual({ kind: "progress", pct: 43, label: "Extracting Entities" });
  });

  it("no fraction from the server => null pct (no fake number)", () => {
    const item = buildActivityItem(
      doc({ id: "a", status: "processing", updated_at: ago(MIN) }),
    );
    expect(item.detail).toMatchObject({ kind: "progress", pct: null });
  });

  it("silent processing is Stalled, not a live bar", () => {
    const item = buildActivityItem(
      doc({ id: "a", status: "processing", stage_progress: 0.5, updated_at: ago(40 * MIN) }),
    );
    expect(item.detail).toMatchObject({ kind: "note", tone: "warn" });
    expect((item.detail as { text: string }).text).toMatch(/Stalled .* 40 minutes/);
  });

  it("failed shows a categorised one-line reason", () => {
    const item = buildActivityItem(
      doc({
        id: "a",
        status: "failed",
        error_message:
          "Knowledge graph persist failed: Storage error: Conflict: cannot ingest into a tombstoned document",
      }),
    );
    expect(item.detail).toMatchObject({ kind: "note", tone: "error" });
    expect((item.detail as { text: string }).text.length).toBeGreaterThan(0);
  });

  it("pending is a quiet note, not progress", () => {
    expect(buildActivityItem(doc({ id: "a", status: "pending" })).detail).toEqual({
      kind: "note",
      tone: "muted",
      text: "Waiting to start",
    });
  });

  it("falls back to file name then Untitled for the title", () => {
    expect(buildActivityItem({ id: "a", file_name: "f.pdf" } as Document).title).toBe("f.pdf");
    expect(buildActivityItem({ id: "a" } as Document).title).toBe("Untitled");
  });
});

describe("progressPct", () => {
  it("clamps and rounds", () => {
    expect(progressPct({ stage_progress: 1.4 })).toBe(100);
    expect(progressPct({ stage_progress: -1 })).toBe(0);
    expect(progressPct({})).toBeNull();
  });
});

describe("summarizeAttention", () => {
  it("folds partial failures into failed and detects a settled workspace", () => {
    const s = summarizeAttention({ processing: 1, pending: 2, failed: 3, partial_failure: 1 });
    expect(s).toEqual({ processing: 1, pending: 2, failed: 4 });
    expect(isAllSettled(s)).toBe(false);
    expect(isAllSettled(summarizeAttention({ completed: 9 } as never))).toBe(true);
    expect(isAllSettled(summarizeAttention(undefined))).toBe(true);
  });
});

describe("hasStatusCounts", () => {
  it("empty or partial objects are unknown, not zero", () => {
    expect(hasStatusCounts(undefined)).toBe(false);
    expect(hasStatusCounts({})).toBe(false);
    expect(hasStatusCounts({ processing: 1 })).toBe(false);
    expect(
      hasStatusCounts({ pending: 0, processing: 0, completed: 3, failed: 0 }),
    ).toBe(true);
  });
});

describe("failure phrasing", () => {
  it("tombstone failure gets actionable copy, not the raw storage string", () => {
    const item = buildActivityItem(
      doc({ id: "a", status: "failed", error_message: "Storage error: Conflict: cannot ingest into a tombstoned document" }),
    );
    expect((item.detail as { text: string }).text).toMatch(/Deleted before it finished/);
  });
});
