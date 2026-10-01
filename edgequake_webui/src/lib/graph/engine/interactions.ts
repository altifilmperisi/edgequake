/**
 * Sigma ⇄ pointer state machine adapter (see pointer-fsm.ts for the state
 * diagram and invariants). This file only translates DOM/Sigma events into FSM
 * events and executes the effects the FSM returns — no interaction rules live
 * here.
 *
 * Gesture contract:
 *  - hover node/edge → cursor + hover preview
 *  - primary press+move ≥ 4px on a node → drag; the release never clicks
 *  - click node → select, click empty canvas → clear
 *  - double-click node → "focus here" (zoom-to-cursor suppressed)
 *  - right-click (or Ctrl+click on macOS) node → context menu, never a drag
 *  - the native browser menu is always suppressed on the canvas
 */
import type Graph from "graphology";
import type Sigma from "sigma";
import {
  cursorFor,
  INITIAL_POINTER_STATE,
  transition,
  type PointerEffect,
  type PointerEvent,
  type PointerState,
} from "./pointer-fsm";

export { DRAG_THRESHOLD_PX } from "./pointer-fsm";

export interface InteractionTarget {
  graph: Graph;
  container: HTMLElement;
  isDragEnabled(): boolean;
  onHoverNode(nodeId: string | null): void;
  onHoverEdge(edgeId: string | null): void;
  onNodeClick(nodeId: string): void;
  onNodeDoubleClick(nodeId: string): void;
  onStageClick(): void;
  onNodeRightClick(nodeId: string, clientX: number, clientY: number): void;
}

export interface InteractionHandle {
  /** The context menu for the right-clicked node was dismissed. */
  menuClosed(): void;
  unbind(): void;
}

/** Sigma wraps mouse and touch events; the gestures below are mouse-only. */
function asMouse(original: MouseEvent | TouchEvent): MouseEvent | null {
  return original instanceof MouseEvent ? original : null;
}

export function bindInteractions(
  sigma: Sigma,
  target: InteractionTarget,
): InteractionHandle {
  const { graph, container } = target;
  const mouse = sigma.getMouseCaptor();

  let state: PointerState = INITIAL_POINTER_STATE;
  let swallowNextClick = false;
  let lastClient: { x: number; y: number } | null = null;

  const run = (effect: PointerEffect, trigger?: MouseEvent) => {
    switch (effect.type) {
      case "hoverNode":
        target.onHoverNode(effect.id);
        break;
      case "hoverEdge":
        target.onHoverEdge(effect.id);
        break;
      case "dragTo":
        if (graph.hasNode(effect.id)) {
          const p = sigma.viewportToGraph(effect.pos);
          graph.setNodeAttribute(effect.id, "x", p.x);
          graph.setNodeAttribute(effect.id, "y", p.y);
        }
        break;
      case "swallowClick":
        // The browser fires `click` right after `mouseup`; a drag must not select.
        swallowNextClick = true;
        setTimeout(() => {
          swallowNextClick = false;
        }, 0);
        break;
      case "openMenu":
        target.onNodeRightClick(
          effect.id,
          trigger?.clientX ?? lastClient?.x ?? 0,
          trigger?.clientY ?? lastClient?.y ?? 0,
        );
        break;
      case "replayPointer":
        if (lastClient) {
          container.dispatchEvent(
            new MouseEvent("mousemove", {
              bubbles: true,
              clientX: lastClient.x,
              clientY: lastClient.y,
            }),
          );
        }
        break;
    }
  };

  const dispatch = (event: PointerEvent, trigger?: MouseEvent) => {
    const next = transition(state, event);
    state = next.state;
    const cursor = cursorFor(state, target.isDragEnabled());
    container.style.cursor = cursor === "default" ? "" : cursor;
    for (const effect of next.effects) run(effect, trigger);
  };

  // --- hover ---------------------------------------------------------------
  sigma.on("enterNode", ({ node }) => dispatch({ type: "enterNode", id: node }));
  sigma.on("leaveNode", () => dispatch({ type: "leaveNode" }));
  sigma.on("enterEdge", ({ edge }) => dispatch({ type: "enterEdge", id: edge }));
  sigma.on("leaveEdge", () => dispatch({ type: "leaveEdge" }));

  // --- press / drag / pan --------------------------------------------------
  sigma.on("downNode", ({ node, event }) => {
    dispatch({
      type: "downNode",
      id: node,
      pos: { x: event.x, y: event.y },
      button: asMouse(event.original)?.button ?? 0,
      ctrl: asMouse(event.original)?.ctrlKey ?? false,
      dragEnabled: target.isDragEnabled(),
    });
  });

  sigma.on("downStage", ({ event }) => {
    dispatch({
      type: "downStage",
      pos: { x: event.x, y: event.y },
      button: asMouse(event.original)?.button ?? 0,
      ctrl: asMouse(event.original)?.ctrlKey ?? false,
    });
  });

  mouse.on("mousemovebody", (e) => {
    const original = asMouse(e.original);
    if (!original) return;
    dispatch({
      type: "move",
      pos: { x: e.x, y: e.y },
      primaryDown: (original.buttons & 1) === 1,
    });
    // A node press owns the gesture: the camera must not pan underneath it.
    if (state.mode === "pressed" || state.mode === "dragging") {
      e.preventSigmaDefault();
      original.preventDefault();
      original.stopPropagation();
    }
  });

  // Sigma never emits `mouseup` for the right button, so listen on the document.
  const onUp = () => dispatch({ type: "up" });
  const onCancel = () => dispatch({ type: "cancel" });
  const onPointerMove = (e: globalThis.MouseEvent) => {
    lastClient = { x: e.clientX, y: e.clientY };
  };
  document.addEventListener("mouseup", onUp);
  document.addEventListener("mousemove", onPointerMove, true);
  window.addEventListener("blur", onCancel);
  document.addEventListener("visibilitychange", onCancel);

  // --- clicks --------------------------------------------------------------
  sigma.on("clickNode", ({ node }) => {
    if (!swallowNextClick) target.onNodeClick(node);
  });
  sigma.on("clickStage", () => {
    if (!swallowNextClick) target.onStageClick();
  });
  sigma.on("doubleClickNode", (event) => {
    event.preventSigmaDefault();
    target.onNodeDoubleClick(event.node);
  });

  // --- context menu --------------------------------------------------------
  const suppressNativeMenu = (e: Event) => e.preventDefault();
  container.addEventListener("contextmenu", suppressNativeMenu);

  sigma.on("rightClickNode", ({ node, event }) =>
    dispatch({ type: "contextMenu", id: node }, asMouse(event.original) ?? undefined),
  );
  sigma.on("rightClickStage", () => dispatch({ type: "contextMenu", id: null }));

  return {
    menuClosed: () => dispatch({ type: "menuClosed" }),
    unbind: () => {
      document.removeEventListener("mouseup", onUp);
      document.removeEventListener("mousemove", onPointerMove, true);
      window.removeEventListener("blur", onCancel);
      document.removeEventListener("visibilitychange", onCancel);
      container.removeEventListener("contextmenu", suppressNativeMenu);
      container.style.cursor = "";
    },
  };
}
