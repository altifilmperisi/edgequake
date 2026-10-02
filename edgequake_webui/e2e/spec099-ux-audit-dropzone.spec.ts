/**
 * SPEC-099 F-099-13 — Drop zone always present and locatable.
 */
import { expect, test } from "@playwright/test";
import { GOTO_OPTS } from "./helpers/app-ready";
import {
  mockSpec038AdmissionRoutes,
  seedSpec038TenantContext,
} from "./helpers/spec038-admission-mocks";
import {
  makeSpec086ListDoc,
  mockSpec086BusyPipeline,
  mockSpec086DocumentList,
} from "./helpers/spec086-ingestion-mocks";
import { ensureIntakeExpanded, expandIntakeWorking } from "./helpers/workspace-runs";

test.describe("SPEC-099 drop zone presence", () => {
  test("idle: document-dropzone visible, expandable, accepts file input", async ({
    page,
  }) => {
    await mockSpec038AdmissionRoutes(page);
    await seedSpec038TenantContext(page);
    await mockSpec086DocumentList(page, []);
    await page.goto("/documents", GOTO_OPTS);
    await ensureIntakeExpanded(page);

    const dropzone = page.getByTestId("document-dropzone");
    await expect(dropzone).toBeVisible({ timeout: 20_000 });
    await expect(dropzone).toHaveAttribute("data-collapsed", "false");
    await expect(dropzone).toHaveAttribute("data-upload", "true");
    await expect(dropzone.locator('input[type="file"]')).toHaveCount(1);
    // Full-width band, not a tiny icon chip
    const box = await dropzone.boundingBox();
    expect(box).toBeTruthy();
    expect(box!.width).toBeGreaterThan(200);
  });

  test("busy: dropzone still present when Active runs open (collapsed band)", async ({
    page,
  }) => {
    const doc = makeSpec086ListDoc({
      id: "doc-099-dz-busy",
      file_name: "busy.pdf",
      status: "processing",
      current_stage: "extracting",
      stage_message: "Extracting",
      stage_progress: 0.4,
      track_id: "track-099-dz-busy",
      admission_staging: false,
    });
    await mockSpec038AdmissionRoutes(page);
    await seedSpec038TenantContext(page);
    await mockSpec086BusyPipeline(page);
    await mockSpec086DocumentList(page, [doc]);
    await page.goto("/documents", GOTO_OPTS);
    await expandIntakeWorking(page);

    await expect(page.getByTestId("spec048-active-runs-panel")).toBeVisible({
      timeout: 20_000,
    });
    const dropzone = page.getByTestId("document-dropzone");
    await expect(dropzone).toBeVisible();
    // SPEC-155: quiet, not collapsed — Upload is its own zone.
    await expect(dropzone).toHaveAttribute("data-quiet", "true");
    await expect(dropzone).toHaveAttribute("data-collapsed", "false");
    await expect(dropzone.locator('input[type="file"]')).toHaveCount(1);
    const box = await dropzone.boundingBox();
    expect(box).toBeTruthy();
    expect(box!.width).toBeGreaterThan(200);
  });
});
