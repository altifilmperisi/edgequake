/**
 * SPEC-155 axe smoke (@a11y @spec155) — LAW-155-14 / EC-155-76
 */
import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";
import { prepareSpec155Page } from "./helpers/mock-api";

const ROUTES = [
  "/",
  "/documents",
  "/query",
  "/graph?stream=0",
  "/pipeline",
  "/costs",
  "/workspace",
  "/knowledge",
  "/settings",
  "/login",
];

test.describe("SPEC-155 axe routes @a11y @spec155", () => {
  for (const route of ROUTES) {
    test(`axe ${route}`, async ({ page }) => {
      await prepareSpec155Page(page);
      await page.goto(route, { waitUntil: "domcontentloaded" });
      await page.waitForTimeout(500);
      // Env toasts (WS reconnect) are not product chrome — exclude from axe surface.
      await page.evaluate(() => {
        document.querySelectorAll("[data-sonner-toast]").forEach((el) => el.remove());
      });
      const results = await new AxeBuilder({ page })
        .exclude("[data-sonner-toaster]")
        .withTags(["wcag2a", "wcag2aa", "wcag21aa", "wcag22aa"])
        .analyze();
      const serious = results.violations.filter(
        (v) => v.impact === "serious" || v.impact === "critical",
      );
      expect(
        serious,
        serious.map((v) => `${v.id}: ${v.help}`).join("\n"),
      ).toEqual([]);
    });
  }
});
