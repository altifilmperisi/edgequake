/**
 * SPEC-120: keep WS-advanced in-flight stage ahead of a stale list poll.
 *
 * When the same document + track is already converting (or further), a lagging
 * poll that still says queued must not regress the badge / ActiveRuns.
 * A new track_id is a new run and replaces wholesale.
 */

import type { Document } from "@/types";

import {
  documentStageRank,
  getDocumentDisplayStatus,
  isTerminalStatus,
} from "./status-domain";
import {
  clampMonotonic,
  isRunProgress,
  type RunProgress,
} from "@/lib/pipeline/run-progress";

/**
 * Merge polled list rows with the previous React Query cache.
 * Never regresses non-terminal stage rank for the same track_id.
 */
export function mergeMonotonicListDocuments(
  polled: Document[],
  previous: Document[] | undefined,
): Document[] {
  if (!previous?.length) return polled;
  const prevById = new Map(previous.map((d) => [d.id, d]));

  return polled.map((incoming) => {
    const prev = prevById.get(incoming.id);
    if (!prev) return incoming;

    if (
      incoming.track_id &&
      prev.track_id &&
      incoming.track_id !== prev.track_id
    ) {
      return incoming;
    }

    const incomingStatus = getDocumentDisplayStatus(incoming);
    const prevStatus = getDocumentDisplayStatus(prev);

    // SPEC-098 LAW-098-10: cached deleting must beat a stale terminal success poll
    // (pins also protect; this is the merge-layer safety net).
    if (
      (prevStatus === "deleting" ||
        (prev.status || "").toLowerCase() === "deleting") &&
      isTerminalStatus(incomingStatus) &&
      incomingStatus !== "delete_failed" &&
      (incoming.status || "").toLowerCase() !== "delete_failed"
    ) {
      return {
        ...incoming,
        status: prev.status ?? "deleting",
        current_stage: prev.current_stage ?? "deleting",
        display_status: prev.display_status ?? incoming.display_status,
        ui_phase: prev.ui_phase ?? incoming.ui_phase,
        stage_message: prev.stage_message ?? incoming.stage_message,
        stage_progress: prev.stage_progress ?? incoming.stage_progress,
        track_id: prev.track_id ?? incoming.track_id,
      };
    }

    // Honest terminal poll always wins (pins handle optimistic admit).
    if (isTerminalStatus(incomingStatus)) return incoming;

    const prevRank = documentStageRank(prevStatus);
    const incomingRank = documentStageRank(incomingStatus);
    if (prevRank > incomingRank) {
      return {
        ...incoming,
        status: prev.status ?? incoming.status,
        current_stage: prev.current_stage ?? incoming.current_stage,
        display_status: prev.display_status ?? incoming.display_status,
        ui_phase: prev.ui_phase ?? incoming.ui_phase,
        stage_message: prev.stage_message ?? incoming.stage_message,
        stage_progress: prev.stage_progress ?? incoming.stage_progress,
        track_id: prev.track_id ?? incoming.track_id,
        run_progress: clampMonotonic(
          prev.run_progress as RunProgress | null | undefined,
          incoming.run_progress as RunProgress | null | undefined,
        ),
      };
    }
    // Same or advanced stage: still never regress the typed ledger.
    if (isRunProgress(prev.run_progress) || isRunProgress(incoming.run_progress)) {
      return {
        ...incoming,
        run_progress: clampMonotonic(
          prev.run_progress as RunProgress | null | undefined,
          incoming.run_progress as RunProgress | null | undefined,
        ),
      };
    }
    return incoming;
  });
}
