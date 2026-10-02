/**
 * Documents table column-sort E2E (mocked API).
 *
 * First principles: clicking a sortable header reorders rows using the same
 * client-side sort SSOT as the toolbar; aria-sort reflects the active column.
 */

import { expect, test } from "@playwright/test";
import { GOTO_OPTS } from "./helpers/app-ready";
import { seedTenantStoreOnPage } from "./helpers/spec013-bootstrap";

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

const DOC_LOW = {
  id: "sort-doc-low-00000000-0000-0000-0000-000000000001",
  title: "aaa-low-entities.pdf",
  file_name: "aaa-low-entities.pdf",
  status: "completed",
  chunk_count: 10,
  entity_count: 10,
  cost_usd: 0.9,
  source_type: "pdf",
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:00Z",
};

const DOC_MID = {
  id: "sort-doc-mid-00000000-0000-0000-0000-000000000002",
  title: "mmm-mid-entities.pdf",
  file_name: "mmm-mid-entities.pdf",
  status: "completed",
  chunk_count: 50,
  entity_count: 50,
  cost_usd: 0.1,
  source_type: "pdf",
  created_at: "2026-01-02T00:00:00Z",
  updated_at: "2026-01-02T00:00:00Z",
};

const DOC_HIGH = {
  id: "sort-doc-high-00000000-0000-0000-0000-000000000003",
  title: "zzz-high-entities.pdf",
  file_name: "zzz-high-entities.pdf",
  status: "completed",
  chunk_count: 100,
  entity_count: 100,
  cost_usd: 0.5,
  source_type: "pdf",
  created_at: "2026-01-03T00:00:00Z",
  updated_at: "2026-01-03T00:00:00Z",
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

  await page.route("**/api/v1/pipeline/status**", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        status: "idle",
        pending: 0,
        processing: 0,
        indexed: 3,
      }),
    });
  });

  await page.route("**/api/v1/documents**", async (route) => {
    const url = route.request().url();
    const method = route.request().method();
    if (method === "GET" && !url.includes("/documents/pdf")) {
      const documents = [DOC_LOW, DOC_MID, DOC_HIGH];
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
            processing: 0,
            completed: 3,
            failed: 0,
          },
        }),
      });
    } else {
      await route.fallback();
    }
  });
}

async function rowTitlesInOrder(page: import("@playwright/test").Page) {
  return page.locator("[data-document-title]").evaluateAll((els) =>
    els.map((el) => el.getAttribute("data-document-title") ?? ""),
  );
}

test.describe("Documents table column sort", () => {
  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
      localStorage.setItem(
        "edgequake:documents:prefs",
        JSON.stringify({
          pageSize: 20,
          statusFilter: "all",
          sortField: "created_at",
          sortDirection: "desc",
          showCostColumn: true,
        }),
      );
    });
    await mockBaseApi(page);
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

  test("sorts by Entities header and toggles direction", async ({ page }) => {
    await page.goto("/documents", GOTO_OPTS);
    // Maximize library so @container progressive column hide does not drop Entities.
    await page.setViewportSize({ width: 1400, height: 900 });
    await expect(page.getByTestId("sort-header-entity_count")).toBeVisible({
      timeout: 30_000,
    });

    // Default prefs: created_at desc → newest first
    await expect.poll(async () => rowTitlesInOrder(page)).toEqual([
      "zzz-high-entities.pdf",
      "mmm-mid-entities.pdf",
      "aaa-low-entities.pdf",
    ]);

    const entitiesHeader = page.getByTestId("sort-header-entity_count");
    await entitiesHeader.getByRole("button").click();

    // First click on metric → desc (highest entities first)
    await expect(entitiesHeader).toHaveAttribute("aria-sort", "descending");
    await expect.poll(async () => rowTitlesInOrder(page)).toEqual([
      "zzz-high-entities.pdf",
      "mmm-mid-entities.pdf",
      "aaa-low-entities.pdf",
    ]);

    await entitiesHeader.getByRole("button").click();
    await expect(entitiesHeader).toHaveAttribute("aria-sort", "ascending");
    await expect.poll(async () => rowTitlesInOrder(page)).toEqual([
      "aaa-low-entities.pdf",
      "mmm-mid-entities.pdf",
      "zzz-high-entities.pdf",
    ]);
  });

  test("sorts by Title and Cost columns", async ({ page }) => {
    await page.goto("/documents", GOTO_OPTS);
    await page.setViewportSize({ width: 1400, height: 900 });
    await expect(page.getByTestId("sort-header-title")).toBeVisible({
      timeout: 30_000,
    });

    await page.getByTestId("sort-header-title").getByRole("button").click();
    await expect(page.getByTestId("sort-header-title")).toHaveAttribute(
      "aria-sort",
      "ascending",
    );
    await expect.poll(async () => rowTitlesInOrder(page)).toEqual([
      "aaa-low-entities.pdf",
      "mmm-mid-entities.pdf",
      "zzz-high-entities.pdf",
    ]);

    await page.getByTestId("sort-header-cost_usd").getByRole("button").click();
    await expect(page.getByTestId("sort-header-cost_usd")).toHaveAttribute(
      "aria-sort",
      "descending",
    );
    await expect.poll(async () => rowTitlesInOrder(page)).toEqual([
      "aaa-low-entities.pdf",
      "zzz-high-entities.pdf",
      "mmm-mid-entities.pdf",
    ]);
  });
});
