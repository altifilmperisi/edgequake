/**
 * Ingestion alert banner E2E — stuck vs queued vs working (mocked API).
 *
 * Strategy: API route mocking (no live backend). Validates SPEC-045
 * honest ingestion UX: document-task desync surfaces as "needs attention".
 */

import { expect, test } from "@playwright/test";
import { GOTO_OPTS } from "./helpers/app-ready";
import { seedTenantStoreOnPage } from "./helpers/spec013-bootstrap";
import { expandIntakeWorking, freshIso } from "./helpers/workspace-runs";

const MOCK_TENANT_ID = "tenant-aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";
const MOCK_WORKSPACE_ID = "ws-aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";

const MOCK_CTX = {
  tenantId: MOCK_TENANT_ID,
  tenantName: "TestTenant",
  workspaceId: MOCK_WORKSPACE_ID,
  workspaceName: "Default Workspace",
  workspaceSlug: "bootstrap-workspace",
};

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

const STUCK_PENDING_DOC = {
  id: "stuck-doc-00000000-0000-0000-0000-000000000001",
  title: "deep_2604.26962v2.pdf",
  file_name: "deep_2604.26962v2.pdf",
  status: "pending",
  current_stage: "pending",
  stage_message:
    "Auto-recovered after server restart (was in 'pending' stage). Resuming from checkpoint...",
  chunk_count: 0,
  entity_count: 0,
  source_type: "pdf",
  created_at: "2026-06-06T10:00:00Z",
  updated_at: "2026-06-06T10:05:00Z",
};

const EXTRACTING_DOC = {
  id: "active-doc-00000000-0000-0000-0000-000000000002",
  title: "active.md",
  file_name: "active.md",
  status: "processing",
  current_stage: "extracting",
  stage_message: "Extracting entities...",
  chunk_count: 5,
  entity_count: 0,
  source_type: "text",
  track_id: "track-alert-extract",
  created_at: freshIso(-60_000),
  updated_at: freshIso(-5_000),
};

const GRAPH_MERGE_DOC = {
  id: "merge-doc-00000000-0000-0000-0000-000000000004",
  title: "deep_2604.26962v2.pdf",
  file_name: "deep_2604.26962v2.pdf",
  status: "indexing",
  current_stage: "storing",
  stage_message:
    "Storing in knowledge graph — RelationshipGraph (2654/2654 entities (100%), 128/1999 relationships (6%))",
  stage_progress: 0.66,
  chunk_count: 120,
  entity_count: 2654,
  source_type: "pdf",
  track_id: "track-alert-merge",
  created_at: freshIso(-60_000),
  updated_at: freshIso(-5_000),
};

const QUEUED_PENDING_DOC = {
  ...STUCK_PENDING_DOC,
  id: "queued-doc-00000000-0000-0000-0000-000000000003",
  title: "queued.md",
  file_name: "queued.md",
  stage_message: "Waiting for a processing slot",
  created_at: freshIso(-30_000),
  updated_at: freshIso(-5_000),
};

async function mockBaseApi(page: import("@playwright/test").Page) {
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

  // Trailing `*` so `?limit=` list calls match (Playwright globs include query).
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
        body: JSON.stringify({
          items: [MOCK_TENANT],
          total: 1,
          offset: 0,
          limit: 50,
        }),
      });
    } else {
      await route.fallback();
    }
  });

  await page.route("**/api/v1/tenants/*/workspaces**", async (route) => {
    if (route.request().method() === "GET") {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          items: [MOCK_WORKSPACE],
          total: 1,
          offset: 0,
          limit: 50,
        }),
      });
    } else {
      await route.fallback();
    }
  });
}

async function mockDocuments(
  page: import("@playwright/test").Page,
  documents: object[],
  taskStats: {
    pending: number;
    processing: number;
    indexed?: number;
  },
) {
  await mockBaseApi(page);

  await page.route("**/api/v1/documents**", async (route) => {
    const url = route.request().url();
    const method = route.request().method();
    if (method === "GET" && !url.includes("/documents/pdf")) {
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
            pending: documents.filter(
              (d: { status?: string }) => d.status === "pending",
            ).length,
            processing: documents.filter(
              (d: { status?: string }) => d.status === "processing",
            ).length,
            completed: 0,
            partial_failure: 0,
            failed: 0,
            cancelled: 0,
          },
        }),
      });
    } else {
      await route.fallback();
    }
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
            pending: taskStats.pending,
            processing: taskStats.processing,
            indexed: taskStats.indexed ?? 18,
            failed: 0,
            cancelled: 0,
          },
        }),
      });
    } else {
      await route.fallback();
    }
  });

  await page.route("**/api/v1/pipeline/status**", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        is_busy: taskStats.processing > 0,
        pending_tasks: taskStats.pending,
        processing_tasks: taskStats.processing,
        completed_tasks: taskStats.indexed ?? 18,
      }),
    });
  });
}

