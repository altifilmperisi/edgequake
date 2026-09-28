/**
 * SPEC-151 detail-page lifecycle SSOT.
 *
 * Maps coarse document status + fine-grained stage/progress into one header
 * story so page-health strip and empty-markdown never contradict the badge.
 */

import {
  getDocumentDisplayStatus,
  isProcessingStatus,
  type DocumentStatus,
  type DocumentStatusInput,
} from "@/lib/documents/status-domain";

export type DetailLifecycleKind =
  | "queued"
  | "converting"
  | "extracting"
  | "indexing"
  | "ready"
  | "failed"
  | "partial"
  | "cancelled"
  | "stopping";

export type EmptyMarkdownMode = "in_flight" | "failed" | "missing";

export interface ProgressCountsLike {
  unit?: string | null;
  current?: number | null;
  total?: number | null;
}

export interface DetailLifecycleInput extends DocumentStatusInput {
  stage_message?: string | null;
  progress_counts?: ProgressCountsLike | null;
  page_count?: number | null;
}

export interface DetailLifecycle {
  kind: DetailLifecycleKind;
  /** Wire status used for classification (display/stage). */
  displayStatus: DocumentStatus;
  /** Short badge label (English defaults; callers may i18n by kind). */
  label: string;
  showSpinner: boolean;
  isInFlight: boolean;
  canCancel: boolean;
  canReprocess: boolean;
  canReprocessPages: boolean;
  emptyMarkdownMode: EmptyMarkdownMode;
}

function kindFromStatus(status: DocumentStatus): DetailLifecycleKind {
  switch (status) {
    case "queued":
    case "pending":
    case "uploading":
    case "held":
      return "queued";
    case "converting":
    case "preprocessing":
      return "converting";
    case "chunking":
    case "extracting":
    case "gleaning":
    case "processing":
      return "extracting";
    case "embedding":
    case "re_embedding":
    case "merging":
    case "summarizing":
    case "storing":
    case "projecting":
    case "indexing":
      return "indexing";
    case "completed":
    case "indexed":
    case "partial_success":
      return "ready";
    case "failed":
    case "delete_failed":
    case "dead_letter":
      return "failed";
    case "partial_failure":
      return "partial";
    case "cancelled":
      return "cancelled";
    case "stopping":
    case "cancelling":
      return "stopping";
    case "cleaning":
    case "deleting":
      return "indexing";
    default:
      return isProcessingStatus(status) ? "extracting" : "queued";
  }
}

function baseLabel(kind: DetailLifecycleKind): string {
  switch (kind) {
    case "queued":
      return "Queued";
    case "converting":
      return "Converting";
    case "extracting":
      return "Extracting";
    case "indexing":
      return "Indexing";
    case "ready":
      return "Ready";
    case "failed":
      return "Failed";
    case "partial":
      return "Partial failure";
    case "cancelled":
      return "Cancelled";
    case "stopping":
      return "Stopping";
  }
}

function progressSuffix(
  counts: ProgressCountsLike | null | undefined,
  pageCount: number | null | undefined,
): string | null {
  const current = counts?.current;
  const total =
    counts?.total ??
    (typeof pageCount === "number" && pageCount > 0 ? pageCount : null);
  if (
    typeof current === "number" &&
    typeof total === "number" &&
    total > 0 &&
    current >= 0
  ) {
    const unit = (counts?.unit || "pages").toLowerCase();
    if (unit === "pages" || unit === "page") {
      return `${current}/${total}`;
    }
    return `${current}/${total} ${unit}`;
  }
  return null;
}

/**
 * Resolve one lifecycle view for the document detail header + empty states.
 */
export function resolveDetailLifecycle(
  input: DetailLifecycleInput,
): DetailLifecycle {
  const displayStatus = getDocumentDisplayStatus(input);
  const kind = kindFromStatus(displayStatus);
  const isInFlight =
    kind === "queued" ||
    kind === "converting" ||
    kind === "extracting" ||
    kind === "indexing" ||
    kind === "stopping";
  const suffix = progressSuffix(input.progress_counts, input.page_count);
  let label = baseLabel(kind);
  if (isInFlight && suffix && (kind === "converting" || kind === "extracting")) {
    label = `${label} · ${suffix}`;
  } else if (
    isInFlight &&
    input.stage_message &&
    input.stage_message.trim().length > 0 &&
    input.stage_message.trim().length < 48 &&
    !/failed|error|retry/i.test(input.stage_message)
  ) {
    // Prefer short stage messages over bare Processing.
    label = input.stage_message.trim();
  }

  const canCancel =
    Boolean(input.track_id) &&
    (kind === "queued" ||
      kind === "converting" ||
      kind === "extracting" ||
      kind === "indexing");
  const canReprocess =
    kind === "ready" ||
    kind === "failed" ||
    kind === "partial" ||
    kind === "cancelled";
  const canReprocessPages = canReprocess && !isInFlight;

  let emptyMarkdownMode: EmptyMarkdownMode = "missing";
  if (isInFlight) emptyMarkdownMode = "in_flight";
  else if (kind === "failed" || kind === "partial" || kind === "cancelled") {
    emptyMarkdownMode = "failed";
  }

  return {
    kind,
    displayStatus,
    label,
    showSpinner: isInFlight,
    isInFlight,
    canCancel,
    canReprocess,
    canReprocessPages,
    emptyMarkdownMode,
  };
}
