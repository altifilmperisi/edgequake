/**
 * Document status notice E2E — vision fallback must not render as Failed.
 *
 * Strategy: API route mocking (no live backend). A processing document with a
 * legacy informational error_message should show an active pipeline badge.
 */

import { expect, test } from "@playwright/test";
import { GOTO_OPTS } from "./helpers/app-ready";
import { seedTenantStoreOnPage } from "./helpers/spec013-bootstrap";

const MOCK_TENANT_ID = "tenant-aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";
const MOCK_WORKSPACE_ID = "ws-aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";
const MOCK_DOC_ID = "97079c92-d2f1-436c-a384-06537a684b4a";
const FAILED_DOC_ID = "failed-doc-00000000-0000-0000-0000-000000000001";

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

/** Fresh timestamps so SPEC-155 stall detection does not paint "Stalled". */
const NOW = new Date().toISOString();

/** Simulates SPF TOME II.pdf during chunking with vision fallback notice. */
const PROCESSING_DOC_WITH_LEGACY_NOTICE = {
  id: MOCK_DOC_ID,
  title: "SPF TOME II.pdf",
  file_name: "SPF TOME II.pdf",
  status: "processing",
  current_stage: "chunking",
  stage_message: "Splitting document into chunks...",
  error_message: "Vision unavailable. Falling back to EdgeParse.",
  warning_message: "Vision unavailable. Falling back to EdgeParse.",
  chunk_count: 0,
  entity_count: 0,
  source_type: "pdf",
  created_at: NOW,
  updated_at: NOW,
};

const TERMINAL_FAILED_DOC = {
  ...PROCESSING_DOC_WITH_LEGACY_NOTICE,
  id: FAILED_DOC_ID,
  title: "broken.pdf",
  file_name: "broken.pdf",
  status: "failed",
  current_stage: "failed",
  error_message: "Pipeline processing failed: entity extraction timeout",
  warning_message: undefined,
};

async function mockDocumentsApi(page: import("@playwright/test").Page) {
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

  await page.route("**/api/v1/setup/status**", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        needs_setup: false,
        has_login_users: true,
        tenant_count: 1,
        workspace_count: 1,
        auth_enabled: false,
        bootstrap_admin_configured: true,
      }),
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

  await page.route("**/api/v1/documents**", async (route) => {
    const url = route.request().url();
    const method = route.request().method();
    if (method === "GET" && !url.includes("/documents/pdf")) {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          documents: [PROCESSING_DOC_WITH_LEGACY_NOTICE, TERMINAL_FAILED_DOC],
          total: 2,
          page: 1,
          page_size: 20,
          total_pages: 1,
          has_more: false,
          status_counts: {
            pending: 0,
            processing: 1,
            completed: 0,
            partial_failure: 0,
            failed: 1,
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
        running_tasks: 1,
        is_busy: true,
        queued_tasks: 0,
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
            processing: 0,
            indexed: 0,
            failed: 0,
            cancelled: 0,
          },
        }),
      });
    } else {
      await route.fallback();
    }
  });
}

test.describe("Document status notices", () => {
  test.setTimeout(60_000);

  test.beforeEach(async ({ page }) => {
    await mockDocumentsApi(page);
    await seedTenantStoreOnPage(
      page,
      {
        tenantId: MOCK_TENANT_ID,
        tenantName: "TestTenant",
        workspaceId: MOCK_WORKSPACE_ID,
        workspaceName: "Default Workspace",
        workspaceSlug: "bootstrap-workspace",
      },
      { waitForReady: false },
    );
  });

  test("processing doc with vision fallback does not show Failed badge", async ({
    page,
  }) => {
    await page.goto("/documents", GOTO_OPTS);

    // Prefer stable row test id (SPEC-155 split header/body tables).
    const spfRow = page.getByTestId(`document-row-${MOCK_DOC_ID}`);
    await expect(spfRow).toBeVisible({ timeout: 15_000 });

    await expect(spfRow.getByText("Failed", { exact: true })).not.toBeVisible();
    // Compact inventory badge keeps the stage in `title` (icon-first density).
    await expect(spfRow.getByTestId("status-badge")).toHaveAttribute(
      "title",
      /Chunking|Processing|Preprocessing/i,
    );
  });

  test("terminal failed doc still shows Failed badge", async ({ page }) => {
    await page.goto("/documents", GOTO_OPTS);

    const failedRow = page.getByTestId(`document-row-${FAILED_DOC_ID}`);
    await expect(failedRow).toBeVisible({ timeout: 15_000 });
    await expect(
      failedRow.getByText("Failed", { exact: true }),
    ).toBeVisible();
  });
});
