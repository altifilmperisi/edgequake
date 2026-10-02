/**
 * SPEC-157 W1 — layout honesty: the width budget, the splitter and the sheet.
 */
import { expect, test, type Page } from "@playwright/test";
import { askQuestion, DOC_ID, openFirstCitation, openQuery, pane, shot } from "./helpers";

const splitter = (page: Page) => page.getByRole("separator", { name: "Resize companion pane" });

async function width(page: Page, testId: string) {
  const box = await page.getByTestId(testId).boundingBox();
  return Math.round(box?.width ?? 0);
}

async function noHorizontalOverflow(page: Page) {
  const overflow = await page.evaluate(
    () => document.documentElement.scrollWidth - window.innerWidth,
  );
  expect(overflow).toBeLessThanOrEqual(1);
}

test.describe("SPEC-157 companion — layout @spec157", () => {
  for (const vw of [1280, 1440, 1920]) {
    test(`layout_budget_${vw}: chat, pane and history all stay usable`, async ({ page }) => {
      await openQuery(page, "/query", { viewport: { width: vw, height: 900 } });
      await askQuestion(page);
      await openFirstCitation(page);

      await expect(pane(page)).toHaveAttribute("data-companion-mode", "side");
      const paneW = await width(page, "query-companion");
      expect(paneW).toBeGreaterThanOrEqual(430);
      expect(paneW).toBeLessThanOrEqual(920);
      expect(await width(page, "query-composer")).toBeGreaterThanOrEqual(380);
      await noHorizontalOverflow(page);
      await shot(page, `10-layout-${vw}`);
    });
  }

  test("layout_splitter_keyboard: arrows resize, Home/End clamp, value is honest", async ({ page }) => {
    await openQuery(page);
    await askQuestion(page);
    await openFirstCitation(page);

    const handle = splitter(page);
    const start = Number(await handle.getAttribute("aria-valuenow"));
    const max0 = Number(await handle.getAttribute("aria-valuemax"));
    // The budget (not the global 920) caps the pane while history is docked.
    expect(max0).toBeLessThanOrEqual(920);
    const grown = Math.min(start + 100, max0);
    await handle.focus();
    await page.keyboard.press("Shift+ArrowLeft");
    await page.keyboard.press("Shift+ArrowLeft");
    await expect(handle).toHaveAttribute("aria-valuenow", String(grown));
    // The announced value matches what is painted.
    expect(Math.abs((await width(page, "query-companion")) - grown)).toBeLessThanOrEqual(2);

    await page.keyboard.press("Home");
    const min = Number(await handle.getAttribute("aria-valuemin"));
    await expect(handle).toHaveAttribute("aria-valuenow", String(min));
    await page.keyboard.press("End");
    const max = Number(await handle.getAttribute("aria-valuemax"));
    await expect(handle).toHaveAttribute("aria-valuenow", String(max));
    expect(await width(page, "query-composer")).toBeGreaterThanOrEqual(380);
    await noHorizontalOverflow(page);
  });

  test("layout_splitter_drag_persists: dragged width survives reopening", async ({ page }) => {
    await openQuery(page);
    await askQuestion(page);
    await openFirstCitation(page);

    const handle = splitter(page);
    const box = await handle.boundingBox();
    if (!box) throw new Error("splitter not laid out");
    const x = box.x + box.width / 2;
    const y = box.y + box.height / 2;
    await page.mouse.move(x, y);
    await page.mouse.down();
    await page.mouse.move(x - 120, y, { steps: 6 });
    await page.mouse.up();
    const dragged = Number(await handle.getAttribute("aria-valuenow"));
    expect(dragged).toBeGreaterThan(520);

    await page.goto(`/query?pane=pdf&doc=${DOC_ID}&page=2`);
    await expect(pane(page)).toBeVisible({ timeout: 30_000 });
    await expect(splitter(page)).toHaveAttribute("aria-valuenow", String(dragged));
  });

  test("layout_sheet_narrow: below the budget the pane becomes a bottom sheet", async ({ page }) => {
    await openQuery(page, "/query", { viewport: { width: 820, height: 900 } });
    await askQuestion(page);
    await openFirstCitation(page);

    await expect(pane(page)).toHaveAttribute("data-companion-mode", "sheet");
    await expect(splitter(page)).toHaveCount(0);
    const box = await pane(page).boundingBox();
    expect(box?.width ?? 0).toBeGreaterThanOrEqual(815);
    expect(Math.round((box?.y ?? 0) + (box?.height ?? 0))).toBeGreaterThanOrEqual(895);
    await noHorizontalOverflow(page);
    await shot(page, "11-layout-sheet");

    await pane(page).getByTestId("companion-close").focus();
    await page.keyboard.press("Escape");
    await expect(pane(page)).toHaveCount(0);
  });

  test("layout_resize_across_budget: shrinking the window flips side → sheet and back", async ({ page }) => {
    await openQuery(page);
    await askQuestion(page);
    await openFirstCitation(page);
    await expect(pane(page)).toHaveAttribute("data-companion-mode", "side");

    await page.setViewportSize({ width: 820, height: 900 });
    await expect(pane(page)).toHaveAttribute("data-companion-mode", "sheet");
    await page.setViewportSize({ width: 1600, height: 900 });
    await expect(pane(page)).toHaveAttribute("data-companion-mode", "side");
    // Same target throughout.
    await expect(page.getByTestId("pdf-page-indicator")).toHaveAttribute("data-page", "3");
  });

  test("layout_dark_theme: both panes read well in dark mode", async ({ page }) => {
    await page.addInitScript(() => localStorage.setItem("theme", "dark"));
    await openQuery(page);
    await askQuestion(page);
    await openFirstCitation(page);
    await expect(page.locator("html")).toHaveClass(/dark/);
    await expect(page.getByTestId("pdf-page-indicator")).toHaveAttribute("data-page", "3", { timeout: 30_000 });
    await shot(page, "14-dark-source");

    await page.getByTestId("show-on-graph").first().click();
    await expect(page.getByTestId("companion-graph-canvas").locator("canvas").first()).toBeVisible({
      timeout: 20_000,
    });
    await shot(page, "15-dark-graph");
  });
});
