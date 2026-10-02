/**
 * SPEC-057 P4 / REQ-057-05 — Cancel ingest shows Stopping… then Cancelled
 * (not Failed). Mocked API; CI-friendly without make dev-bg.
 */
import { expect, test, type Page } from "@playwright/test";
import { GOTO_OPTS } from "./helpers/app-ready";
import { expandIntakeWorking, freshIso } from "./helpers/workspace-runs";

const MOCK_TENANT_ID = "tenant-aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";
const MOCK_WORKSPACE_ID = "ws-aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";

const MOCK_TENANT = {
  id: MOCK_TENANT_ID,
  name: "TestTenant",
  slug: "test-tenant",
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:00Z",
};

const MOCK_WORKSPACE = {
  id: MOCK_WORKSPACE_ID,
  name: "Default Workspace",
  slug: "bootstrap-workspace",
  tenant_id: MOCK_TENANT_ID,
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:00Z",
};

const TRACK_ID = "track-p4-cancel-001";
const DOC_ID = "p4-cancel-doc-00000000-0000-0000-0000-000000000001";

type DocState = {
  id: string;
  title: string;
  file_name: string;
  status: string;
  current_stage: string;
  display_status: string;
  ui_phase: string;
  stage_message: string;
  stage_progress?: number;
  chunk_count: number;
  entity_count: number;
  source_type: string;
  track_id: string;
  created_at: string;
  updated_at: string;
};

function baseDoc(overrides: Partial<DocState> = {}): DocState {
  return {
    id: DOC_ID,
    title: "cancel-me.pdf",
    file_name: "cancel-me.pdf",
    status: "processing",
    current_stage: "extracting",
    display_status: "extracting",
    ui_phase: "running",
    stage_message: "Extracting entities — chunk 10/40",
    chunk_count: 40,
    entity_count: 0,
    source_type: "pdf",
    track_id: TRACK_ID,
    created_at: freshIso(-60_000),
    updated_at: freshIso(-5_000),
    ...overrides,
  };
}

async function mockShell(page: Page) {
  await page.route("**/live", async (route) => {
    await route.fulfill({ status: 200, contentType: "text/plain", body: "OK" });
  });
  await page.route("**/health", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        status: "healthy",
        version: "0.1.0-test",
        storage_mode: "postgresql",
      }),
    });
  });
  await page.route("**/ready", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ status: "ready" }),
    });
  });
  await page.route("**/api/v1/tenants*", async (route) => {
    const url = route.request().url();
    if (/\/api\/v1\/tenants\/[^/?]+/.test(url)) {
      await route.fallback();
      return;
    }
    if (route.request().method() === "GET") {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify([MOCK_TENANT]),
      });
    } else await route.fallback();
  });
  await page.route("**/api/v1/tenants/*/workspaces**", async (route) => {
    if (route.request().method() === "GET") {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify([MOCK_WORKSPACE]),
      });
    } else await route.fallback();
  });
  await page.route("**/api/v1/pipeline/status**", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        is_busy: true,
        job_name: "ingest",
        total_documents: 1,
        processed_documents: 0,
        current_batch: 0,
        total_batches: 0,
        history_messages: [],
        cancellation_requested: false,
        pending_tasks: 0,
        processing_tasks: 1,
      }),
    });
  });
  await page.route("**/api/v1/pipeline/activity", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        busy: true,
        working: [],
        queued: [],
        tasks: [],
        updated_at: new Date().toISOString(),
      }),
    });
  });
  await page.route("**/api/v1/tasks**", async (route) => {
    if (route.request().method() === "GET") {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          tasks: [],
          pagination: { total: 0, page: 1, page_size: 50, total_pages: 0 },
          statistics: {
            pending: 0,
            processing: 1,
            indexed: 0,
            failed: 0,
            cancelled: 0,
          },
        }),
      });
    } else await route.fallback();
  });
}

async function mockDocumentsList(page: Page, getDocs: () => DocState[]) {
  await page.route("**/api/v1/documents**", async (route) => {
    const url = route.request().url();
    if (route.request().method() === "GET" && !url.includes("/documents/pdf")) {
      const documents = getDocs();
      const cancelled = documents.filter((d) => d.status === "cancelled").length;
      const failed = documents.filter((d) => d.status === "failed").length;
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          documents,
          total: documents.length,
          page: 1,
          page_size: 20,
          total_pages: 1,
          has_more: false,
          status_counts: {
            pending: 0,
            processing: documents.filter((d) => d.status === "processing").length,
            completed: 0,
            partial_failure: 0,
            failed,
            cancelled,
          },
        }),
      });
    } else await route.fallback();
  });
}

