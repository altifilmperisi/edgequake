/**
 * SPEC-155 regression — malformed `run_progress` on the wire must never turn
 * the Documents page into "Error loading documents … reading 'find'".
 */
import { expect, test } from "@playwright/test";
import { prepareSpec155Page } from "./helpers/mock-api";

const now = new Date().toISOString();

function row(over: Record<string, unknown>): Record<string, unknown> {
  return {
    id: "dddddddd-4444-4444-8444-dddddddddddd",
    title: "ledger.pdf",
    file_name: "ledger.pdf",
    status: "processing",
    current_stage: "extracting",
    stage_progress: 0.3,
    source_type: "pdf",
    track_id: "track-ledger",
    created_at: now,
    updated_at: now,
    ...over,
  };
}

test.describe("SPEC-155 malformed ledger @spec155", () => {
  test("ledger without phases/tasks keeps the list rendered", async ({ page }) => {
    const good = {
      seq: 2,
      phases: [{ id: "prepare", state: "active", tasks: [] }],
    };
    let call = 0;
    await prepareSpec155Page(page, { documents: [row({ run_progress: good })] });
    // Second and later polls return a truncated ledger (seq only / no tasks).
    await page.route("**/api/v1/documents?**", async (route) => {
      call += 1;
      const bad = call > 1 ? { seq: 3 } : good;
      await route.fulfill({
        json: {
          documents: [row({ run_progress: bad })],
          total: 1,
          page: 1,
          page_size: 20,
          total_pages: 1,
          has_more: false,
          status_counts: { pending: 0, processing: 1, completed: 0, failed: 0 },
        },
      });
    });
    await page.setViewportSize({ width: 1280, height: 720 });
    await page.goto("/documents", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("documents-inventory-section")).toBeVisible({
      timeout: 30_000,
    });
    await page.getByRole("button", { name: /refresh/i }).first().click();
    await page.waitForTimeout(500);
    await expect(page.getByText(/Error loading documents/i)).toHaveCount(0);
    await expect(page.getByText("ledger.pdf").first()).toBeVisible();
    await page.screenshot({ path: "test-results/spec155-polish.png" });
  });
});
