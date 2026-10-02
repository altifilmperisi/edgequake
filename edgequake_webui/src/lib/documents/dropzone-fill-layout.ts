/**
 * SPEC-155 — Upload dropzone layout adapted to its parent panel size.
 *
 * - row: short wide band (tools strip) → icon | text | parser on one line
 * - stack: tall narrow column → vertical stack
 * - hero: roomy panel → centered column with subtitle
 */

export type DropzoneFillLayout = "row" | "stack" | "hero";

/** Below this height (px), prefer a single horizontal band. */
export const DROPZONE_ROW_MAX_HEIGHT_PX = 120;
/** Below this width (px), prefer a vertical stack. */
export const DROPZONE_STACK_MAX_WIDTH_PX = 280;

export function resolveDropzoneFillLayout(
  width: number,
  height: number,
): DropzoneFillLayout {
  if (!(width > 0) || !(height > 0)) return "hero";
  if (height < DROPZONE_ROW_MAX_HEIGHT_PX) return "row";
  if (width < DROPZONE_STACK_MAX_WIDTH_PX) return "stack";
  return "hero";
}
