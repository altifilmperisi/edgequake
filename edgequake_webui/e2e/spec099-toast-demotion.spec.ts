/**
 * SPEC-099 F-099-03 / LAW-099-6 — Toast demoted when feedback zone owns upload.
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
import { expandIntakeWorking, freshIso } from "./helpers/workspace-runs";

test.describe("SPEC-099 toast demotion", () => {
  test("no persistent Uploading N file(s) toast when zone lists session", async ({
    page,
  }) => {
    test.setTimeout(60_000);
    const live = makeSpec086ListDoc({
      id: "doc-099-toast-live",
      file_name: "toast-live.pdf",
      status: "processing",
      current_stage: "extracting",
      stage_message: "Extracting",
      stage_progress: 0.2,
      track_id: "track-099-toast",
      admission_staging: false,
      created_at: freshIso(-60_000),
      updated_at: freshIso(-2_000),
    });
    await mockSpec038AdmissionRoutes(page);
    await seedSpec038TenantContext(page);
    await mockSpec086BusyPipeline(page);
    await mockSpec086DocumentList(page, [live]);

    // Slow admit so upload list stays visible beside the live run.
    await page.route("**/api/v1/documents/text**", async (route) => {
      await new Promise((r) => setTimeout(r, 2500));
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          id: "doc-099-toast",
          status: "pending",
          track_id: "insert-099-toast",
        }),
      });
    });

    await page.goto("/documents", GOTO_OPTS);
    await expandIntakeWorking(page);
    await expect(page.getByTestId("spec051-feedback-zone")).toBeVisible({
      timeout: 20_000,
    });

    const fileInput = page.locator('input[type="file"]').first();
    await fileInput.setInputFiles({
      name: "toast-demote.txt",
      mimeType: "text/plain",
      buffer: Buffer.from("hello toast demotion"),
    });

    // Feedback zone already owns the narrative — no competing loading toast.
    await expect(page.getByTestId("spec051-feedback-zone")).toBeVisible();
    const loadingToast = page.getByText(/Uploading \d+ file/i);
    await expect(loadingToast).toHaveCount(0);
  });
});
