/**
 * SPEC-155 — what can the user do about this document's in-flight work?
 *
 * Pure, document-keyed (no track_id requirement): an orphan row with no
 * task must still be cancellable, and one that went silent must say so.
 */

import {
  getDocumentDisplayStatus,
  isProcessingStatus,
  type DocumentStatusInput,
} from "@/lib/documents/status-domain";
import { isActiveProcessingStatus } from "@/lib/pipeline/pipeline-document-state";
import { stalledForMs } from "@/lib/pipeline/run-liveness";

type RunStateInput = DocumentStatusInput & { updated_at?: string | null };

/** Already on its way out (or being deleted) — a second Cancel is noise. */
const WINDING_DOWN = new Set(["stopping", "cancelling", "deleting"]);

/** In-flight (working, queued or stalled) and not already stopping. */
export function canCancelDocument(doc: RunStateInput): boolean {
  const status = getDocumentDisplayStatus(doc);
  return isProcessingStatus(status) && !WINDING_DOWN.has(status);
}

/** Silence in ms when the document claims to work but the server went quiet. */
export function documentStalledForMs(
  doc: RunStateInput,
  now: number = Date.now(),
): number | null {
  if (!isActiveProcessingStatus(getDocumentDisplayStatus(doc))) return null;
  return stalledForMs(doc.updated_at, now);
}

/**
 * Delete started (tombstoned) but cleanup did not finish. Lifecycle-exclusive:
 * the only valid action is to finish the delete — never Reprocess/Reset.
 */
export function isDeleteFailedDocument(doc: RunStateInput): boolean {
  return getDocumentDisplayStatus(doc) === "delete_failed";
}
