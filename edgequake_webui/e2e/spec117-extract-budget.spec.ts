/**
 * SPEC-117 — Workspace extract budget card + LightRAG preset (mocked API).
 *
 * Run:
 *   cd edgequake_webui && pnpm exec playwright test e2e/spec117-extract-budget.spec.ts --project=chromium
 */

import { expect, test, type Page, type Route } from "@playwright/test";
import { GOTO_OPTS } from "./helpers/app-ready";

const MOCK_TENANT_ID = "aaaaaaaa-0117-0117-0117-aaaaaaaaaaaa";
const MOCK_WORKSPACE_ID = "bbbbbbbb-0117-0117-0117-bbbbbbbbbbbb";

const MOCK_TENANT = {
  id: MOCK_TENANT_ID,
  name: "SPEC-117 Tenant",
  slug: "spec117-tenant",
  plan: "pro",
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:00Z",
};

let mockWorkspace = {
  id: MOCK_WORKSPACE_ID,
  tenant_id: MOCK_TENANT_ID,
  name: "SPEC-117 Workspace",
  slug: "spec117-ws",
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
  extract_budget_mode: null as string | null,
  extract_max_entities: null as number | null,
  extract_max_records: null as number | null,
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

async function mockSpec117Backend(page: Page) {
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
        extract_budget_mode?: string | null;
        extract_max_entities?: number | null;
        extract_max_records?: number | null;
      };
      if (
        body.extract_budget_mode === "inherit" ||
        body.extract_budget_mode === "none" ||
        body.extract_budget_mode === ""
      ) {
        mockWorkspace = {
          ...mockWorkspace,
          extract_budget_mode: null,
          extract_max_entities: null,
          extract_max_records: null,
        };
      } else if (
        body.extract_budget_mode === "custom" ||
        typeof body.extract_max_entities === "number"
      ) {
        mockWorkspace = {
          ...mockWorkspace,
          extract_budget_mode: "custom",
          extract_max_entities: body.extract_max_entities ?? 40,
          extract_max_records: body.extract_max_records ?? 100,
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

test.describe("SPEC-117 extract budget", () => {
  test.setTimeout(90_000);

  test.beforeEach(async ({ page }) => {
    mockWorkspace = {
      ...mockWorkspace,
      extract_budget_mode: null,
      extract_max_entities: null,
      extract_max_records: null,
    };
    await mockSpec117Backend(page);
    await seedTenantContext(page);
  });

  test("card inherit + LightRAG preset round-trip", async ({ page }) => {
    await page.goto("/workspace", GOTO_OPTS);

    const card = page.getByTestId("workspace-extract-budget-card");
    await expect(card).toBeVisible({ timeout: 30_000 });
    await expect(
      page.getByTestId("extract-budget-future-only-hint"),
    ).toBeVisible();
    await expect(page.getByTestId("ws-extract-budget-value")).toContainText(
      /Inherit/i,
    );

    await page.getByTestId("workspace-edit-config").click();
    await expect(page.getByTestId("reconfigure-workspace-wizard")).toBeVisible({
      timeout: 15_000,
    });
    // models → document-parsing → chunking → extract-budget → extraction → review
    await page.getByTestId("wizard-next").click();
    await page.getByTestId("wizard-next").click();
    await expect(page.getByTestId("wizard-step-chunking")).toBeVisible();
    await page.getByTestId("wizard-next").click();

    const wizard = page.getByTestId("reconfigure-workspace-wizard");
    await expect(wizard.getByTestId("wizard-step-extract-budget")).toBeVisible();
    await expect(page.getByTestId("wizard-step-label")).toHaveText(
      /Extract budget/i,
    );
    await expect(
      wizard.getByTestId("wizard-extract-budget-chunking-hint"),
    ).toBeVisible();

    await page.getByTestId("extract-budget-preset-lightrag").click();
    await expect(page.getByTestId("extract-budget-entities")).toHaveValue("40");
    await expect(page.getByTestId("extract-budget-records")).toHaveValue("100");

    await page.getByTestId("wizard-next").click();
    await expect(wizard.getByTestId("wizard-step-extraction")).toBeVisible();
    await expect(wizard.getByTestId("workspace-extract-budget-card")).toHaveCount(
      0,
    );

    await page.getByTestId("wizard-next").click();
    await expect(page.getByTestId("wizard-review-extract-budget")).toBeVisible();
    await expect(page.getByTestId("wizard-reconfigure-impact")).toBeVisible();
    await page.getByTestId("wizard-finish").click();
    await expect(page.getByTestId("reconfigure-workspace-wizard")).toBeHidden({
      timeout: 15_000,
    });
    await expect(page.getByTestId("ws-extract-budget-value")).toContainText(
      /40\/100|Custom/i,
    );
  });
});
