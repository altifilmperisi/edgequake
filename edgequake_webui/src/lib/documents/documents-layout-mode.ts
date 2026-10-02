/**
 * SPEC-155 — Documents page curated layout modes.
 *
 * Three presets (focus / split / stack) with persistence and a pure
 * resolve helper for the intake strip variant. No free-form panel geometry.
 */

export type DocumentsLayoutMode = "focus" | "split" | "stack";

export const DOCUMENTS_LAYOUT_MODES: readonly DocumentsLayoutMode[] = [
  "focus",
  "split",
  "stack",
] as const;

export const LAYOUT_MODE_STORAGE_KEY = "edgequake.documents.pageLayoutMode";

/** Pointer-drag thresholds (px) from grip start → next mode. */
export const LAYOUT_SNAP_TO_SPLIT_PX = 40;
export const LAYOUT_SNAP_TO_STACK_PX = 100;

export type IntakeStripVariant = "solo" | "chip" | "split" | "stack";

export interface ResolvedIntakeLayout {
  /** Visual arrangement inside DocumentIntakeStrip. */
  stripVariant: IntakeStripVariant;
  /**
   * When true, strip may use the full intake max-height budget
   * (split/stack with live feedback). Focus keeps a chip-height cap.
   */
  showExpandedBudget: boolean;
}

export function isDocumentsLayoutMode(value: unknown): value is DocumentsLayoutMode {
  return value === "focus" || value === "split" || value === "stack";
}

export function parseDocumentsLayoutMode(
  raw: string | null | undefined,
): DocumentsLayoutMode {
  if (isDocumentsLayoutMode(raw)) return raw;
  return "focus";
}

export function readLayoutMode(storage?: Storage | null): DocumentsLayoutMode {
  try {
    const store =
      storage ?? (typeof localStorage !== "undefined" ? localStorage : null);
    if (!store) return "focus";
    return parseDocumentsLayoutMode(store.getItem(LAYOUT_MODE_STORAGE_KEY));
  } catch {
    return "focus";
  }
}

export function writeLayoutMode(
  mode: DocumentsLayoutMode,
  storage?: Storage | null,
): void {
  try {
    const store =
      storage ?? (typeof localStorage !== "undefined" ? localStorage : null);
    if (!store) return;
    store.setItem(LAYOUT_MODE_STORAGE_KEY, mode);
  } catch {
    // ignore quota / private-mode failures
  }
}

/**
 * Map page mode + whether feedback is open → strip geometry.
 *
 * - Idle (no feedback): always `solo` (full-width dropzone), regardless of mode.
 * - Live + focus: `chip` (one-line band).
 * - Live + split: `split` (dz | runs on md+).
 * - Live + stack: `stack` (dz then runs).
 */
export function resolveIntakeLayout(
  mode: DocumentsLayoutMode,
  hasFeedback: boolean,
): ResolvedIntakeLayout {
  if (!hasFeedback) {
    return { stripVariant: "solo", showExpandedBudget: false };
  }
  switch (mode) {
    case "focus":
      return { stripVariant: "chip", showExpandedBudget: false };
    case "split":
      return { stripVariant: "split", showExpandedBudget: true };
    case "stack":
      return { stripVariant: "stack", showExpandedBudget: true };
    default:
      return { stripVariant: "chip", showExpandedBudget: false };
  }
}

/**
 * Given pointer deltaY from grip start (positive = drag toward inventory /
 * compress; negative = drag expand), return the mode to snap to.
 *
 * Compress (positive dy): toward focus.
 * Expand (negative dy): focus → split → stack.
 */
export function snapLayoutModeFromDrag(
  startMode: DocumentsLayoutMode,
  deltaY: number,
): DocumentsLayoutMode {
  if (deltaY >= LAYOUT_SNAP_TO_SPLIT_PX) {
    // Compress toward focus
    if (startMode === "stack") {
      return deltaY >= LAYOUT_SNAP_TO_STACK_PX ? "focus" : "split";
    }
    return "focus";
  }
  if (deltaY <= -LAYOUT_SNAP_TO_SPLIT_PX) {
    // Expand away from focus
    const expand = Math.abs(deltaY);
    if (startMode === "focus") {
      return expand >= LAYOUT_SNAP_TO_STACK_PX ? "stack" : "split";
    }
    if (startMode === "split") {
      return expand >= LAYOUT_SNAP_TO_SPLIT_PX ? "stack" : "split";
    }
    return "stack";
  }
  return startMode;
}

/** Chip-row max height for focus mode (one line). */
export const FOCUS_STRIP_MAX_REM = 3.5;

export function focusStripMaxHeightCss(): string {
  return `${FOCUS_STRIP_MAX_REM}rem`;
}

export function cycleLayoutMode(
  mode: DocumentsLayoutMode,
  direction: 1 | -1 = 1,
): DocumentsLayoutMode {
  const idx = DOCUMENTS_LAYOUT_MODES.indexOf(mode);
  const next =
    (idx + direction + DOCUMENTS_LAYOUT_MODES.length) %
    DOCUMENTS_LAYOUT_MODES.length;
  return DOCUMENTS_LAYOUT_MODES[next]!;
}
