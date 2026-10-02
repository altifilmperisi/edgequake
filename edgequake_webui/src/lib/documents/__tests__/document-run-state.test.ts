import { describe, expect, it } from "vitest";
import {
  canCancelDocument,
  documentStalledForMs,
  isDeleteFailedDocument,
} from "@/lib/documents/document-run-state";

const NOW = Date.parse("2026-10-01T12:00:00Z");
const ago = (ms: number) => new Date(NOW - ms).toISOString();
const DAY = 24 * 3_600_000;

describe("canCancelDocument", () => {
  it("allows cancel for in-flight docs even without a track_id", () => {
    expect(canCancelDocument({ status: "processing", track_id: null })).toBe(true);
    expect(canCancelDocument({ status: "processing", current_stage: "preprocessing" })).toBe(true);
    expect(canCancelDocument({ status: "pending" })).toBe(true);
    expect(canCancelDocument({ status: "processing", current_stage: "queued" })).toBe(true);
  });

  it("blocks cancel for terminal and winding-down docs", () => {
    for (const status of ["completed", "indexed", "failed", "cancelled", "partial_failure"]) {
      expect(canCancelDocument({ status })).toBe(false);
    }
    expect(canCancelDocument({ status: "processing", ui_phase: "stopping" })).toBe(false);
    expect(canCancelDocument({ status: "deleting" })).toBe(false);
  });
});

describe("isDeleteFailedDocument", () => {
  it("is true only for a half-finished delete", () => {
    expect(isDeleteFailedDocument({ status: "delete_failed" })).toBe(true);
    for (const status of ["failed", "deleting", "processing", "completed"]) {
      expect(isDeleteFailedDocument({ status })).toBe(false);
    }
  });

  it("is never cancellable (it is not in-flight work)", () => {
    expect(canCancelDocument({ status: "delete_failed", current_stage: "materialize" })).toBe(false);
  });
});

describe("documentStalledForMs", () => {
  it("flags a processing doc that has been silent for days", () => {
    expect(
      documentStalledForMs(
        { status: "processing", current_stage: "preprocessing", updated_at: ago(3 * DAY) },
        NOW,
      ),
    ).toBe(3 * DAY);
  });

  it("does not flag fresh work, queued docs or terminals", () => {
    expect(documentStalledForMs({ status: "processing", updated_at: ago(30_000) }, NOW)).toBeNull();
    expect(documentStalledForMs({ status: "pending", updated_at: ago(3 * DAY) }, NOW)).toBeNull();
    expect(documentStalledForMs({ status: "completed", updated_at: ago(3 * DAY) }, NOW)).toBeNull();
    expect(documentStalledForMs({ status: "failed", updated_at: ago(3 * DAY) }, NOW)).toBeNull();
  });

  it("does not flag when the timestamp is unknown", () => {
    expect(documentStalledForMs({ status: "processing" }, NOW)).toBeNull();
  });
});
