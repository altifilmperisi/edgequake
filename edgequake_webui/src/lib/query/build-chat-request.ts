/**
 * Single SSOT for chat completion request payloads (stream + non-stream).
 * @implements SPEC-155 W7Q DRY
 */
import type { ChatCompletionRequest } from "@/lib/api/chat";
import type { DocumentFilter, QueryMode } from "@/types";

export interface QueryRequestSettings {
  mode: QueryMode;
  maxTokens: number;
  temperature: number;
  topK: number;
  stream: boolean;
  provider?: string;
  model?: string;
  reasoningEffort?: string;
  systemPrompt?: string;
  fullChunkContent?: boolean;
  documentFilter?: DocumentFilter;
  scopedDocumentIds?: string[];
}

export interface BuildChatRequestOptions {
  settings: QueryRequestSettings;
  message: string;
  conversationId?: string | null;
  language?: string;
  images?: Array<{ data: string; mime_type: string }>;
  /** Force stream flag (overrides settings.stream when set). */
  stream?: boolean;
}

/**
 * Merge SPEC-031 scopedDocumentIds into a DocumentFilter for the API call.
 * - If neither documentFilter nor scopedDocumentIds are set → undefined
 * - Otherwise merges them: scopedDocumentIds maps to document_ids field
 */
export function buildDocumentFilter(
  settings: Pick<QueryRequestSettings, "documentFilter" | "scopedDocumentIds">,
): DocumentFilter | undefined {
  const hasScopedIds =
    settings.scopedDocumentIds && settings.scopedDocumentIds.length > 0;
  const hasDateOrPattern =
    settings.documentFilter?.date_from ||
    settings.documentFilter?.date_to ||
    settings.documentFilter?.document_pattern;

  if (!hasScopedIds && !hasDateOrPattern) return undefined;

  return {
    ...settings.documentFilter,
    document_ids: hasScopedIds ? settings.scopedDocumentIds : undefined,
  };
}

/** Build a ChatCompletionRequest from UI settings (DRY for stream + non-stream). */
export function buildChatRequest(
  options: BuildChatRequestOptions,
): ChatCompletionRequest {
  const { settings, message, conversationId, language, images, stream } =
    options;

  return {
    conversation_id: conversationId || undefined,
    message,
    mode: settings.mode,
    max_tokens: settings.maxTokens,
    temperature: settings.temperature,
    top_k: settings.topK,
    stream: stream ?? settings.stream,
    provider: settings.provider,
    model: settings.model,
    reasoning_effort: settings.reasoningEffort || undefined,
    language,
    system_prompt: settings.systemPrompt || undefined,
    document_filter: buildDocumentFilter(settings),
    content_granularity: settings.fullChunkContent ? "agent" : "citation",
    images,
  };
}
