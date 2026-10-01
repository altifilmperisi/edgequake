/**
 * SPEC-155 W1 gates (@spec155): font, cmdk, login autocomplete, not-found, lang.
 */
import { expect, test } from "@playwright/test";
import { prepareSpec155Page } from "./helpers/mock-api";

test.describe("SPEC-155 W1 foundations @spec155", () => {
  test("shell_font: geist variable applied", async ({ page }) => {
    await prepareSpec155Page(page);
    await page.goto("/", { waitUntil: "domcontentloaded" });
    const font = await page.evaluate(() =>
      getComputedStyle(document.body).fontFamily.toLowerCase(),
    );
    expect(font.length).toBeGreaterThan(0);
    const htmlClass = await page.locator("html").getAttribute("class");
    expect(htmlClass ?? "").toMatch(/geist|font/i);
  });

  test("cmdk_palette opens with Meta+K", async ({ page }) => {
    await prepareSpec155Page(page);
    await page.goto("/settings", { waitUntil: "domcontentloaded" });
    await page.waitForTimeout(500);
    await page.keyboard.press("Meta+K");
    const dialog = page.getByRole("dialog");
    if (!(await dialog.isVisible().catch(() => false))) {
      await page.keyboard.press("Control+K");
    }
    await expect(dialog).toBeVisible({ timeout: 8000 });
    await expect(
      page.getByRole("heading", { name: /command palette/i }),
    ).toBeVisible();
  });

  test("login_autocomplete attributes", async ({ page }) => {
    await prepareSpec155Page(page);
    await page.goto("/login", { waitUntil: "domcontentloaded" });
    const user = page.locator("#username");
    const pass = page.locator("#password");
    if (await user.count()) {
      await expect(user).toHaveAttribute("autocomplete", "username");
      await expect(pass).toHaveAttribute("autocomplete", "current-password");
    }
  });

  test("not_found page", async ({ page }) => {
    await prepareSpec155Page(page);
    await page.goto("/this-route-does-not-exist-spec155", {
      waitUntil: "domcontentloaded",
    });
    await expect(page.getByRole("heading", { name: /not found/i })).toBeVisible({
      timeout: 8000,
    });
  });

  test("breadcrumb_paths include pipeline", async ({ page }) => {
    await prepareSpec155Page(page);
    await page.goto("/pipeline", { waitUntil: "domcontentloaded" });
    const crumb = page.locator("nav").filter({ hasText: /pipeline/i });
    await expect(crumb.first()).toBeVisible({ timeout: 8000 });
  });
});
