/**
 * SPEC-157 — Width budget for the Query companion pane (LAW-157-6).
 *
 * Pure: the caller measures the Query container (everything right of the app
 * sidebar) and gets back how to lay out chat | companion | history.
 *
 *   container >= chatMin + companionMin + historyRail  → side pane
 *   otherwise                                          → bottom sheet
 *
 * History never steals width from chat or the companion: when it cannot fit
 * beside them it moves behind the header button (same sheet as < xl).
 */

export const CHAT_MIN_PX = 420;
export const COMPANION_MIN_PX = 440;
export const COMPANION_MAX_PX = 920;
export const COMPANION_DEFAULT_PX = 520;
/** Docked history budget while a companion is open (panel is clamped to it). */
export const HISTORY_OPEN_PX = 340;
export const HISTORY_RAIL_PX = 40;

/** How the history panel is presented. */
export type HistoryPresentation = "docked" | "rail" | "overlay";

export type CompanionLayout =
  | { mode: "sheet"; history: HistoryPresentation }
  | {
      mode: "side";
      history: HistoryPresentation;
      widthPx: number;
      minPx: number;
      maxPx: number;
    };

export type CompanionLayoutInput = {
  /** Measured width of the Query region (px). */
  containerWidth: number;
  /** History panel can be docked at this breakpoint (xl+). */
  historyDocked: boolean;
  /** User's preference for the history panel. */
  historyOpen: boolean;
  /** Preferred companion width (persisted). */
  preferredWidth: number;
};

function clamp(n: number, lo: number, hi: number): number {
  return Math.min(hi, Math.max(lo, n));
}

function preferredHistory(input: CompanionLayoutInput): HistoryPresentation {
  if (!input.historyDocked) return "overlay";
  return input.historyOpen ? "docked" : "rail";
}

export function resolveCompanionLayout(
  input: CompanionLayoutInput,
): CompanionLayout {
  const { containerWidth, historyDocked, historyOpen, preferredWidth } = input;
  const railReserve = historyDocked ? HISTORY_RAIL_PX : 0;

  if (containerWidth < CHAT_MIN_PX + COMPANION_MIN_PX + railReserve) {
    return { mode: "sheet", history: preferredHistory(input) };
  }

  const openFits =
    containerWidth - HISTORY_OPEN_PX >= CHAT_MIN_PX + COMPANION_MIN_PX;
  const history: HistoryPresentation = !historyDocked
    ? "overlay"
    : historyOpen
      ? openFits
        ? "docked"
        : "overlay"
      : "rail";

  const historyReserve =
    history === "docked"
      ? HISTORY_OPEN_PX
      : history === "rail"
        ? HISTORY_RAIL_PX
        : 0;
  const maxPx = Math.min(
    COMPANION_MAX_PX,
    containerWidth - historyReserve - CHAT_MIN_PX,
  );
  return {
    mode: "side",
    history,
    widthPx: clamp(preferredWidth, COMPANION_MIN_PX, maxPx),
    minPx: COMPANION_MIN_PX,
    maxPx,
  };
}
