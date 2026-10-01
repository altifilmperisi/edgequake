/**
 * Graph page chrome capture for live-stack visual polish (SPEC-155 Round 02).
 * @spec155
 */
import { test, expect } from "@playwright/test";
import {
  applyTheme,
  shotPath,
  type ShotTheme,
  type ShotViewport,
} from "./helpers/screenshots";

const THEMES: ShotTheme[] = ["light", "dark"];
const VIEWPORTS: ShotViewport[] = [375, 768, 1280];

async function settleGraph(page: import("@playwright/test").Page) {
  // Prefer canvas; fall back to empty/loading chrome
  await page.waitForTimeout(1200);
  await page
    .locator('[data-tour="graph-canvas"], [data-testid="graph-view-table-toggle"]')
    .first()
    .waitFor({ state: "visible", timeout: 30_000 })
    .catch(() => undefined);
  await page.waitForTimeout(800);
}

test.describe("Graph chrome capture @spec155", () => {
  test.describe.configure({ mode: "serial" });
  test.setTimeout(240_000);

  test("capture graph matrix + interactive states", async ({ page, browserName }) => {
    test.skip(browserName !== "chromium", "chromium-only");

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
        await settleGraph(page);
        const file = shotPath("round", "graph", theme, viewport, 2);
        await page.screenshot({ path: file, fullPage: false });
      }
    }

    // Interactive states at 1280 light
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/graph?stream=0", { waitUntil: "domcontentloaded" });
    await applyTheme(page, "light");
    await settleGraph(page);

    // Table view
    const tableToggle = page.getByTestId("graph-view-table-toggle");
    if (await tableToggle.isVisible()) {
      await tableToggle.click();
      await page.waitForTimeout(500);
      await page.screenshot({
        path: shotPath("round", "graph-table", "light", 1280, 2),
        fullPage: false,
      });
      await tableToggle.click();
      await page.waitForTimeout(400);
    }

    // Open legend if present
    const legend = page.locator('[data-tour="graph-canvas"]').locator("text=Legend").first();
    // Click a canvas node area first if possible
    const canvas = page.locator('[data-tour="graph-canvas"]');
    await expect(canvas).toBeVisible();
    await page.screenshot({
      path: shotPath("round", "graph-idle", "light", 1280, 2),
      fullPage: false,
    });

    // Mobile: open entity drawer + legend toggle
    await page.setViewportSize({ width: 375, height: 812 });
    await page.goto("/graph?stream=0", { waitUntil: "domcontentloaded" });
    await applyTheme(page, "light");
    await settleGraph(page);
    const menuBtn = page.getByRole("button", { name: /entity browser|open entity/i });
    if (await menuBtn.isVisible().catch(() => false)) {
      await menuBtn.click();
      await page.waitForTimeout(400);
      await page.screenshot({
        path: shotPath("round", "graph-entity-drawer", "light", 375, 2),
        fullPage: false,
      });
      await page.keyboard.press("Escape");
    }
    const legendBtn = page.getByRole("button", { name: /legend/i });
    if (await legendBtn.isVisible().catch(() => false)) {
      await legendBtn.click();
      await page.waitForTimeout(400);
      await page.screenshot({
        path: shotPath("round", "graph-legend", "light", 375, 2),
        fullPage: false,
      });
    }

    // Filters drawer
    const filterBtn = page.getByRole("button", { name: /open filters|filters/i }).first();
    if (await filterBtn.isVisible().catch(() => false)) {
      await filterBtn.click();
      await page.waitForTimeout(400);
      await page.screenshot({
        path: shotPath("round", "graph-filters-drawer", "light", 375, 2),
        fullPage: false,
      });
    }
  });
});
