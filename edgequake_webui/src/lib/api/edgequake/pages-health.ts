/**
 * SPEC-151 — Page health API client.
 */

import { api } from "../client";

export type StageStatus = "pending" | "running" | "ok" | "failed" | "skipped";

export interface StageHealthDto {
  status: StageStatus | string;
  error?: string | null;
  count?: number | null;
  chunk_count?: number | null;
  failed_chunk_count?: number | null;
}

export interface PageHealthDto {
  page_number: number;
  parse: StageHealthDto;
  figures: StageHealthDto;
  entities: StageHealthDto;
}

export interface PageHealthResponse {
  document_id: string;
  page_count: number;
  pages: PageHealthDto[];
  summary: {
    parse_failed: number;
    figures_failed: number;
    entities_failed: number;
  };
  source: string;
}

export type ReprocessPageStage = "parse" | "figures" | "entities";

export interface PartialReprocessPlan {
  pages: number[];
  requested_stages: string[];
  effective_stages: string[];
  dirty_chunk_count: number;
  reusable_chunk_count: number;
  estimated_vision_calls: number;
  warnings: string[];
  suggest_full_reprocess: boolean;
}

export interface ReprocessPagesRequest {
  pages?: number[] | string;
  page_numbers?: number[];
  stages: ReprocessPageStage[];
  dry_run?: boolean;
}

export interface ReprocessPagesResponse {
  dry_run: boolean;
  plan: PartialReprocessPlan;
  track_id?: string | null;
  task_id?: string | null;
}

export async function getPagesHealth(
  documentId: string,
): Promise<PageHealthResponse> {
  return api.get<PageHealthResponse>(`/documents/${documentId}/pages/health`);
}

export async function reprocessPages(
  documentId: string,
  body: ReprocessPagesRequest,
): Promise<ReprocessPagesResponse> {
  return api.post<ReprocessPagesResponse>(
    `/documents/${documentId}/pages/reprocess`,
    body,
  );
}
