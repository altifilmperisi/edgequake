import { api } from "../client";

export interface CancelDocumentResponse {
  document_id: string;
  status: string;
  track_id?: string | null;
  /** True when a live/queued task row was cancelled (false for orphan rows). */
  task_cancelled: boolean;
}

/**
 * Cancel a document's in-flight work by document id.
 * Works for orphaned rows that have no `track_id` (cancelTask cannot).
 * Idempotent; 409 when the document already finished.
 */
export async function cancelDocument(
  documentId: string,
): Promise<CancelDocumentResponse> {
  return api.post<CancelDocumentResponse>(
    `/documents/${encodeURIComponent(documentId)}/cancel`,
  );
}
