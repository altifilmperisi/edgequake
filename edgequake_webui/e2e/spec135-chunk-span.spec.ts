/**
 * SPEC-135 E2E-135-UI — span badge `p.1–2` and workspace pdf-pack hint.
 *
 * Run:
 *   cd edgequake_webui && pnpm exec playwright test e2e/spec135-chunk-span.spec.ts --project=chromium
 */

import { expect, test, type Page, type Route } from "@playwright/test";
import * as path from "node:path";
import { GOTO_OPTS } from "./helpers/app-ready";
import { buildBlankPdf } from "./helpers/blank-pdf";
import {
  mockSpec038AdmissionRoutes,
  seedSpec038TenantContext,
} from "./helpers/spec038-admission-mocks";

const MOCK_TENANT_ID = "aaaaaaaa-0135-0135-0135-aaaaaaaaaaaa";
const MOCK_WORKSPACE_ID = "bbbbbbbb-0135-0135-0135-bbbbbbbbbbbb";
const DOC_ID = "cccccccc-0135-0135-0135-cccccccccccc";
const CHUNK_ID = "span-chunk-0";

const MOCK_TENANT = {
  id: MOCK_TENANT_ID,
  name: "SPEC-135 Tenant",
  slug: "spec135-tenant",
  plan: "pro",
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:00Z",
};

const mockWorkspace = {
  id: MOCK_WORKSPACE_ID,
  tenant_id: MOCK_TENANT_ID,
  name: "SPEC-135 Workspace",
  slug: "spec135-ws",
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

async function mockWorkspaceBackend(page: Page) {
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
    await fulfillJson(route, 200, mockWorkspace);
  });
}

