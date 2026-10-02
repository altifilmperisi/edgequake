import { describe, expect, test } from "bun:test";
import {
  FOCUS_STRIP_MAX_REM,
  LAYOUT_MODE_STORAGE_KEY,
  LAYOUT_SNAP_TO_SPLIT_PX,
  LAYOUT_SNAP_TO_STACK_PX,
  cycleLayoutMode,
  focusStripMaxHeightCss,
  parseDocumentsLayoutMode,
  readLayoutMode,
  resolveIntakeLayout,
  snapLayoutModeFromDrag,
  writeLayoutMode,
} from "../documents-layout-mode";

function memStorage() {
  const mem = new Map<string, string>();
  return {
    getItem: (k: string) => mem.get(k) ?? null,
    setItem: (k: string, v: string) => {
      mem.set(k, v);
    },
    removeItem: (k: string) => {
      mem.delete(k);
    },
    _mem: mem,
  } as Storage & { _mem: Map<string, string> };
}

describe("documents-layout-mode", () => {
  test("parseDocumentsLayoutMode: valid + invalid fallback to focus", () => {
    expect(parseDocumentsLayoutMode("focus")).toBe("focus");
    expect(parseDocumentsLayoutMode("split")).toBe("split");
    expect(parseDocumentsLayoutMode("stack")).toBe("stack");
    expect(parseDocumentsLayoutMode(null)).toBe("focus");
    expect(parseDocumentsLayoutMode("")).toBe("focus");
    expect(parseDocumentsLayoutMode("dashboard")).toBe("focus");
  });

  test("read/writeLayoutMode persistence + bad storage", () => {
    const storage = memStorage();
    expect(readLayoutMode(storage)).toBe("focus");
    writeLayoutMode("split", storage);
    expect(storage.getItem(LAYOUT_MODE_STORAGE_KEY)).toBe("split");
    expect(readLayoutMode(storage)).toBe("split");
    writeLayoutMode("stack", storage);
    expect(readLayoutMode(storage)).toBe("stack");

    const throwing = {
      getItem: () => {
        throw new Error("blocked");
      },
      setItem: () => {
        throw new Error("blocked");
      },
    } as unknown as Storage;
    expect(readLayoutMode(throwing)).toBe("focus");
    expect(() => writeLayoutMode("focus", throwing)).not.toThrow();
  });

  test("resolveIntakeLayout matrix", () => {
    expect(resolveIntakeLayout("focus", false)).toEqual({
      stripVariant: "solo",
      showExpandedBudget: false,
    });
    expect(resolveIntakeLayout("split", false)).toEqual({
      stripVariant: "solo",
      showExpandedBudget: false,
    });
    expect(resolveIntakeLayout("stack", false)).toEqual({
      stripVariant: "solo",
      showExpandedBudget: false,
    });
    expect(resolveIntakeLayout("focus", true)).toEqual({
      stripVariant: "chip",
      showExpandedBudget: false,
    });
    expect(resolveIntakeLayout("split", true)).toEqual({
      stripVariant: "split",
      showExpandedBudget: true,
    });
    expect(resolveIntakeLayout("stack", true)).toEqual({
      stripVariant: "stack",
      showExpandedBudget: true,
    });
  });

  test("snapLayoutModeFromDrag compress / expand thresholds", () => {
    expect(snapLayoutModeFromDrag("split", 0)).toBe("split");
    expect(snapLayoutModeFromDrag("split", LAYOUT_SNAP_TO_SPLIT_PX)).toBe(
      "focus",
    );
    expect(snapLayoutModeFromDrag("stack", LAYOUT_SNAP_TO_SPLIT_PX)).toBe(
      "split",
    );
    expect(snapLayoutModeFromDrag("stack", LAYOUT_SNAP_TO_STACK_PX)).toBe(
      "focus",
    );
    expect(snapLayoutModeFromDrag("focus", -LAYOUT_SNAP_TO_SPLIT_PX)).toBe(
      "split",
    );
    expect(snapLayoutModeFromDrag("focus", -LAYOUT_SNAP_TO_STACK_PX)).toBe(
      "stack",
    );
    expect(snapLayoutModeFromDrag("split", -LAYOUT_SNAP_TO_SPLIT_PX)).toBe(
      "stack",
    );
  });

  test("cycleLayoutMode", () => {
    expect(cycleLayoutMode("focus", 1)).toBe("split");
    expect(cycleLayoutMode("split", 1)).toBe("stack");
    expect(cycleLayoutMode("stack", 1)).toBe("focus");
    expect(cycleLayoutMode("focus", -1)).toBe("stack");
  });

  test("focusStripMaxHeightCss", () => {
    expect(focusStripMaxHeightCss()).toBe(`${FOCUS_STRIP_MAX_REM}rem`);
  });
});
