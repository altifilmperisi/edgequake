/**
 * SPEC-120 Lens 8: client-side cancel intent for optimistic Stopping… UX.
 *
 * Persists until the server confirms terminal cancel or the page reloads.
 */

import type { QueryClient } from "@tanstack/react-query";
import type { Document } from "@/types";

export interface CancelIntent {
  startedAt: number;
  expectedStopBy?: string | null;
}

const cancelIntents = new Map<string, CancelIntent>();

type DocumentsQueryData = {
  items?: Document[];
  [key: string]: unknown;
};

export function pinCancelIntent(
  trackId: string,
  expectedStopBy?: string | null,
): void {
  if (!trackId.trim()) return;
  cancelIntents.set(trackId, {
    startedAt: Date.now(),
    expectedStopBy: expectedStopBy ?? null,
  });
}

export function clearCancelIntent(trackId: string): void {
  cancelIntents.delete(trackId);
}

export function getCancelIntent(trackId: string): CancelIntent | undefined {
  return cancelIntents.get(trackId);
}

export function hasCancelIntent(trackId: string | null | undefined): boolean {
  return Boolean(trackId && cancelIntents.has(trackId));
}

/** Seconds remaining until expected_stop_by; negative when overdue. */
export function secondsUntilExpectedStop(
  expectedStopBy: string | null | undefined,
  nowMs: number = Date.now(),
): number | null {
  if (!expectedStopBy) return null;
  const deadline = Date.parse(expectedStopBy);
  if (Number.isNaN(deadline)) return null;
  return Math.ceil((deadline - nowMs) / 1000);
}

/** True when stop deadline exceeded by 2× the original window (Lens 8). */
export function isStopOverdue(
  intent: CancelIntent,
  nowMs: number = Date.now(),
): boolean {
  if (intent.expectedStopBy) {
    const deadline = Date.parse(intent.expectedStopBy);
    if (!Number.isNaN(deadline)) {
      const windowMs = Math.max(deadline - intent.startedAt, 1);
      return nowMs - intent.startedAt > windowMs * 2;
    }
  }
  // Fallback: 120s without expected_stop_by.
  return nowMs - intent.startedAt > 120_000;
}

export function stoppingMessageForIntent(
  intent: CancelIntent,
  nowMs: number = Date.now(),
): string {
  if (isStopOverdue(intent, nowMs)) {
    return "Still stopping. This can take up to two minutes if the worker became unavailable.";
  }
  const remaining = secondsUntilExpectedStop(intent.expectedStopBy, nowMs);
  if (remaining != null && remaining > 0) {
    return `Finishing the current step, then cleaning up · ~${remaining}s`;
  }
  return "Cancellation requested…";
}

/** Apply optimistic stopping fields when cancel intent is pinned. */
export function applyCancelIntentToDocument<
  T extends {
    track_id?: string | null;
    status?: string | null;
    ui_phase?: string | null;
    display_status?: string | null;
    current_stage?: string | null;
  },
>(doc: T): T {
  if (!doc.track_id || !hasCancelIntent(doc.track_id)) {
    return doc;
  }
  if (
    doc.status?.toLowerCase() === "cancelled" ||
    doc.ui_phase?.toLowerCase() === "terminal"
  ) {
    clearCancelIntent(doc.track_id);
    return doc;
  }
  return {
    ...doc,
    ui_phase: "stopping",
    display_status:
      doc.display_status ?? doc.current_stage ?? doc.status ?? undefined,
  };
}

/** Patch documents list cache to show Stopping… immediately after cancel click. */
export function patchDocumentsCancelOptimistic(
  queryClient: QueryClient,
  trackId: string,
): void {
  queryClient.setQueriesData(
    { queryKey: ["documents"] },
    (oldData: DocumentsQueryData | undefined) => {
      if (!oldData?.items) return oldData;
      return {
        ...oldData,
        items: oldData.items.map((doc) =>
          doc.track_id === trackId
            ? applyCancelIntentToDocument({
                ...doc,
                ui_phase: "stopping",
                display_status:
                  doc.display_status ?? doc.current_stage ?? doc.status,
              })
            : doc,
        ),
      };
    },
  );
}

/**
 * Patch documents list cache to terminal Cancelled by document id.
 * For orphan rows with no `track_id`, where no Stopping… phase exists.
 */
export function patchDocumentsCancelledByDocumentId(
  queryClient: QueryClient,
  documentId: string,
): void {
  queryClient.setQueriesData(
    { queryKey: ["documents"] },
    (oldData: DocumentsQueryData | undefined) => {
      if (!oldData?.items) return oldData;
      return {
        ...oldData,
        items: oldData.items.map((doc) =>
          doc.id === documentId
            ? {
                ...doc,
                status: "cancelled" as Document["status"],
                current_stage: "cancelled",
                ui_phase: "terminal",
                display_status: "cancelled",
              }
            : doc,
        ),
      };
    },
  );
}
