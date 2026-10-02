/**
 * SPEC-155 — one cancel flow for every surface (run card, preview panel, bulk bar).
 *
 * - With a `track_id`: pin intent + optimistic Stopping…, cancel the task.
 * - Without one (orphan row): cancel by document id — never leave the user
 *   with Delete as the only way out.
 * Falls back from task to document cancel when the task endpoint fails.
 */

import type { QueryClient } from "@tanstack/react-query";
import { cancelDocument, cancelTask } from "@/lib/api/edgequake";
import {
  patchDocumentsCancelOptimistic,
  patchDocumentsCancelledByDocumentId,
  pinCancelIntent,
} from "@/lib/documents/cancel-intent";

export interface CancelTarget {
  documentId: string;
  trackId?: string | null;
}

/** Minimal document shape the UI hands to cancel (a list row satisfies it). */
export interface CancelableDocument {
  id: string;
  track_id?: string | null;
}

export type CancelOutcome = "cancelled" | "failed";

export function invalidateRunQueries(queryClient: QueryClient): void {
  void queryClient.invalidateQueries({ queryKey: ["documents"] });
  void queryClient.invalidateQueries({ queryKey: ["tasks"] });
  void queryClient.invalidateQueries({ queryKey: ["pipeline-status"] });
}

async function cancelByDocument(
  queryClient: QueryClient,
  documentId: string,
): Promise<boolean> {
  try {
    await cancelDocument(documentId);
    patchDocumentsCancelledByDocumentId(queryClient, documentId);
    return true;
  } catch {
    return false;
  }
}

/** Cancel one document's in-flight work; always refreshes the run queries. */
export async function cancelDocumentRun(
  queryClient: QueryClient,
  target: CancelTarget,
): Promise<CancelOutcome> {
  const { documentId, trackId } = target;
  try {
    if (trackId) {
      pinCancelIntent(trackId);
      patchDocumentsCancelOptimistic(queryClient, trackId);
      try {
        await cancelTask(trackId);
        return "cancelled";
      } catch {
        // Task row may be gone or lag behind — converge via the document.
        return (await cancelByDocument(queryClient, documentId))
          ? "cancelled"
          : "failed";
      }
    }
    return (await cancelByDocument(queryClient, documentId))
      ? "cancelled"
      : "failed";
  } finally {
    invalidateRunQueries(queryClient);
  }
}
