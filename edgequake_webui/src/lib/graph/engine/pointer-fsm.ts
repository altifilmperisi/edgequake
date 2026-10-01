/**
 * Pointer interaction state machine for the graph canvas (pure, no DOM/Sigma).
 *
 * ```
 *                    downNode(primary)               move ≥ 4px
 *            ┌──────────────────────────► PRESSED ───────────────► DRAGGING
 *            │                              │  │                     │  │
 *            │                       up ────┘  │ up (click proceeds) │  │ up → swallow the click
 *            │                                 │                     │  │
 *   IDLE ◄───┼─────────────────────────────────┴─────────────────────┘  │
 *    ▲ │ ▲   │                                                           │
 *    │ │ │   │ downStage(primary) ──► PANNING ── up ──► IDLE             │
 *    │ │ └───┼───────────────────────── cancel (blur / buttons==0) ◄─────┘
 *    │ │     │
 *    │ │  contextMenu(node) from ANY mode (aborts press/drag)
 *    │ └───────────────────────────► MENU(node) ── menuClosed ──► IDLE (+ replay pointer)
 *    └──────────────────────────────── contextMenu(stage) ─ suppressed, stays in mode IDLE
 * ```
 *
 * Invariants (each one is a regression test in pointer-fsm.test.ts):
 *  1. A node moves ONLY in DRAGGING, which is entered ONLY from PRESSED by a
 *     primary press that travelled ≥ DRAG_THRESHOLD_PX.
 *  2. Only a primary, un-modified press arms PRESSED. Right press and macOS
 *     Ctrl+click are context gestures: they never arm a drag (Sigma emits
 *     `downNode` for every button but never a `mouseup` for the right one, which
 *     is what used to leave the node glued to the pointer — "sticky drag").
 *  3. Every way out of a press/drag exists: up, cancel (blur), a move report
 *     whose primary button is no longer down, and contextMenu.
 *  4. While MENU is open the pointer is frozen: no hover changes, no drag.
 *  5. A drag never produces a click (swallowClick on drag release).
 */

export const DRAG_THRESHOLD_PX = 4;

export type PointerMode = "idle" | "pressed" | "dragging" | "panning" | "menu";

export interface Pt {
  x: number;
  y: number;
}

export interface PointerState {
  mode: PointerMode;
  hoverNode: string | null;
  hoverEdge: string | null;
  /** Node under the primary press (PRESSED / DRAGGING). */
  pressNode: string | null;
  origin: Pt | null;
  /** Node the open context menu belongs to (MENU). */
  menuNode: string | null;
}

export type PointerEvent =
  | { type: "enterNode"; id: string }
  | { type: "leaveNode" }
  | { type: "enterEdge"; id: string }
  | { type: "leaveEdge" }
  | {
      type: "downNode";
      id: string;
      pos: Pt;
      button: number;
      ctrl: boolean;
      dragEnabled: boolean;
    }
  | { type: "downStage"; pos: Pt; button: number; ctrl: boolean }
  | { type: "move"; pos: Pt; primaryDown: boolean }
  | { type: "up" }
  | { type: "cancel" }
  | { type: "contextMenu"; id: string | null }
  | { type: "menuClosed" };

export type PointerEffect =
  | { type: "hoverNode"; id: string | null }
  | { type: "hoverEdge"; id: string | null }
  | { type: "dragTo"; id: string; pos: Pt }
  | { type: "swallowClick" }
  | { type: "openMenu"; id: string }
  /** Re-run Sigma's hover hit-test at the last pointer position. */
  | { type: "replayPointer" };

export interface Transition {
  state: PointerState;
  effects: PointerEffect[];
}

export const INITIAL_POINTER_STATE: PointerState = {
  mode: "idle",
  hoverNode: null,
  hoverEdge: null,
  pressNode: null,
  origin: null,
  menuNode: null,
};

const PRIMARY = 0;

/** Clears press/drag bookkeeping while keeping hover. */
function released(state: PointerState, mode: PointerMode = "idle"): PointerState {
  return { ...state, mode, pressNode: null, origin: null, menuNode: null };
}

function stay(state: PointerState): Transition {
  return { state, effects: [] };
}

