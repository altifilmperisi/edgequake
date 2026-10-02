/**
 * SPEC-155 — Documents intake zones (dropzone + runs) in the docking workspace.
 * Legacy strip geometry tests live in documents-workspace.spec.ts.
 */
import { expect, test, type Page } from "@playwright/test";
import { prepareSpec155Page } from "./helpers/mock-api";

const RUN_A = "cccccccc-3333-4333-8333-cccccccccccc";

function liveDocs(): Record<string, unknown>[] {
  const now = new Date().toISOString();
  return [
    {
      id: RUN_A,
      title: "algo_2608.pdf",
      file_name: "algo_2608.pdf",
      status: "processing",
      current_stage: "converting",
      stage_progress: 0.1,
      source_type: "pdf",
      track_id: "track-intake",
      created_at: now,
      updated_at: now,
    },
  ];
}

async function ensureRunsExpanded(page: Page) {
  const zone = page.getByTestId("workspace-zone-runs");
  try {
    await expect(zone).toBeVisible({ timeout: 12_000 });
    return;
  } catch {
    /* still railed */
  }
  const rail = page.getByTestId("workspace-zone-rail-runs");
  if ((await rail.count()) > 0) {
    await rail
      .first()
      .evaluate((el) => (el as HTMLButtonElement).click())
      .catch(() => undefined);
  }
  await expect(zone).toBeVisible({ timeout: 12_000 });
}

async function expandIntakeWorking(page: Page) {
  await ensureRunsExpanded(page);
  const toggle = page.getByTestId("documents-intake-toggle");
  await expect(toggle).toBeVisible({ timeout: 30_000 });
  if ((await toggle.getAttribute("aria-expanded")) !== "true") {
    await toggle.click();
  }
}

test.describe("SPEC-155 documents intake layout @spec155", () => {
  test("idle: dropzone visible in Intake zone", async ({ page }) => {
    await prepareSpec155Page(page, { emptyDocs: true });
    await page.setViewportSize({ width: 1280, height: 720 });
    await page.goto("/documents", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("documents-workspace")).toBeVisible({
      timeout: 30_000,
    });
    await expect(page.getByTestId("document-dropzone")).toBeVisible();
    await expect(page.getByTestId("documents-intake-dropzone-slot")).toBeVisible();
  });

  test("live: Runs zone shows Active runs panel", async ({ page }) => {
    await prepareSpec155Page(page, { documents: liveDocs() });
    await page.setViewportSize({ width: 1280, height: 720 });
    await page.goto("/documents", { waitUntil: "domcontentloaded" });
    await ensureRunsExpanded(page);
    await expect(page.getByTestId("spec051-feedback-zone")).toBeVisible();
    await expect(page.getByTestId("spec048-active-runs-panel")).toBeVisible({
      timeout: 30_000,
    });
    await expandIntakeWorking(page);
    await expect(page.getByTestId("spec048-active-run-card")).toBeVisible();
  });

  test("inventory section remains present with live work", async ({ page }) => {
    await prepareSpec155Page(page, { documents: liveDocs() });
    await page.setViewportSize({ width: 1280, height: 900 });
    await page.goto("/documents", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("documents-inventory-section")).toBeVisible({
      timeout: 30_000,
    });
    await expect(page.getByTestId("workspace-zone-library")).toBeVisible();
    const inv = await page.getByTestId("documents-inventory-section").boundingBox();
    expect(inv).toBeTruthy();
    // Library keeps a usable band even with Runs expanded (zone mins + classic split).
    expect(inv!.height).toBeGreaterThan(80);
  });
});
