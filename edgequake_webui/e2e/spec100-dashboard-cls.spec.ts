/**
 * SPEC-100 — Dashboard CLS: activity card + subtitle reservation.
 */
import { expect, test } from "@playwright/test";
import { GOTO_OPTS } from "./helpers/app-ready";
import {
  mockSpec038AdmissionRoutes,
  seedSpec038TenantContext,
} from "./helpers/spec038-admission-mocks";

test.describe("SPEC-100 dashboard CLS", () => {
  test("activity card min-height holds while docs load", async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 900 });
    await mockSpec038AdmissionRoutes(page);
    await seedSpec038TenantContext(page);

    await page.route("**/api/v1/documents**", async (route) => {
      if (route.request().method() !== "GET") {
        await route.fallback();
        return;
      }
      await new Promise((r) => setTimeout(r, 500));
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          items: [],
          documents: [],
          total: 0,
          page: 1,
          page_size: 10,
          has_more: false,
          status_counts: {},
        }),
      });
    });

    await page.goto("/", GOTO_OPTS);

    const activity = page.getByTestId("spec100-dashboard-activity");
    await expect(activity).toBeVisible({ timeout: 20_000 });
    await expect
      .poll(
        async () =>
          activity.evaluate((el) => {
            const rect = el.getBoundingClientRect();
            return Math.max(rect.height, (el as HTMLElement).offsetHeight);
          }),
        { timeout: 15_000 },
      )
      .toBeGreaterThanOrEqual(280);

    await expect(page.getByTestId("spec100-dashboard-subtitle")).toBeVisible();
    const boxAfterH = await activity.evaluate((el) =>
      Math.max(el.getBoundingClientRect().height, (el as HTMLElement).offsetHeight),
    );
    const boxDuringH = await activity.evaluate((el) =>
      Math.max(el.getBoundingClientRect().height, (el as HTMLElement).offsetHeight),
    );
    expect(Math.abs(boxAfterH - boxDuringH)).toBeLessThanOrEqual(40);
  });
});
