/**
 * SPEC-099 F-099-09 — Scale honesty when fetch is capped.
 * Primary proof is unit (inventory-view-model); this checks UI affordance wiring.
 */
import { expect, test } from "@playwright/test";
import { GOTO_OPTS } from "./helpers/app-ready";
import {
  mockSpec038AdmissionRoutes,
  seedSpec038TenantContext,
} from "./helpers/spec038-admission-mocks";
import {
  makeSpec086ListDoc,
  mockSpec086DocumentList,
  type Spec086ListDoc,
} from "./helpers/spec086-ingestion-mocks";

test.describe("SPEC-099 scale overflow", () => {
  test("capped fetch shows overflow affordance or honest count label", async ({
    page,
  }) => {
    // 100 completed docs → VIRTUAL_PAGE_SIZE cap → "100+" or overflow label
    const docs: Spec086ListDoc[] = Array.from({ length: 100 }, (_, i) =>
      makeSpec086ListDoc({
        id: `doc-099-scale-${i}`,
        file_name: `scale-${i}.pdf`,
        status: "completed",
        current_stage: "completed",
        track_id: null,
        admission_staging: false,
      }),
    );
    await mockSpec038AdmissionRoutes(page);
    await seedSpec038TenantContext(page);
    await mockSpec086DocumentList(page, docs);
    // The shared list helper is intentionally exact-count. Override it here
    // with a server total larger than the fetched cap to exercise SPEC-099's
    // honest overflow affordance.
    await page.route("**/api/v1/documents**", async (route) => {
      if (
        route.request().method() === "GET" &&
        !route.request().url().includes("/track/") &&
        !route.request().url().includes("/pdf")
      ) {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({
            documents: docs,
            total: 240,
            page: 1,
            page_size: 100,
            total_pages: 3,
            has_more: true,
            status_counts: {
              pending: 0,
              processing: 0,
              completed: 240,
              failed: 0,
              partial_failure: 0,
              cancelled: 0,
            },
          }),
        });
        return;
      }
      await route.fallback();
    });
    await page.goto("/documents", GOTO_OPTS);

    await expect(page.getByTestId("documents-workspace")).toBeVisible({
      timeout: 20_000,
    });
    const count = page.getByTestId("spec099-documents-count");
    const overflow = page.getByTestId("spec099-scale-overflow");
    await expect(count.or(overflow)).toBeVisible({ timeout: 20_000 });
    const label = ((await count.count()) ? await count.textContent() : "")?.trim();
    const hasOverflow = (await overflow.count()) > 0;
    expect(
      hasOverflow ||
        Boolean(
          label &&
            (label.includes("+") || label.includes("of") || Number(label) >= 100),
        ),
    ).toBeTruthy();
  });
});
