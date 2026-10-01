import { describe, expect, it } from "vitest";
import {
  cursorFor,
  INITIAL_POINTER_STATE,
  transition,
  type PointerEffect,
  type PointerEvent,
  type PointerState,
} from "./pointer-fsm";

function run(events: PointerEvent[], from: PointerState = INITIAL_POINTER_STATE) {
  let state = from;
  const effects: PointerEffect[] = [];
  for (const event of events) {
    const t = transition(state, event);
    state = t.state;
    effects.push(...t.effects);
  }
  return { state, effects };
}

const at = (x: number, y: number) => ({ x, y });
const down = (button = 0, ctrl = false, dragEnabled = true): PointerEvent => ({
  type: "downNode",
  id: "n1",
  pos: at(10, 10),
  button,
  ctrl,
  dragEnabled,
});
const move = (x: number, y: number, primaryDown = true): PointerEvent => ({
  type: "move",
  pos: at(x, y),
  primaryDown,
});
const drags = (effects: PointerEffect[]) => effects.filter((e) => e.type === "dragTo");

describe("pointer FSM — drag", () => {
  it("a press that stays under the threshold is a click, not a drag", () => {
    const { state, effects } = run([down(), move(12, 11), { type: "up" }]);
    expect(drags(effects)).toHaveLength(0);
    expect(effects.some((e) => e.type === "swallowClick")).toBe(false);
    expect(state.mode).toBe("idle");
  });

  it("travel beyond the threshold drags and the release swallows the click", () => {
    const { state, effects } = run([down(), move(30, 10), move(40, 10), { type: "up" }]);
    expect(drags(effects)).toHaveLength(2);
    expect(effects.at(-1)).toEqual({ type: "swallowClick" });
    expect(state.mode).toBe("idle");
    expect(state.pressNode).toBeNull();
  });

  it("drag is disabled → press never arms", () => {
    const { effects } = run([down(0, false, false), move(100, 100)]);
    expect(drags(effects)).toHaveLength(0);
  });

  it("a move report with the primary button up ends the drag (missed mouseup)", () => {
    const { state, effects } = run([down(), move(40, 10), move(60, 10, false), move(80, 10)]);
    expect(drags(effects)).toHaveLength(1);
    expect(state.mode).toBe("idle");
  });

  it("cancel (window blur) ends a drag without a click", () => {
    const { state, effects } = run([down(), move(40, 10), { type: "cancel" }, move(90, 10)]);
    expect(drags(effects)).toHaveLength(1);
    expect(state.mode).toBe("idle");
  });
});

describe("pointer FSM — context menu (sticky-drag regression)", () => {
  it("a right press never arms a drag, even though Sigma reports downNode", () => {
    const { state, effects } = run([down(2), move(200, 200), move(300, 300)]);
    expect(drags(effects)).toHaveLength(0);
    expect(state.mode).toBe("idle");
  });

  it("macOS Ctrl+click is a context gesture, not a press", () => {
    const { effects } = run([down(0, true), move(200, 200)]);
    expect(drags(effects)).toHaveLength(0);
  });

  it("right-click then moving the pointer (no mouseup ever arrives) moves nothing", () => {
    const { state, effects } = run([
      { type: "enterNode", id: "n1" },
      down(2),
      { type: "contextMenu", id: "n1" },
      move(150, 150, false),
      move(300, 300, false),
      move(320, 320, true),
    ]);
    expect(drags(effects)).toHaveLength(0);
    expect(effects).toContainEqual({ type: "openMenu", id: "n1" });
    expect(state.mode).toBe("menu");
  });

  it("opening the menu mid-drag aborts the drag", () => {
    const { state, effects } = run([
      down(),
      move(40, 10),
      { type: "contextMenu", id: "n1" },
      move(80, 10),
    ]);
    expect(drags(effects)).toHaveLength(1);
    expect(state.mode).toBe("menu");
    expect(state.pressNode).toBeNull();
  });

  it("freezes hover and ignores a stray mouseup while the menu is open", () => {
    const open = run([{ type: "enterNode", id: "n1" }, { type: "contextMenu", id: "n1" }]);
    const { state, effects } = run(
      [{ type: "leaveNode" }, { type: "enterNode", id: "n2" }, { type: "up" }],
      open.state,
    );
    expect(effects).toEqual([]);
    expect(state.mode).toBe("menu");
    expect(state.hoverNode).toBe("n1");
  });

  it("closing the menu returns to idle and replays the pointer for fresh hover", () => {
    const open = run([{ type: "contextMenu", id: "n1" }]);
    const { state, effects } = run([{ type: "menuClosed" }], open.state);
    expect(state.mode).toBe("idle");
    expect(effects).toEqual([{ type: "replayPointer" }]);
  });

  it("right-click on empty canvas opens nothing", () => {
    const { state, effects } = run([{ type: "contextMenu", id: null }]);
    expect(effects).toEqual([]);
    expect(state.mode).toBe("idle");
  });
});

describe("pointer FSM — stage + cursor", () => {
  it("primary press on the stage pans; release returns to idle", () => {
    const pan = run([{ type: "downStage", pos: at(0, 0), button: 0, ctrl: false }]);
    expect(pan.state.mode).toBe("panning");
    expect(cursorFor(pan.state, true)).toBe("grabbing");
    expect(run([{ type: "up" }], pan.state).state.mode).toBe("idle");
  });

  it("cursor reflects what a press would do", () => {
    const hoverNode = run([{ type: "enterNode", id: "n1" }]).state;
    expect(cursorFor(hoverNode, true)).toBe("grab");
    expect(cursorFor(hoverNode, false)).toBe("pointer");
    expect(cursorFor(run([{ type: "enterEdge", id: "e1" }]).state, true)).toBe("pointer");
    expect(cursorFor(INITIAL_POINTER_STATE, true)).toBe("default");
    const menu = run([{ type: "enterNode", id: "n1" }, { type: "contextMenu", id: "n1" }]).state;
    expect(cursorFor(menu, true)).toBe("default");
  });
});