test.describe("Ingestion alert banner", () => {
  test.setTimeout(60_000);

  test.beforeEach(async ({ page }) => {
    await seedTenantStoreOnPage(page, MOCK_CTX, { waitForReady: false });
  });

  test("shows stuck state when pending doc has no queue coverage", async ({
    page,
  }) => {
    await mockDocuments(page, [STUCK_PENDING_DOC], {
      pending: 0,
      processing: 0,
      indexed: 18,
    });

    await page.goto("/documents", GOTO_OPTS);

    const banner = page.getByTestId("ingestion-status-banner");
    await expect(banner).toBeVisible({ timeout: 15_000 });
    await expect(page.getByTestId("ingestion-alert-stuck")).toBeVisible();
    await expect(banner.getByText(/need attention/i)).toBeVisible();
    await expect(banner.getByText(/18 done/i)).not.toBeVisible();
    await expect(page.getByTestId("ingestion-banner-reprocess")).toBeVisible();
    await expect(banner.getByText(/no worker is processing/i).first()).toBeVisible();
  });

  test("shows queued state when pending tasks exist", async ({ page }) => {
    await mockDocuments(page, [QUEUED_PENDING_DOC], {
      pending: 1,
      processing: 0,
    });

    await page.goto("/documents", GOTO_OPTS);

    // Non-stuck chrome is demoted when the feedback zone owns the narrative.
    await expandIntakeWorking(page);
    await expect(page.getByTestId("spec048-active-runs-panel")).toBeVisible({
      timeout: 15_000,
    });
    await expect(page.getByTestId("pipeline-header-button")).toContainText(
      /Queued|Waiting/i,
    );
    await expect(page.getByTestId("ingestion-status-banner")).toHaveCount(0);
    await expect(page.getByTestId("ingestion-banner-reprocess")).toHaveCount(0);
  });

  test("shows working state for actively extracting document", async ({
    page,
  }) => {
    await mockDocuments(page, [EXTRACTING_DOC], {
      pending: 0,
      processing: 1,
    });

    await page.goto("/documents", GOTO_OPTS);

    await expandIntakeWorking(page);
    await expect(page.getByTestId("spec048-active-runs-panel")).toBeVisible({
      timeout: 15_000,
    });
    // SPEC-091/155: phase caption (Extract · …) or legacy Extracting Entities.
    await expect(page.getByTestId("spec048-run-headline").first()).toContainText(
      /Extract(ing)?|Extract ·/i,
    );
    await expect(page.getByTestId("pipeline-header-button")).toContainText(
      /Working/i,
    );
    await expect(page.getByTestId("ingestion-status-banner")).toHaveCount(0);
  });

  test("shows live graph merge progress with bar and relationship counters", async ({
    page,
  }) => {
    await mockDocuments(page, [GRAPH_MERGE_DOC], {
      pending: 0,
      processing: 1,
    });

    await page.goto("/documents", GOTO_OPTS);

    await expandIntakeWorking(page);
    const zone = page.getByTestId("spec048-active-runs-panel");
    await expect(zone).toBeVisible({ timeout: 15_000 });
    await expect(page.getByTestId("spec048-run-headline").first()).toContainText(
      /Materialize|Storing|relationships/i,
    );
    const stageProgress = page
      .getByTestId("spec048-stage-progress")
      .or(page.getByTestId("spec048-overall-progress"));
    await expect(stageProgress.first()).toBeVisible();
    await expect(zone).toContainText(/relationships|66%|Materialize|Storing/i);
    await expect(page.getByTestId("pipeline-header-button")).toContainText(
      /Working/i,
    );
    await expect(page.getByTestId("ingestion-status-banner")).toHaveCount(0);
  });

  test("opens stuck pipeline dialog from banner click", async ({ page }) => {
    await mockDocuments(page, [STUCK_PENDING_DOC], {
      pending: 0,
      processing: 0,
    });

    await page.goto("/documents", GOTO_OPTS);

    await page.getByTestId("ingestion-status-banner").click();
    await expect(page.getByTestId("pipeline-dialog-stuck")).toBeVisible({
      timeout: 10_000,
    });
    await expect(page.getByText(/Stuck documents/i)).toBeVisible();
  });

  test("reprocess stuck document from banner button", async ({ page }) => {
    await mockDocuments(page, [STUCK_PENDING_DOC], {
      pending: 0,
      processing: 0,
    });

    let reprocessCalled = false;
    await page.route("**/api/v1/documents/reprocess**", async (route) => {
      if (route.request().method() === "POST") {
        reprocessCalled = true;
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({
            status: "accepted",
            document_id: STUCK_PENDING_DOC.id,
            track_id: "recover_test_track",
          }),
        });
      } else {
        await route.fallback();
      }
    });

    await page.goto("/documents", GOTO_OPTS);

    await page.getByTestId("ingestion-banner-reprocess").click();
    await expect.poll(() => reprocessCalled).toBe(true);
  });
});
