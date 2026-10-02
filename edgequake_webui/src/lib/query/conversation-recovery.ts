/**
 * Shared recovery handlers for stale conversations and LLM auth failures.
 * @implements SPEC-155 W7Q DRY (was duplicated 4× / 2× in use-query-streaming)
 */
import {
  isConversationGoneError,
  isConversationGoneStreamCode,
  isConversationNotFoundError,
} from "@/lib/query/conversation-errors";
import { isLlmProviderAuthFailure } from "@/lib/query-model-selection";

export type RecoveryCopy = {
  conversationExpired: string;
  startingNewConversation: string;
  modelAuthReset: string;
  modelAuthResetDesc: string;
};

export type RecoveryActions = {
  clearActiveConversation: () => void;
  clearPending: () => void;
  resetProviderModel: () => void;
  toastWarning: (title: string, description?: string) => void;
};

export function isConversationRecoveryError(
  error: unknown,
  streamCode?: string,
): boolean {
  return (
    isConversationGoneStreamCode(streamCode) ||
    isConversationNotFoundError(error) ||
    isConversationGoneError(error)
  );
}

/**
 * Handle conversation-gone / not-found: clear active conversation + pending UI.
 * Returns true when handled.
 */
export function handleConversationRecovery(
  error: unknown,
  actions: RecoveryActions,
  copy: RecoveryCopy,
  streamCode?: string,
): boolean {
  if (!isConversationRecoveryError(error, streamCode)) return false;

  actions.clearActiveConversation();
  actions.clearPending();
  actions.toastWarning(copy.conversationExpired, copy.startingNewConversation);
  return true;
}

/**
 * Handle LLM provider auth failure: reset provider/model selection.
 * Returns true when handled.
 */
export function handleProviderAuthRecovery(
  errorMessage: string,
  actions: RecoveryActions,
  copy: RecoveryCopy,
): boolean {
  if (!isLlmProviderAuthFailure(errorMessage)) return false;

  actions.resetProviderModel();
  actions.toastWarning(copy.modelAuthReset, copy.modelAuthResetDesc);
  return true;
}

export function errorMessageOf(error: unknown, fallback = "Query failed"): string {
  return error instanceof Error ? error.message : fallback;
}