function onHover(state: PointerState, event: PointerEvent): Transition {
  if (state.mode === "menu") return stay(state); // invariant 4
  switch (event.type) {
    case "enterNode":
      return {
        state: { ...state, hoverNode: event.id },
        effects: [{ type: "hoverNode", id: event.id }],
      };
    case "leaveNode":
      return {
        state: { ...state, hoverNode: null },
        effects: [{ type: "hoverNode", id: null }],
      };
    case "enterEdge":
      return {
        state: { ...state, hoverEdge: event.id },
        effects: [{ type: "hoverEdge", id: event.id }],
      };
    default:
      return {
        state: { ...state, hoverEdge: null },
        effects: [{ type: "hoverEdge", id: null }],
      };
  }
}

function onDownNode(
  state: PointerState,
  event: Extract<PointerEvent, { type: "downNode" }>,
): Transition {
  if (state.mode === "menu") return stay(state);
  const primary = event.button === PRIMARY && !event.ctrl; // invariant 2
  if (!primary || !event.dragEnabled) return stay(released(state));
  return stay({ ...released(state, "pressed"), pressNode: event.id, origin: event.pos });
}

function onMove(
  state: PointerState,
  event: Extract<PointerEvent, { type: "move" }>,
): Transition {
  const { mode, pressNode, origin } = state;
  if (mode !== "pressed" && mode !== "dragging" && mode !== "panning") {
    return stay(state);
  }
  // Missed mouseup (released outside the window, over a menu…): end gracefully.
  if (!event.primaryDown) return onUp(state);

  if (mode === "pressed" && pressNode && origin) {
    const travelled = Math.hypot(event.pos.x - origin.x, event.pos.y - origin.y);
    if (travelled < DRAG_THRESHOLD_PX) return stay(state);
    return {
      state: { ...state, mode: "dragging" },
      effects: [{ type: "dragTo", id: pressNode, pos: event.pos }],
    };
  }
  if (mode === "dragging" && pressNode) {
    return { state, effects: [{ type: "dragTo", id: pressNode, pos: event.pos }] };
  }
  return stay(state);
}

function onUp(state: PointerState): Transition {
  switch (state.mode) {
    case "dragging":
      return { state: released(state), effects: [{ type: "swallowClick" }] }; // invariant 5
    case "pressed":
    case "panning":
      return stay(released(state));
    default:
      return stay(state); // idle, or MENU (a stray mouseup must not close it)
  }
}

function onContextMenu(
  state: PointerState,
  event: Extract<PointerEvent, { type: "contextMenu" }>,
): Transition {
  if (state.mode === "menu") return stay(state);
  const aborted = released(state);
  if (event.id === null) return stay(aborted);
  return {
    state: { ...aborted, mode: "menu", menuNode: event.id },
    effects: [{ type: "openMenu", id: event.id }],
  };
}

export function transition(state: PointerState, event: PointerEvent): Transition {
  switch (event.type) {
    case "enterNode":
    case "leaveNode":
    case "enterEdge":
    case "leaveEdge":
      return onHover(state, event);
    case "downNode":
      return onDownNode(state, event);
    case "downStage":
      if (state.mode === "menu") return stay(state);
      return stay(
        event.button === PRIMARY && !event.ctrl
          ? released(state, "panning")
          : released(state),
      );
    case "move":
      return onMove(state, event);
    case "up":
      return onUp(state);
    case "cancel":
      return stay(state.mode === "menu" ? state : released(state));
    case "contextMenu":
      return onContextMenu(state, event);
    case "menuClosed":
      if (state.mode !== "menu") return { state, effects: [{ type: "replayPointer" }] };
      return { state: released(state), effects: [{ type: "replayPointer" }] };
  }
}

export type CursorKind = "default" | "pointer" | "grab" | "grabbing";

/** The pointer always says what a press would do. */
export function cursorFor(state: PointerState, dragEnabled: boolean): CursorKind {
  if (state.mode === "dragging" || state.mode === "panning") return "grabbing";
  if (state.mode === "menu") return "default";
  if (state.hoverNode) return dragEnabled ? "grab" : "pointer";
  if (state.hoverEdge) return "pointer";
  return "default";
}
