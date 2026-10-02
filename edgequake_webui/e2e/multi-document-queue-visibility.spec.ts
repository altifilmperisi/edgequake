import { expect, test } from "@playwright/test";

import { GOTO_OPTS } from "./helpers/app-ready";
import {
  mockSpec038AdmissionRoutes,
  seedSpec038TenantContext,
  SPEC038_MOCK_TENANT_ID,
  SPEC038_MOCK_WORKSPACE_ID,
} from "./helpers/spec038-admission-mocks";
import { expandIntakeWorking } from "./helpers/workspace-runs";

test.describe("multi-document queue visibility", () => {
  test("accepts a later selection and keeps four independent queued runs", async ({
    page,
  }) => {
    await mockSpec038AdmissionRoutes(page);

    const pendingReleases: Array<() => void> = [];
    const admitted: Array<{
      id: string;
      title: string;
      track_id: string;
      status: string;
      tenant_id: string;
      workspace_id: string;
      created_at: string;
      file_name?: string;
      current_stage?: string;
      updated_at?: string;
    }> = [];
    let requestCount = 0;

    await page.route("**/api/v1/documents**", async (route) => {
      const request = route.request();
      if (request.method() === "GET") {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({
            documents: admitted,
            total: admitted.length,
            page: 1,
            page_size: 50,
            total_pages: 1,
            has_more: false,
            status_counts: {
              pending: admitted.length,
              processing: 0,
              completed: 0,
              partial_failure: 0,
              failed: 0,
              cancelled: 0,
            },
          }),
        });
        return;
      }
      if (request.method() !== "POST") {
        await route.fallback();
        return;
      }

      requestCount += 1;
      const body = request.postDataJSON() as { title: string };
      const ordinal = requestCount;
      await new Promise<void>((resolve) => pendingReleases.push(resolve));
      const trackId = `insert-queue-${ordinal}`;
      const now = new Date().toISOString();
      admitted.push({
        id: `00000000-0000-0000-0000-0000000000${ordinal
          .toString()
          .padStart(2, "0")}`,
        title: body.title,
        file_name: body.title,
        track_id: trackId,
        status: "pending",
        current_stage: "queued",
        tenant_id: SPEC038_MOCK_TENANT_ID,
        workspace_id: SPEC038_MOCK_WORKSPACE_ID,
        created_at: now,
        updated_at: now,
      });
      await route.fulfill({
        status: 202,
        contentType: "application/json",
        body: JSON.stringify({
          document_id: admitted.at(-1)?.id,
          task_id: trackId,
          track_id: trackId,
          status: "queued",
        }),
      });
    });

    await seedSpec038TenantContext(page);
    await page.goto("/documents", GOTO_OPTS);
    await page.getByRole("heading", { name: "Documents" }).waitFor();
    await expect(page.getByTestId("document-dropzone")).toBeVisible({
      timeout: 15_000,
    });

    const input = page
      .getByTestId("document-dropzone")
      .locator('input[type="file"]');
    const firstSelection = [1, 2, 3].map((ordinal) => ({
      name: `batch-a-${ordinal}.md`,
      mimeType: "text/markdown",
      buffer: Buffer.from(`# Batch A ${ordinal}\n\nUnique ${ordinal}`),
    }));
    await input.setInputFiles(firstSelection);
    await expect.poll(() => requestCount, { timeout: 20_000 }).toBe(3);

    await input.setInputFiles({
      name: "batch-b-4.md",
      mimeType: "text/markdown",
      buffer: Buffer.from("# Batch B 4\n\nA later selection"),
    });

    for (const file of [...firstSelection, { name: "batch-b-4.md" }]) {
      await expect(page.getByText(file.name).first()).toBeVisible();
    }
    // The fourth intent is visible but waits behind the shared three-request cap.
    expect(requestCount).toBe(3);

    pendingReleases.shift()?.();
    await expect.poll(() => requestCount).toBe(4);
    await expect.poll(() => pendingReleases.length).toBe(3);
    pendingReleases.splice(0).forEach((release) => release());

    await expect.poll(() => admitted.length).toBe(4);
    expect(new Set(admitted.map((document) => document.track_id)).size).toBe(4);
    await expandIntakeWorking(page);
    await expect(page.getByTestId("spec048-active-run-card")).toHaveCount(4, {
      timeout: 15_000,
    });
  });
});