async function seedWorkspaceContext(page: Page) {
  await page.goto("/", GOTO_OPTS);
  await page.evaluate(
    ({ tenantId, workspaceId }) => {
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

test.describe("SPEC-135 chunk span UI", () => {
  test.setTimeout(90_000);

  test("workspace card shows pdf-pack and future-only hints", async ({
    page,
  }) => {
    await mockWorkspaceBackend(page);
    await seedWorkspaceContext(page);
    await page.goto("/workspace", GOTO_OPTS);
    await expect(page.getByTestId("workspace-chunking-card")).toBeVisible({
      timeout: 30_000,
    });
    await expect(page.getByTestId("chunking-pdf-pack-hint")).toBeVisible();
    await expect(page.getByTestId("chunking-future-only-hint")).toBeVisible();
  });

  test("hierarchy badge shows p.1–2 and deeplinks to start page", async ({
    page,
  }) => {
    await page.setViewportSize({ width: 1400, height: 900 });
    await mockSpec038AdmissionRoutes(page);
    await seedSpec038TenantContext(page);

    await page.route(`**/api/v1/documents/${DOC_ID}**`, async (route) => {
      if (route.request().method() !== "GET") {
        await route.fallback();
        return;
      }
      const url = route.request().url();
      if (url.includes("/lineage")) {
        await fulfillJson(route, 200, {
          document_id: DOC_ID,
          metadata: { title: "SPEC-135 Span Doc" },
          lineage: {
            document_name: "span.md",
            chunks: [
              {
                chunk_id: CHUNK_ID,
                chunk_index: 0,
                start_line: 1,
                end_line: 14,
                page_start: 1,
                page_end: 2,
                entity_ids: [],
              },
            ],
            entities: {},
          },
        });
        return;
      }
      await fulfillJson(route, 200, {
        id: DOC_ID,
        title: "SPEC-135 Span Doc",
        file_name: "span.md",
        status: "completed",
        source_type: "markdown",
        mime_type: "text/markdown",
        content: "span body across two pages",
        chunk_count: 1,
        entity_count: 0,
        relationship_count: 0,
        created_at: "2026-01-01T00:00:00Z",
        updated_at: "2026-01-01T00:00:00Z",
        track_id: null,
      });
    });

    await page.goto(`/documents/${DOC_ID}?chunk=${CHUNK_ID}&page=1`, GOTO_OPTS);

    const hierarchyToggle = page.getByRole("button", { name: /Data Hierarchy/i });
    await expect(hierarchyToggle).toBeVisible({ timeout: 30_000 });
    await hierarchyToggle.click();

    const badge = page.getByTestId("chunk-page-badge");
    await expect(badge).toBeVisible({ timeout: 30_000 });
    await expect(badge).toHaveText("p.1–2");
    await expect(badge).toHaveAttribute("href", /[?&]page=1(?:&|$)/);
    await expect(badge).not.toHaveAttribute("href", /[?&]page=2(?:&|$)/);
  });

  test("PDF citation deeplink opens page 4, highlights heading, and shows chunk row", async ({
    page,
  }) => {
    await page.setViewportSize({ width: 1400, height: 900 });
    await mockSpec038AdmissionRoutes(page);
    await seedSpec038TenantContext(page);

    const pdfBytes = buildBlankPdf(4);
    const pdfB64 = pdfBytes.toString("base64");
    const heading =
      "## 2.1 Challenges in the Prefill Stage: Transfer and Recomputation Costs";
    const markdown = [
      "# FreeToken",
      "<!-- edgequake-page:1 -->",
      "Intro paragraph on page one.",
      "<!-- edgequake-page:4 -->",
      heading,
      "",
      "Prefill determines TTFT of every agent turn.",
    ].join("\n");

    await page.addInitScript((b64: string) => {
      const origFetch = window.fetch.bind(window);
      window.fetch = async (input: RequestInfo | URL, init?: RequestInit) => {
        const url =
          typeof input === "string"
            ? input
            : input instanceof URL
              ? input.href
              : input.url;
        if (url.includes("/documents/pdf/") && url.includes("/download")) {
          const bin = Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
          return new Response(bin, {
            status: 200,
            headers: {
              "Content-Type": "application/pdf",
              "Content-Length": String(bin.length),
            },
          });
        }
        return origFetch(input, init);
      };
    }, pdfB64);

    await page.context().route(/\/api\/v1\/documents\/pdf\/[^/]+\/download/, async (route) => {
      if (route.request().method() === "OPTIONS") {
        await route.fulfill({ status: 204 });
        return;
      }
      await route.fulfill({
        status: 200,
        contentType: "application/pdf",
        body: pdfBytes,
      });
    });

    await page.route("**/pdf.worker.min.mjs*", async (route) => {
      const workerPath = path.resolve(
        __dirname,
        "../node_modules/pdfjs-dist/build/pdf.worker.min.mjs",
      );
      await route.fulfill({
        status: 200,
        path: workerPath,
        contentType: "text/javascript",
      });
    });

    await page.route(`**/api/v1/documents/${DOC_ID}**`, async (route) => {
      if (route.request().method() !== "GET") {
        await route.fallback();
        return;
      }
      const url = route.request().url();
      if (url.includes("/lineage")) {
        await fulfillJson(route, 200, {
          document_id: DOC_ID,
          metadata: { title: "free_token.pdf" },
          lineage: {
            document_name: "free_token.pdf",
            chunks: [
              {
                chunk_id: "page1-chunk",
                chunk_index: 0,
                start_line: 1,
                end_line: 3,
                page_start: 1,
                page_end: 1,
                entity_ids: [],
              },
              {
                chunk_id: CHUNK_ID,
                chunk_index: 5,
                start_line: 5,
                end_line: 7,
                page_start: 4,
                page_end: 4,
                entity_ids: [],
              },
            ],
            entities: {},
          },
        });
        return;
      }
      if (url.includes("/pages")) {
        await fulfillJson(route, 200, {
          document_id: DOC_ID,
          pages: [1, 2, 3, 4].map((n) => ({
            page_number: n,
            width_pt: 612,
            height_pt: 792,
            rotation: 0,
            layout_status: "skipped",
            region_count: 0,
          })),
        });
        return;
      }
      await fulfillJson(route, 200, {
        id: DOC_ID,
        title: "free_token.pdf",
        file_name: "free_token.pdf",
        status: "completed",
        source_type: "pdf",
        mime_type: "application/pdf",
        pdf_id: DOC_ID,
        content: markdown,
        chunk_count: 2,
        entity_count: 0,
        relationship_count: 0,
        created_at: "2026-01-01T00:00:00Z",
        updated_at: "2026-01-01T00:00:00Z",
        track_id: null,
      });
    });

    await page.route(`**/api/v1/documents/pdf/${DOC_ID}/content`, async (route) => {
      await fulfillJson(route, 200, {
        pdf_id: DOC_ID,
        document_id: DOC_ID,
        filename: "free_token.pdf",
        file_size_bytes: pdfBytes.length,
        content_type: "application/pdf",
        markdown_content: markdown,
        is_processed: true,
      });
    });

    const highlight = `${heading}\n\nPrefill determines TTFT of`;
    const qs = new URLSearchParams({
      chunk: CHUNK_ID,
      page: "4",
      highlight,
    });
    await page.goto(`/documents/${DOC_ID}?${qs.toString()}`, GOTO_OPTS);

    await expect(page.getByTestId("pdf-viewer")).toBeVisible({ timeout: 30_000 });
    await expect(page.getByTestId("pdf-page-indicator")).toHaveAttribute(
      "data-page",
      "4",
      { timeout: 30_000 },
    );

    await expect(
      page.getByTestId("side-by-side-viewer").getByText(/Challenges in the Prefill/i),
    ).toBeVisible({ timeout: 30_000 });
    await expect(
      page.getByTestId("side-by-side-viewer").locator('[data-highlighted="true"]').first(),
    ).toBeVisible({ timeout: 30_000 });

    await expect(page.getByTestId("data-hierarchy-section")).toBeVisible();
    const page4Group = page.locator('[data-testid="page-group"][data-page="4"]');
    await expect(page4Group).toBeVisible();
    await expect(page4Group).toHaveAttribute("aria-expanded", "true");

    const selectedRow = page.locator(
      `[data-testid="hierarchy-chunk-row"][data-chunk-id="${CHUNK_ID}"]`,
    );
    await expect(selectedRow).toBeVisible();
    await expect(selectedRow).toHaveAttribute("aria-current", "true");
  });
});
