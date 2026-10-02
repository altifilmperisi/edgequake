/**
 * SPEC-155 — a half-finished delete (`delete_failed`) is a lifecycle problem,
 * not a live ingest run: no Active-run card, no Cancel, no Reprocess — the only
 * offered action is "Finish delete".
 */
import { expect, test } from "@playwright/test";
import { prepareSpec155Page } from "./helpers/mock-api";

const ID = "dddddddd-4444-4444-8444-dddddddddddd";

function deleteFailedDocs(): Record<string, unknown>[] {
  const now = new Date().toISOString();
  return [
    {
      id: ID,
      title: "001_2608.06377v1.pdf",
      file_name: "001_2608.06377v1.pdf",
      status: "delete_failed",
      // Stale ingest fields the server keeps on the half-deleted row.
      current_stage: "materialize",
      stage_message:
        "Delete did not finish — retry the delete, or re-upload the file as a new document.",
      source_type: "pdf",
      track_id: "track-155-old",
      created_at: now,
      updated_at: now,
    },
  ];
}

test.describe("SPEC-155 delete_failed is not an active run @spec155", () => {
  test("no run card; menu offers Finish delete, not Reprocess", async ({ page }) => {
    await prepareSpec155Page(page, { documents: deleteFailedDocs() });
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto("/documents", { waitUntil: "domcontentloaded" });

    const row = page.getByRole("row", { name: /001_2608\.06377v1\.pdf/ });
    await expect(row).toBeVisible({ timeout: 30_000 });
    await expect(row).toContainText("Delete failed");

    await expect(page.getByTestId("spec048-active-runs-panel")).toHaveCount(0);
    await expect(page.getByTestId("spec048-active-run-card")).toHaveCount(0);

    await row.getByRole("button", { name: "More actions" }).click();
    await expect(page.getByRole("menuitem", { name: "Finish delete" })).toBeVisible();
    await expect(page.getByRole("menuitem", { name: /^Reprocess$/ })).toHaveCount(0);
    await expect(page.getByRole("menuitem", { name: /Cancel/ })).toHaveCount(0);
  });
});
