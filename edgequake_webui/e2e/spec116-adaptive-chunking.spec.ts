/**
 * SPEC-116 — Workspace chunking card + Acc-fair wizard round-trip (mocked API).
 *
 * Run:
 *   cd edgequake_webui && pnpm exec playwright test e2e/spec116-adaptive-chunking.spec.ts --project=chromium
 */

import { expect, test, type Page, type Route } from "@playwright/test";
import { GOTO_OPTS } from "./helpers/app-ready";

const MOCK_TENANT_ID = "aaaaaaaa-0116-0116-0116-aaaaaaaaaaaa";
const MOCK_WORKSPACE_ID = "bbbbbbbb-0116-0116-0116-bbbbbbbbbbbb";

const MOCK_TENANT = {
  id: MOCK_TENANT_ID,
  name: "SPEC-116 Tenant",
  slug: "spec116-tenant",
  plan: "pro",
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:00Z",
};

let mockWorkspace = {
  id: MOCK_WORKSPACE_ID,
  tenant_id: MOCK_TENANT_ID,
  name: "SPEC-116 Workspace",
  slug: "spec116-ws",
  llm_model: "gemma4:latest",
  llm_provider: "ollama",
  llm_full_id: "ollama/gemma4:latest",
  embedding_model: "embeddinggemma:latest",
  embedding_provider: "ollama",
  embedding_dimension: 768,
  embedding_full_id: "ollama/embeddinggemma:latest",
  entity_types: ["PERSON", "ORGANIZATION", "LOCATION"],
  entity_types_strict: true,
  entity_type_colors: {} as Record<string, string>,
  extraction_language: null as string | null,
  chunking_mode: null as string | null,
  chunk_token_size: null as number | null,
  chunk_overlap_token_size: null as number | null,
  is_active: true,
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:00Z",
};

async function fulfillJson(route: Route, status: number, body: unknown) {
  await route.fulfill({
    status,
    contentType: "application/json",
    body: JSON.stringify(body),
  });
}

async function mockSpec116Backend(page: Page) {
  await page.route("**/health", (route) =>
    fulfillJson(route, 200, { status: "healthy" }),
  );
  await page.route("**/api/health", (route) =>
    fulfillJson(route, 200, { status: "healthy" }),
  );
  await page.route("**/live", (route) =>
    route.fulfill({ status: 200, body: "OK" }),
  );

  await page.route("**/api/v1/setup/status", async (route) => {
    await fulfillJson(route, 200, {
      needs_setup: false,
      has_login_users: true,
      tenant_count: 1,
      workspace_count: 1,
      auth_enabled: false,
      bootstrap_admin_configured: true,
    });
  });

  await page.route("**/api/v1/tenants/*/workspaces**", async (route) => {
    await fulfillJson(route, 200, [mockWorkspace]);
  });

  await page.route("**/api/v1/tenants*", async (route) => {
    await fulfillJson(route, 200, [MOCK_TENANT]);
  });

  await page.route(`**/api/v1/tenants/${MOCK_TENANT_ID}`, async (route) => {
    await fulfillJson(route, 200, MOCK_TENANT);
  });

  await page.route(
    `**/api/v1/tenants/${MOCK_TENANT_ID}/workspaces/by-slug/*`,
    async (route) => {
      await fulfillJson(route, 200, mockWorkspace);
    },
  );

  await page.route(`**/api/v1/workspaces/${MOCK_WORKSPACE_ID}*`, async (route) => {
    const method = route.request().method();
    if (method === "PUT" || method === "PATCH") {
      const body = route.request().postDataJSON() as {
        chunking_mode?: string | null;
        chunk_token_size?: number | null;
        chunk_overlap_token_size?: number | null;
      };
      if (
        body.chunking_mode === "inherit" ||
        body.chunking_mode === "none" ||
        body.chunking_mode === ""
      ) {
        mockWorkspace = {
          ...mockWorkspace,
          chunking_mode: null,
          chunk_token_size: null,
          chunk_overlap_token_size: null,
        };
      } else if (body.chunking_mode) {
        mockWorkspace = {
          ...mockWorkspace,
          chunking_mode: body.chunking_mode,
          chunk_token_size:
            body.chunking_mode === "fixed"
              ? (body.chunk_token_size ?? 1200)
              : null,
          chunk_overlap_token_size:
            body.chunking_mode === "fixed"
              ? (body.chunk_overlap_token_size ?? 100)
              : null,
        };
      }
      await fulfillJson(route, 200, mockWorkspace);
      return;
    }
    await fulfillJson(route, 200, mockWorkspace);
  });

  await page.route("**/api/v1/workspaces/*/stats*", (route) =>
    fulfillJson(route, 200, {
      workspace_id: MOCK_WORKSPACE_ID,
      document_count: 0,
      entity_count: 0,
      relationship_count: 0,
      chunk_count: 0,
      embedding_count: 0,
      storage_bytes: 0,
    }),
  );

  await page.route("**/api/v1/settings/**", (route) =>
    fulfillJson(route, 200, {
      effective: {
        llm_provider: "ollama",
        llm_model: "gemma4:latest",
        embedding_provider: "ollama",
        embedding_model: "embeddinggemma:latest",
        vision_provider: "ollama",
        vision_model: "gemma4:latest",
      },
    }),
  );
  await page.route("**/api/v1/settings/provider/status**", (route) =>
    fulfillJson(route, 200, {
      provider: {
        name: "ollama",
        type: "llm",
        status: "connected",
      },
    }),
  );
  await page.route("**/api/v1/models**", (route) =>
    fulfillJson(route, 200, {
      default_llm_provider: "ollama",
      default_llm_model: "gemma4:latest",
      default_embedding_provider: "ollama",
      default_embedding_model: "embeddinggemma:latest",
      providers: [],
    }),
  );
  await page.route("**/api/v1/models/health**", (route) =>
    fulfillJson(route, 200, []),
  );
  await page.route("**/api/v1/providers*", (route) =>
    fulfillJson(route, 200, []),
  );
  await page.route("**/api/v1/documents*", (route) =>
    fulfillJson(route, 200, { items: [], total: 0, offset: 0, limit: 10 }),
  );
  await page.route("**/ws/**", (route) =>
    route.fulfill({ status: 200, body: "" }),
  );
}

