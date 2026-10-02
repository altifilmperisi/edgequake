import { describe, expect, it } from "vitest";
import {
  CHAT_MIN_PX,
  COMPANION_DEFAULT_PX,
  COMPANION_MAX_PX,
  COMPANION_MIN_PX,
  HISTORY_OPEN_PX,
  resolveCompanionLayout,
} from "../companion-layout";

const base = {
  historyDocked: true,
  historyOpen: true,
  preferredWidth: COMPANION_DEFAULT_PX,
};

describe("resolveCompanionLayout (LAW-157-6)", () => {
  it("falls to a sheet when chat + companion cannot both fit", () => {
    const l = resolveCompanionLayout({ ...base, containerWidth: 800 });
    expect(l.mode).toBe("sheet");
  });

  it("uses the sheet on narrow viewports without docked history", () => {
    const l = resolveCompanionLayout({
      ...base,
      historyDocked: false,
      containerWidth: 700,
    });
    expect(l).toEqual({ mode: "sheet", history: "overlay" });
  });

  it("side pane exactly at the minimum budget, history railed", () => {
    const w = CHAT_MIN_PX + COMPANION_MIN_PX + 40;
    const l = resolveCompanionLayout({
      ...base,
      historyOpen: false,
      containerWidth: w,
    });
    expect(l).toMatchObject({ mode: "side", history: "rail" });
    if (l.mode === "side") {
      expect(l.widthPx).toBe(COMPANION_MIN_PX);
      expect(l.widthPx + 40 + CHAT_MIN_PX).toBeLessThanOrEqual(w);
    }
  });

  it("open history moves to the overlay when it would starve the panes", () => {
    const l = resolveCompanionLayout({ ...base, containerWidth: 1000 });
    expect(l).toMatchObject({ mode: "side", history: "overlay" });
  });

  it("keeps docked history when everything fits", () => {
    const w = CHAT_MIN_PX + COMPANION_MIN_PX + HISTORY_OPEN_PX + 100;
    const l = resolveCompanionLayout({ ...base, containerWidth: w });
    expect(l).toMatchObject({ mode: "side", history: "docked" });
    if (l.mode === "side") {
      expect(l.widthPx + HISTORY_OPEN_PX + CHAT_MIN_PX).toBeLessThanOrEqual(w);
    }
  });

  it("clamps the preferred width to [min, max]", () => {
    const wide = resolveCompanionLayout({
      ...base,
      historyOpen: false,
      containerWidth: 2400,
      preferredWidth: 5000,
    });
    const tiny = resolveCompanionLayout({
      ...base,
      historyOpen: false,
      containerWidth: 2400,
      preferredWidth: 10,
    });
    if (wide.mode !== "side" || tiny.mode !== "side") throw new Error("side");
    expect(wide.widthPx).toBe(COMPANION_MAX_PX);
    expect(tiny.widthPx).toBe(COMPANION_MIN_PX);
  });

  it("chat never drops below its minimum for any container width", () => {
    for (let w = 300; w <= 2600; w += 37) {
      for (const historyOpen of [true, false]) {
        const l = resolveCompanionLayout({ ...base, historyOpen, containerWidth: w });
        if (l.mode !== "side") continue;
        const reserve =
          l.history === "docked" ? HISTORY_OPEN_PX : l.history === "rail" ? 40 : 0;
        expect(w - reserve - l.widthPx).toBeGreaterThanOrEqual(CHAT_MIN_PX);
        expect(l.widthPx).toBeGreaterThanOrEqual(COMPANION_MIN_PX);
      }
    }
  });
});
