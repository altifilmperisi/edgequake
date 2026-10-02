/**
 * Composer UI state machine (SPEC-155).
 * Single source of truth for idle / streaming / queued / stopped chrome.
 */
export type ComposerPhase = "idle" | "streaming" | "queued" | "error";

export type ComposerEvent =
  | { type: "stream_start" }
  | { type: "stream_end" }
  | { type: "queue"; text: string }
  | { type: "clear_queue" }
  | { type: "error" }
  | { type: "reset" };

export interface ComposerUiState {
  phase: ComposerPhase;
  queuedText: string | null;
}

export function createComposerUiState(): ComposerUiState {
  return { phase: "idle", queuedText: null };
}

export function reduceComposerUi(
  state: ComposerUiState,
  event: ComposerEvent,
): ComposerUiState {
  switch (event.type) {
    case "stream_start":
      return { ...state, phase: "streaming" };
    case "stream_end":
      return {
        phase: state.queuedText ? "queued" : "idle",
        queuedText: state.queuedText,
      };
    case "queue":
      return {
        phase: state.phase === "streaming" ? "streaming" : "queued",
        queuedText: event.text,
      };
    case "clear_queue":
      return {
        phase: state.phase === "streaming" ? "streaming" : "idle",
        queuedText: null,
      };
    case "error":
      return { ...state, phase: "error" };
    case "reset":
      return createComposerUiState();
    default:
      return state;
  }
}

/** Map stream-session facts → composer phase (pure). */
export function composerPhaseFromSession(opts: {
  isStreaming: boolean;
  queuedMessage: string | null;
  isError: boolean;
}): ComposerPhase {
  if (opts.isError) return "error";
  if (opts.isStreaming) return "streaming";
  if (opts.queuedMessage) return "queued";
  return "idle";
}

export function composerShowsStop(phase: ComposerPhase): boolean {
  return phase === "streaming";
}

export function composerShowsSend(phase: ComposerPhase): boolean {
  return phase !== "streaming";
}
