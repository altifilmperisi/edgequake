/**
 * Shared message types for query chat bubbles.
 */
import type { QueryContext, QueryMode } from "@/types";

export interface ChatMessageData {
  id: string;
  role: "user" | "assistant";
  content: string;
  timestamp?: number;
  isStreaming?: boolean;
  isError?: boolean;
  mode?: QueryMode;
  tokensUsed?: number;
  durationMs?: number;
  thinkingTimeMs?: number;
  context?: QueryContext;
  llmProvider?: string;
  llmModel?: string;
  /** Client-side: stream was aborted with partial content */
  stopped?: boolean;
  /** Persisted thumbs feedback (SPEC-155 B2) */
  feedbackRating?: "up" | "down" | null;
}

export interface ChatMessageProps {
  message: ChatMessageData;
  isLast?: boolean;
  onCopy?: () => void;
  onRegenerate?: () => void;
  onRetry?: () => void;
  onContinue?: () => void;
  onEdit?: (content: string) => void;
  onFeedback?: (rating: "up" | "down" | null) => void;
  showMetadata?: boolean;
  stage?: string | null;
  stageDetail?: string;
}
