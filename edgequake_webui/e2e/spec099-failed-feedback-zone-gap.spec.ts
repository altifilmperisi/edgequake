/**
 * SPEC-099 — Ordinary Failed must not leave an empty feedback-zone band
 * between the dropzone and the documents inventory (CLS reserve leak).
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

async function dropzoneToInventoryGapPx(
  page: import("@playwright/test").Page,
): Promise<number> {
  const dropzone = page.getByTestId("document-dropzone");
  const inventory = page.getByTestId("documents-inventory-section");
  const dropBox = await dropzone.boundingBox();
  const invBox = await inventory.boundingBox();
  expect(dropBox).toBeTruthy();
  expect(invBox).toBeTruthy();
  return invBox!.y - (dropBox!.y + dropBox!.height);
}

test.describe("SPEC-099 Failed feedback-zone gap", () => {
  test("ordinary Failed: no feedback zone; dropzone→table gap tight", async ({
    page,
  }) => {
    await page.setViewportSize({ width: 1280, height: 800 });
    const failed = makeSpec086ListDoc({
      id: "doc-099-failed-gap",
      file_name: "ts_rag_2608.06223v1.md",
      status: "failed",
      current_stage: "failed",
      stage_message: "Pipeline processing failed: entity extraction timeout",
      error_message: "Pipeline processing failed: entity extraction timeout",
      track_id: null,
      admission_staging: false,
      source_type: "markdown",
    });
    await mockSpec038AdmissionRoutes(page);
    await seedSpec038TenantContext(page);
    await mockSpec086DocumentList(page, [failed]);
    await page.goto("/documents", GOTO_OPTS);

    await expect(page.getByTestId("document-dropzone")).toBeVisible({
      timeout: 20_000,
    });
    await expect(page.getByText("Failed", { exact: true })).toBeVisible();

    await expect(page.getByTestId("spec051-feedback-zone")).toHaveCount(0);
    await expect(page.getByTestId("spec048-active-runs-panel")).toHaveCount(0);
    await expect(page.getByTestId("document-dropzone")).toHaveAttribute(
      "data-collapsed",
      "false",
    );

    const gapIdle = await dropzoneToInventoryGapPx(page);
    expect(gapIdle).toBeGreaterThanOrEqual(0);
    expect(gapIdle).toBeLessThan(48);

    // Selection toolbar must not reintroduce the empty zone.
    await page
      .getByTestId("document-row-doc-099-failed-gap")
      .getByRole("checkbox")
      .click();
    await expect(page.getByTestId("spec099-selection-toolbar")).toBeVisible();
    await expect(page.getByTestId("spec051-feedback-zone")).toHaveCount(0);

    const gapSelected = await dropzoneToInventoryGapPx(page);
    expect(gapSelected).toBeGreaterThanOrEqual(0);
    expect(gapSelected).toBeLessThan(48);
  });

  test("live run positive control: feedback zone open + dropzone collapsed", async ({
    page,
  }) => {
    await page.setViewportSize({ width: 1280, height: 800 });
    const live = makeSpec086ListDoc({
      id: "doc-099-live-gap",
      file_name: "busy-live.pdf",
      status: "processing",
      current_stage: "extracting",
      stage_message: "Extracting entities",
      stage_progress: 0.4,
      source_type: "pdf",
      track_id: "track-099-live-gap",
      admission_staging: false,
    });
    await mockSpec038AdmissionRoutes(page);
    await seedSpec038TenantContext(page);
    await mockSpec086BusyPipeline(page);
    await mockSpec086DocumentList(page, [live]);
    await page.goto("/documents", GOTO_OPTS);

    await expect(page.getByTestId("spec051-feedback-zone")).toBeVisible({
      timeout: 20_000,
    });
    await expect(page.getByTestId("spec048-active-runs-panel")).toBeVisible();
    await expect(page.getByTestId("document-dropzone")).toHaveAttribute(
      "data-collapsed",
      "true",
    );
  });
});