test.describe("SPEC-057 P4 cancel status SSOT", () => {
  test("cancel in-flight → Stopping… → Cancelled (not Failed)", async ({
    page,
  }) => {
    let phase: "running" | "stopping" | "cancelled" = "running";

    await mockShell(page);
    await mockDocumentsList(page, () => {
      if (phase === "stopping") {
        return [
          baseDoc({
            display_status: "extracting",
            ui_phase: "stopping",
            stage_message: "Cancellation requested…",
          }),
        ];
      }
      if (phase === "cancelled") {
        return [
          baseDoc({
            status: "cancelled",
            current_stage: "cancelled",
            display_status: "cancelled",
            ui_phase: "terminal",
            stage_message: "Processing cancelled",
          }),
        ];
      }
      return [baseDoc()];
    });

    await page.route(`**/api/v1/tasks/${TRACK_ID}/cancel`, async (route) => {
      if (route.request().method() === "POST") {
        phase = "stopping";
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({ track_id: TRACK_ID, status: "cancelled" }),
        });
        // After brief Stopping…, terminal Cancelled (mapper SSOT).
        setTimeout(() => {
          phase = "cancelled";
        }, 400);
        return;
      }
      await route.fallback();
    });

    await page.goto("/documents", GOTO_OPTS);

    const row = page.getByTestId(`document-row-${DOC_ID}`);
    await expect(row).toBeVisible({ timeout: 15000 });
    // Compact density: Extracting may be title-only; cancel labels keep text.
    const badge = row.getByTestId("status-badge");
    await expect(badge).toHaveAttribute("title", /Extracting/i);

    // Open row actions → Cancel Extraction
    await row.getByRole("button", { name: /More actions/i }).click();
    const cancelItem = page.getByRole("menuitem", {
      name: /Cancel Extraction/i,
    });
    await expect(cancelItem).toBeVisible({ timeout: 5000 });
    await cancelItem.click();

    // Stopping… while cancel intent is active
    await expect(row.getByTestId("status-badge")).toContainText(/Stopping/i, {
      timeout: 10000,
    });

    // Force a refetch after terminal transition
    await page.waitForTimeout(600);
    // The mock's terminal transition is a backend event; make it explicit
    // before reload so the fresh document query cannot observe stale stopping.
    phase = "cancelled";
    await page.goto("/documents", GOTO_OPTS);
    const rowAfter = page.getByTestId(`document-row-${DOC_ID}`);
    await expect(rowAfter).toContainText(/Cancelled|Processing was cancelled/i, {
      timeout: 15000,
    });
    await expect(rowAfter).not.toContainText(/Failed/i);

    // Failed count chip must not treat cancelled as failed (if chip visible)
    const failedChip = page.getByTestId("status-count-failed");
    if (await failedChip.isVisible().catch(() => false)) {
      await expect(failedChip).not.toContainText(/^[1-9]/);
    }
  });

  test("ActiveRuns: cancelled ack is compact then dismissible (not Failed/Queued)", async ({
    page,
  }) => {
    const justNow = freshIso();
    await mockShell(page);
    await mockDocumentsList(page, () => [
      baseDoc({
        status: "cancelled",
        current_stage: "cancelled",
        display_status: "cancelled",
        ui_phase: "terminal",
        stage_message: "Processing cancelled",
        stage_progress: 0,
        updated_at: justNow,
      }),
    ]);

    await page.goto("/documents", GOTO_OPTS);

    await expandIntakeWorking(page);
    const panel = page.getByTestId("spec048-active-runs-panel");
    await expect(panel).toBeVisible({ timeout: 15000 });
    await expect(panel).toContainText(/Cancelled/i);
    await expect(panel).not.toContainText(/Queued run/i);

    const card = panel.getByTestId("spec048-active-run-card");
    await expect(card).toHaveAttribute("data-compact", "true");
    await expect(card.getByTestId("spec048-run-headline")).toHaveText(
      /Cancelled/i,
    );
    // Cancelled cards retain a frozen progress meter for continuity.
    await expect(card.getByTestId("spec086-cancel-progress-frozen")).toBeVisible();
    await expect(card.getByTestId("spec048-overall-progress")).toBeVisible();
    // Must not look like Failed pipeline
    await expect(card).not.toContainText(/Failed Processing/i);

    await card.getByTestId("spec086-run-dismiss").click();
    await expect(panel).toHaveCount(0, { timeout: 5000 });

    // Table still shows Cancelled (durable SSOT)
    const row = page.getByTestId(`document-row-${DOC_ID}`);
    await expect(row.getByTestId("status-badge")).toContainText(/Cancelled/i);
  });

  test("ActiveRuns: hours-old cancelled does not appear as Queued run", async ({
    page,
  }) => {
    await mockShell(page);
    await mockDocumentsList(page, () => [
      baseDoc({
        status: "cancelled",
        current_stage: "cancelled",
        display_status: "cancelled",
        ui_phase: "terminal",
        stage_message: "Processing cancelled",
        stage_progress: 0,
        updated_at: "2026-07-30T12:00:00Z",
      }),
    ]);

    await page.goto("/documents", GOTO_OPTS);

    await expect(page.getByTestId(`document-row-${DOC_ID}`)).toBeVisible({
      timeout: 15000,
    });
    await expect(
      page.getByTestId(`document-row-${DOC_ID}`).getByTestId("status-badge"),
    ).toContainText(/Cancelled/i);
    await expect(page.getByTestId("spec048-active-runs-panel")).toHaveCount(0);
  });
});