async function seedTenantContext(page: Page) {
  await page.goto("/", GOTO_OPTS);
  await page.evaluate(
    ({ tenantId, workspaceId }) => {
      localStorage.clear();
      sessionStorage.clear();
      const userId = crypto.randomUUID();
      localStorage.setItem("userId", userId);
      localStorage.setItem("tenantId", tenantId);
      localStorage.setItem("workspaceId", workspaceId);
      localStorage.setItem(
        "edgequake-tenant",
        JSON.stringify({
          state: {
            selectedTenantId: tenantId,
            selectedWorkspaceId: workspaceId,
          },
          version: 1,
        }),
      );
    },
    { tenantId: MOCK_TENANT_ID, workspaceId: MOCK_WORKSPACE_ID },
  );
  await page.reload(GOTO_OPTS);
}

test.describe("SPEC-116 workspace chunking", () => {
  test.setTimeout(90_000);

  test.beforeEach(async ({ page }) => {
    mockWorkspace = {
      ...mockWorkspace,
      chunking_mode: null,
      chunk_token_size: null,
      chunk_overlap_token_size: null,
    };
    await mockSpec116Backend(page);
    await seedTenantContext(page);
  });

  test("card inherit + Acc-fair chip round-trip", async ({ page }) => {
    await page.goto("/workspace", GOTO_OPTS);

    const card = page.getByTestId("workspace-chunking-card");
    await expect(card).toBeVisible({ timeout: 30_000 });
    await expect(page.getByTestId("chunking-future-only-hint")).toBeVisible();
    await expect(page.getByTestId("chunking-markdown-pack-hint")).toBeVisible();
    await expect(page.getByTestId("ws-chunking-value")).toContainText(/Inherit/i);

    await page.getByTestId("workspace-edit-config").click();
    await expect(page.getByTestId("reconfigure-workspace-wizard")).toBeVisible({
      timeout: 15_000,
    });
    // models → document-parsing → chunking → extract-budget → extraction → review
    await page.getByTestId("wizard-next").click();
    await page.getByTestId("wizard-next").click();
    await expect(page.getByTestId("wizard-step-chunking")).toBeVisible();
    await expect(page.getByTestId("wizard-step-label")).toHaveText(/Chunking/i);
    await expect(page.getByTestId("wizard-chunking-extract-hint")).toBeVisible();

    await page.getByTestId("chunking-acc-fair-chip").click();
    await expect(page.getByTestId("chunking-size-input")).toHaveValue("1200");
    await expect(page.getByTestId("chunking-overlap-input")).toHaveValue("100");

    await page.getByTestId("wizard-next").click();
    const wizard = page.getByTestId("reconfigure-workspace-wizard");
    await expect(wizard.getByTestId("wizard-step-extract-budget")).toBeVisible();
    await expect(wizard.getByTestId("workspace-chunking-card")).toHaveCount(0);
    await expect(wizard.getByTestId("wizard-step-chunking")).toHaveCount(0);

    await page.getByTestId("wizard-next").click();
    await expect(wizard.getByTestId("wizard-step-extraction")).toBeVisible();

    await page.getByTestId("wizard-next").click();
    await expect(page.getByTestId("wizard-review-chunking")).toBeVisible();
    await expect(page.getByTestId("wizard-reconfigure-impact")).toBeVisible();
    await page.getByTestId("wizard-finish").click();
    await expect(page.getByTestId("reconfigure-workspace-wizard")).toBeHidden({
      timeout: 15_000,
    });
    await expect(page.getByTestId("ws-chunking-value")).toContainText(
      /Fixed.*1200\/100/i,
    );
  });
});
