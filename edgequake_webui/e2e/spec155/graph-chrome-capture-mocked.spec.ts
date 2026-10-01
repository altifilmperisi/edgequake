/**
 * Graph chrome capture with mocked populated graph (SPEC-155 Round 02).
 * Uses Playwright webServer on :3010 so we don't fight make-dev for ports.
 * @spec155
 */
import { test } from "@playwright/test";
import { prepareSpec155Page } from "./helpers/mock-api";
import {
  applyTheme,
  shotPath,
  type ShotTheme,
  type ShotViewport,
} from "./helpers/screenshots";

const THEMES: ShotTheme[] = ["light", "dark"];
const VIEWPORTS: ShotViewport[] = [375, 768, 1280];

test.describe("Graph chrome capture (mocked data) @spec155", () => {
  test.describe.configure({ mode: "serial" });
  test.setTimeout(240_000);

  test("capture populated graph matrix", async ({ page, browserName }) => {
    test.skip(browserName !== "chromium", "chromium-only");
    await prepareSpec155Page(page);

    for (const theme of THEMES) {
      const vps: ShotViewport[] =
        theme === "light" ? VIEWPORTS : ([375, 1280] as ShotViewport[]);
      for (const viewport of vps) {
        await page.setViewportSize({
          width: viewport,
          height: viewport === 375 ? 812 : viewport === 768 ? 1024 : 800,
        });
        await page.goto("/graph?stream=0", { waitUntil: "domcontentloaded" });
        await applyTheme(page, theme);
        // Wait for sigma / nodes
        await page.waitForTimeout(2800);
        const file = shotPath("round", "graph-data", theme, viewport, 2);
        await page.screenshot({ path: file, fullPage: false });
      }
    }

    // Table + drawers
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/graph?stream=0&view=table", { waitUntil: "domcontentloaded" });
    await applyTheme(page, "light");
    await page.waitForTimeout(1500);
    await page.screenshot({
      path: shotPath("round", "graph-data-table", "light", 1280, 2),
      fullPage: false,
    });

    await page.setViewportSize({ width: 375, height: 812 });
    await page.goto("/graph?stream=0", { waitUntil: "domcontentloaded" });
    await applyTheme(page, "light");
    await page.waitForTimeout(2500);
    const legendBtn = page.getByRole("button", { name: /legend/i });
    if (await legendBtn.isVisible().catch(() => false)) {
      await legendBtn.click();
      await page.waitForTimeout(400);
      await page.screenshot({
        path: shotPath("round", "graph-data-legend", "light", 375, 2),
        fullPage: false,
      });
    }
    const menu = page.getByRole("button", { name: /entity browser|open entity/i });
    if (await menu.isVisible().catch(() => false)) {
      await menu.click();
      await page.waitForTimeout(400);
      await page.screenshot({
        path: shotPath("round", "graph-data-entity", "light", 375, 2),
        fullPage: false,
      });
    }
  });
});
