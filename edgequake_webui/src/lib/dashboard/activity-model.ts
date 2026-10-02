/**
 * Dashboard "Recent activity" row model (pure, no React).
 *
 * WHY: the dashboard used to keep its own status map and silently fell back to
 * "Completed" for every status it did not know (delete_failed, stalled, …).
 * The row now derives its state from the same domain helpers as the Documents
 * table, so the two surfaces can never disagree.
 */
import {
  getDocumentDisplayStatus,
  type DocumentStatus,
} from "@/lib/documents/status-domain";
import { documentStalledForMs } from "@/lib/documents/document-run-state";
import { categorizeError } from "@/lib/error-categories";
import { formatSilence } from "@/lib/pipeline/run-liveness";
import type { Document, DocumentStatusCounts } from "@/types";

export type ActivityTone = "error" | "warn" | "muted";

/** Secondary line under the title: live progress, or why it needs attention. */
export type ActivityDetail =
  | { kind: "progress"; pct: number | null; label: string }
  | { kind: "note"; tone: ActivityTone; text: string };

export interface ActivityItem {
  id: string;
  title: string;
  status: DocumentStatus;
  createdAt: string | null;
  detail: ActivityDetail | null;
}

const IN_FLIGHT = new Set<string>([
  "pending",
  "processing",
  "converting",
  "preprocessing",
  "chunking",
  "extracting",
  "gleaning",
  "merging",
  "summarizing",
  "embedding",
  "re_embedding",
  "storing",
  "indexing",
  "projecting",
  "cleaning",
]);

/** 0–100 or null when the server gave no fraction (never invent a number). */
export function progressPct(doc: Pick<Document, "stage_progress">): number | null {
  const p = doc.stage_progress;
  if (typeof p !== "number" || Number.isNaN(p)) return null;
  return Math.min(100, Math.max(0, Math.round(p * 100)));
}

function failureNote(doc: Document): ActivityDetail | null {
  const raw = (doc.error_message || doc.stage_message || "").trim();
  if (!raw) return null;
  const { category, categoryLabel, summary } = categorizeError(raw);
  // The raw first line is long and jargon-heavy; lead with the category.
  if (category === "lifecycle") {
    return {
      kind: "note",
      tone: "error",
      text: "Deleted before it finished — delete it, then re-upload",
    };
  }
  const text = category === "unknown" ? summary : `${categoryLabel}: ${summary}`;
  return { kind: "note", tone: "error", text };
}

function liveDetail(doc: Document, status: DocumentStatus): ActivityDetail | null {
  if (status === "pending") {
    return { kind: "note", tone: "muted", text: "Waiting to start" };
  }
  const label = (doc.stage_message || "").trim() || "Processing";
  return { kind: "progress", pct: progressPct(doc), label };
}

/** Detail line for one document (null = nothing worth saying). */
export function activityDetail(doc: Document, status: DocumentStatus): ActivityDetail | null {
  if (status === "delete_failed") {
    return {
      kind: "note",
      tone: "error",
      text: "Delete did not finish — open Documents to finish it",
    };
  }
  if (status === "failed" || status === "partial_failure") return failureNote(doc);
  if (status === "cancelled") return { kind: "note", tone: "muted", text: "Cancelled" };
  if (!IN_FLIGHT.has(status)) return null;

  const silentMs = documentStalledForMs(doc);
  if (silentMs !== null) {
    return {
      kind: "note",
      tone: "warn",
      text: `Stalled — no progress for ${formatSilence(silentMs)}`,
    };
  }
  return liveDetail(doc, status);
}

export function buildActivityItem(doc: Document): ActivityItem {
  const status = getDocumentDisplayStatus(doc);
  return {
    id: doc.id,
    title: doc.title || doc.file_name || "Untitled",
    status,
    createdAt: doc.created_at ?? null,
    detail: activityDetail(doc, status),
  };
}

export interface AttentionSummary {
  processing: number;
  pending: number;
  failed: number;
}

/**
 * `status_counts` is only trustworthy when the server actually sent numbers.
 * An empty/partial object must read as "unknown", never as "everything is 0".
 */
export function hasStatusCounts(
  counts: Partial<DocumentStatusCounts> | null | undefined,
): counts is Partial<DocumentStatusCounts> {
  return (
    !!counts &&
    ["pending", "processing", "completed", "failed"].every(
      (k) => typeof counts[k as keyof DocumentStatusCounts] === "number",
    )
  );
}

/** Workspace-wide counts (from `status_counts`) that deserve a glance. */
export function summarizeAttention(
  counts: Partial<DocumentStatusCounts> | null | undefined,
): AttentionSummary {
  return {
    processing: counts?.processing ?? 0,
    pending: counts?.pending ?? 0,
    failed: (counts?.failed ?? 0) + (counts?.partial_failure ?? 0),
  };
}

/** True when nothing is running, queued or failed. */
export function isAllSettled(s: AttentionSummary): boolean {
  return s.processing + s.pending + s.failed === 0;
}
