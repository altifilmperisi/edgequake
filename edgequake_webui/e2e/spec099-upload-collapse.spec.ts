/**
 * SPEC-099 F-099-04 — Upload slot collapses when feedback zone has live work.
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

test.describe("SPEC-099 upload collapse", () => {
  test("data-collapsed=true when Active runs open; dropzone still activatable", async ({
    page,
  }) => {
    const doc = makeSpec086ListDoc({
      id: "doc-099-collapse",
      file_name: "busy-collapse.pdf",
      status: "processing",
      current_stage: "extracting",
      stage_message: "Extracting entities",
      stage_progress: 0.4,
      source_type: "pdf",
      track_id: "track-099-collapse",
      admission_staging: false,
    });
    await mockSpec038AdmissionRoutes(page);
    await seedSpec038TenantContext(page);
    await mockSpec086BusyPipeline(page);
    await mockSpec086DocumentList(page, [doc]);
    await page.goto("/documents", GOTO_OPTS);

    await expect(page.getByTestId("spec048-active-runs-panel")).toBeVisible({
      timeout: 20_000,
    });
    const dropzone = page.getByTestId("document-dropzone");
    await expect(dropzone).toHaveAttribute("data-collapsed", "true");
    await expect(dropzone).toBeVisible();
    // Keyboard / click target retained (role=group hosts nested selects)
    await expect(dropzone).toHaveAttribute("role", "group");
    await expect(dropzone.locator('input[type="file"]')).toHaveCount(1);
  });
});
