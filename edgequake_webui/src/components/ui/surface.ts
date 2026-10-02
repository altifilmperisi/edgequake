/**
 * Shared "soft" surface language for every floating layer
 * (dropdown, select, popover, hover-card, context menu, tooltip, dialog).
 *
 * One definition → consistent radius, hairline edge, layered shadow and
 * motion everywhere, so no menu looks bolted-on.
 */

/** Hairline edge + layered ambient shadow (no heavy 1px stroke). */
export const FLOATING_EDGE =
  "border border-border/60 shadow-[0_1px_2px_rgba(0,0,0,0.04),0_8px_24px_-6px_rgba(0,0,0,0.14)] dark:shadow-[0_1px_2px_rgba(0,0,0,0.4),0_10px_28px_-6px_rgba(0,0,0,0.6)]";

/** Radius for menus/popovers. */
export const FLOATING_RADIUS = "rounded-xl";

/** Radius for modal surfaces (dialogs, alert dialogs). */
export const MODAL_RADIUS = "rounded-2xl";

/** Enter/exit motion shared by all anchored floating layers. */
export const FLOATING_MOTION =
  "data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 data-[side=bottom]:slide-in-from-top-1 data-[side=left]:slide-in-from-right-1 data-[side=right]:slide-in-from-left-1 data-[side=top]:slide-in-from-bottom-1 duration-150";

/** Complete anchored-layer surface (menus, popovers, selects). */
export const FLOATING_SURFACE = `bg-popover text-popover-foreground ${FLOATING_RADIUS} ${FLOATING_EDGE} ${FLOATING_MOTION}`;
