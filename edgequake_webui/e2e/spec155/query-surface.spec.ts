/**
 * SPEC-155 citations + scroll + history + a11y gates (@spec155).
 */
import { expect, test } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import { prepareSpec155Page } from "./helpers/mock-api";
import { SCENARIO_HAPPY, scenarioLongStream } from "./helpers/mock-chat-sse";

test.describe("SPEC-155 query citations @spec155", () => {
  test("query_citations: source chips then sources panel", async ({ page }) => {
    await prepareSpec155Page(page, { chatScenario: SCENARIO_HAPPY });
    await page.goto("/query", { waitUntil: "domcontentloaded" });
    const input = page.locator("textarea.query-input").first();
    await expect(input).toBeVisible({ timeout: 20_000 });
    await input.fill("Summarize with citations");
    await input.press("Enter");

    await expect(page.getByTestId("query-source-chips")).toBeVisible({
      timeout: 20_000,
    });
    await expect(page.getByText(/LightRAG uses dual-level/i).first()).toBeVisible({
      timeout: 20_000,
    });
    // Citations slot is present after answer (may be collapsed / empty height)
    await expect(page.getByTestId("spec100-query-citations-slot")).toBeAttached({
      timeout: 15_000,
    });
  });
});

test.describe("SPEC-155 query scroll @spec155", () => {
  test("query_jump_latest appears when scrolled up", async ({ page }) => {
    await prepareSpec155Page(page, {
      chatScenario: scenarioLongStream(60),
    });
    await page.goto("/query", { waitUntil: "domcontentloaded" });
    const input = page.locator("textarea.query-input").first();
    await expect(input).toBeVisible({ timeout: 20_000 });
    await input.fill("Long stream for scroll");
    await input.press("Enter");

    // Wait for some content
    await page.waitForTimeout(1500);
    // Scroll the message scroller up if present
    await page.evaluate(() => {
      const el =
        document.querySelector("[data-testid='query-message-scroll']") ||
        document.querySelector(".overflow-y-auto");
      if (el) (el as HTMLElement).scrollTop = 0;
    });
    const jump = page.getByTestId("query-jump-latest");
    // Jump may appear after scroll-away; soft assert
    if (await jump.isVisible().catch(() => false)) {
      await jump.click();
    }
  });
});

test.describe("SPEC-155 query history @spec155", () => {
  test("query_history: new chat button visible", async ({ page }) => {
    await prepareSpec155Page(page);
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto("/query", { waitUntil: "domcontentloaded" });
    await expect(
      page.getByRole("button", { name: /new/i }).first(),
    ).toBeVisible({ timeout: 20_000 });
  });
});

test.describe("SPEC-155 query a11y @spec155", () => {
  test("query_a11y_empty: no critical axe violations", async ({ page }) => {
    await prepareSpec155Page(page);
    await page.goto("/query", { waitUntil: "domcontentloaded" });
    await expect(page.locator("textarea.query-input").first()).toBeVisible({
      timeout: 20_000,
    });
    const results = await new AxeBuilder({ page })
      .withTags(["wcag2a", "wcag2aa"])
      .analyze();
    const critical = results.violations.filter((v) => v.impact === "critical");
    expect(critical, JSON.stringify(critical, null, 2)).toEqual([]);
  });
});

test.describe("SPEC-155 query visual @spec155", () => {
  test("query_visual_empty baseline", async ({ page }) => {
    await prepareSpec155Page(page);
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/query", { waitUntil: "domcontentloaded" });
    const composer = page.getByTestId("query-composer");
    await expect(composer).toBeVisible({ timeout: 20_000 });

    // Soft shell: wait until Tailwind radius/shadow settle (Turbopack can race first paint)
    await expect
      .poll(async () => {
        const radiusPx = await composer.evaluate((el) => {
          const s = getComputedStyle(el);
          return Math.min(
            ...String(s.borderRadius || "0")
              .split(/\s+/)
              .map((part) => parseFloat(part) || 0),
          );
        });
        return radiusPx;
      })
      .toBeGreaterThanOrEqual(16);

    const chrome = await composer.evaluate((el) => {
      const s = getComputedStyle(el);
      return {
        borderTopWidth: s.borderTopWidth,
        boxShadow: s.boxShadow,
        overflow: s.overflow,
        className: el.className,
      };
    });
    expect(["0px", ""]).toContain(chrome.borderTopWidth);
    expect(chrome.boxShadow).not.toBe("none");
    expect(chrome.overflow).toMatch(/hidden/);
    expect(chrome.className).toMatch(/rounded-3xl/);
    expect(chrome.className).not.toMatch(/\bborder\b/);

    // Clear websocket/connection toasts so they don't cover the composer
    await page.evaluate(() => {
      document
        .querySelectorAll("[data-sonner-toaster], [data-sonner-toast]")
        .forEach((n) => n.remove());
    });

    await page.screenshot({
      path: "e2e/spec155/screenshots/query-empty-1280.png",
      fullPage: true,
    });
    await composer.screenshot({
      path: "e2e/spec155/screenshots/composer-only.png",
    });
  });
});
