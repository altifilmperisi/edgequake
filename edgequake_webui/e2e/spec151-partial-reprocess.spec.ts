/**
 * SPEC-151 E2E — Partial page reprocess UI (mocked).
 *
 * Quiet-header contract: idle detail has no page-health strip/tiles.
 * Selection lives in reprocess-pages-dialog → page-picker-grid.
 *
 * Run:
 *   cd edgequake_webui && pnpm exec playwright test e2e/spec151-partial-reprocess.spec.ts --project=chromium
 */

import { expect, test, type Page, type Route } from "@playwright/test";
import * as fs from "node:fs";
import * as path from "node:path";
import { GOTO_OPTS } from "./helpers/app-ready";
import { buildBlankPdf } from "./helpers/blank-pdf";
import {
  mockSpec038AdmissionRoutes,
  seedSpec038TenantContext,
} from "./helpers/spec038-admission-mocks";

const DOC_ID = "bbbbbbbb-0151-0151-0151-bbbbbbbbbbbb";
const TRACK_ID = "pdf-processing-spec151-track";
const SCREENSHOT_DIR = path.resolve(
  __dirname,
  "../../specs/151-partial-preprocess/e2e/screenshots",
);

async function fulfillJson(route: Route, status: number, body: unknown) {
  await route.fulfill({
    status,
    contentType: "application/json",
    body: JSON.stringify(body),
  });
}

async function spec151Screenshot(page: Page, name: string) {
  fs.mkdirSync(SCREENSHOT_DIR, { recursive: true });
  const file = path.join(SCREENSHOT_DIR, `${name}.png`);
  await page.screenshot({ path: file, fullPage: true });
  return file;
}

function listDoc() {
  return {
    id: DOC_ID,
    title: "SPEC-151 Fixture",
    file_name: "spec151.pdf",
    status: "completed",
    current_stage: "completed",
    source_type: "pdf",
    document_type: "pdf",
    mime_type: "application/pdf",
    pdf_id: DOC_ID,
    page_count: 4,
    track_id: null,
    admission_staging: false,
    created_at: "2026-01-01T00:00:00Z",
    updated_at: "2026-01-01T00:00:00Z",
  };
}

function healthPayload() {
  return {
    document_id: DOC_ID,
    page_count: 4,
    source: "stored",
    summary: { parse_failed: 1, figures_failed: 0, entities_failed: 1 },
    pages: [
      {
        page_number: 1,
        parse: { status: "ok" },
        figures: { status: "ok", count: 1 },
        entities: { status: "ok", chunk_count: 2, failed_chunk_count: 0 },
      },
      {
        page_number: 2,
        parse: { status: "failed", error: "placeholder OCR" },
        figures: { status: "pending" },
        entities: { status: "failed", chunk_count: 1, failed_chunk_count: 1 },
      },
      {
        page_number: 3,
        parse: { status: "ok" },
        figures: { status: "ok", count: 0 },
        entities: { status: "ok", chunk_count: 1, failed_chunk_count: 0 },
      },
      {
        page_number: 4,
        parse: { status: "ok" },
        figures: { status: "ok", count: 2 },
        entities: { status: "ok", chunk_count: 2, failed_chunk_count: 0 },
      },
    ],
  };
}

const MARKDOWN = [
  "<!-- edgequake-page:1 -->",
  "Page one ok",
  "<!-- edgequake-page:2 -->",
  "Page two failed",
  "<!-- edgequake-page:3 -->",
  "Page three",
  "<!-- edgequake-page:4 -->",
  "Page four",
].join("\n\n");

async function mockSpec151Stack(page: Page) {
  await mockSpec038AdmissionRoutes(page);
  await seedSpec038TenantContext(page);

  await page.route("**/api/v1/tenants**", async (route) => {
    if (route.request().method() !== "GET") {
      await route.fallback();
      return;
    }
    const url = route.request().url();
    if (url.includes("/workspaces")) {
      await fulfillJson(route, 200, {
        items: [
          {
            id: "ws-spec038-aaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
            tenant_id: "tenant-spec038-aaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
            name: "SPEC-038 Workspace",
            slug: "spec038-workspace",
            llm_provider: "ollama",
            llm_model: "gemma3:latest",
            embedding_provider: "ollama",
            embedding_model: "embeddinggemma:latest",
            created_at: "2026-01-01T00:00:00Z",
            updated_at: "2026-01-01T00:00:00Z",
          },
        ],
        total: 1,
        offset: 0,
        limit: 50,
      });
      return;
    }
    await fulfillJson(route, 200, {
      items: [
        {
          id: "tenant-spec038-aaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
          name: "SPEC038Tenant",
          slug: "spec038-tenant",
          created_at: "2026-01-01T00:00:00Z",
          updated_at: "2026-01-01T00:00:00Z",
        },
      ],
      total: 1,
    });
  });

  // Seed documents list (overrides SPEC-038 empty inventory). Detail URLs fall through.
  await page.route("**/api/v1/documents**", async (route) => {
    const method = route.request().method();
    const url = route.request().url();
    if (
      method !== "GET" ||
      url.includes("/documents/pdf") ||
      url.includes("/track/") ||
      url.includes(`/documents/${DOC_ID}`)
    ) {
      await route.fallback();
      return;
    }
    const doc = listDoc();
    await fulfillJson(route, 200, {
      items: [doc],
      documents: [doc],
      total: 1,
      page: 1,
      page_size: 500,
      status_counts: {
        pending: 0,
        processing: 0,
        completed: 1,
        failed: 0,
        cancelled: 0,
      },
    });
  });

  await page.route(`**/api/v1/documents/${DOC_ID}**`, async (route) => {
    const url = route.request().url();
    if (url.includes("/pages/health")) {
      await fulfillJson(route, 200, healthPayload());
      return;
    }
    if (url.includes("/pages/reprocess") && route.request().method() === "POST") {
      const body = route.request().postDataJSON() as {
        dry_run?: boolean;
        stages?: string[];
        pages?: string;
      };
      if (body.dry_run) {
        await fulfillJson(route, 200, {
          dry_run: true,
          plan: {
            pages: [2],
            requested_stages: body.stages ?? ["parse"],
            effective_stages: ["parse", "figures", "entities"],
            dirty_chunk_count: 1,
            reusable_chunk_count: 5,
            estimated_vision_calls: 1,
            warnings: ["Failed pages keep current content if OCR fails again."],
            suggest_full_reprocess: false,
          },
        });
        return;
      }
      await fulfillJson(route, 202, {
        dry_run: false,
        plan: {
          pages: [2],
          requested_stages: body.stages ?? ["parse"],
          effective_stages: ["parse", "figures", "entities"],
          dirty_chunk_count: 1,
          reusable_chunk_count: 5,
          estimated_vision_calls: 1,
          warnings: [],
          suggest_full_reprocess: false,
        },
        track_id: TRACK_ID,
        task_id: TRACK_ID,
      });
      return;
    }
    if (url.includes("/download") || url.includes("/content")) {
      await route.fallback();
      return;
    }
    await fulfillJson(route, 200, {
      ...listDoc(),
      content: MARKDOWN,
    });
  });

  await page.route(`**/api/v1/documents/pdf/${DOC_ID}/download**`, async (route) => {
    const pdf = buildBlankPdf(4);
    await route.fulfill({
      status: 200,
      contentType: "application/pdf",
      body: pdf,
    });
  });

  await page.route(`**/api/v1/pdf/${DOC_ID}/content**`, async (route) => {
    await fulfillJson(route, 200, {
      pdf_id: DOC_ID,
      markdown_content: MARKDOWN,
      page_count: 4,
    });
  });

  await page.route(`**/api/v1/documents/pdf/${DOC_ID}/content**`, async (route) => {
    await fulfillJson(route, 200, {
      pdf_id: DOC_ID,
      markdown_content: MARKDOWN,
      page_count: 4,
    });
  });

  // Idle pipeline so list feedback zone is driven by reprocess pin, not ActiveRuns.
  await page.route("**/api/v1/pipeline/status**", async (route) => {
    await fulfillJson(route, 200, {
      running_tasks: 0,
      processing_tasks: 0,
      pending_tasks: 0,
      queued_tasks: 0,
      is_busy: false,
    });
  });

  // Ingestion progress poll (ProgressPanelRow / detail progress slot).
  await page.route(`**/api/v1/ingestion/${TRACK_ID}/progress**`, async (route) => {
    await fulfillJson(route, 200, {
      track_id: TRACK_ID,
      document_id: DOC_ID,
      document_name: "spec151.pdf",
      status: "processing",
      progress: {
        percentage: 25,
        current_stage: "page_reprocess",
        latest_message: "Reprocessing pages",
        stages: [
          {
            stage: "page_reprocess",
            status: "running",
            percentage: 25,
            message: "Reprocessing pages",
          },
        ],
      },
      started_at: "2026-01-01T00:00:00Z",
      updated_at: "2026-01-01T00:00:01Z",
    });
  });
}

test.describe("SPEC-151 partial page reprocess", () => {
  test("detail header → picker → preview → enqueue + progress", async ({
    page,
  }) => {
    let lastEnqueue: {
      dry_run?: boolean;
      stages?: string[];
      pages?: string;
    } | null = null;
    /** idle → running (page 2) → done (page 2 ok); page 1 stays ok */
    let healthPhase: "idle" | "running" | "done" = "idle";
    let healthPollsAfterEnqueue = 0;
    const healthParseStatus: Array<[number, string]>[] = [];

    await mockSpec151Stack(page);

    // Override health so we can prove scoped page status transitions.
    await page.route(`**/api/v1/documents/${DOC_ID}/pages/health**`, async (route) => {
      if (healthPhase === "running") {
        healthPollsAfterEnqueue += 1;
        if (healthPollsAfterEnqueue >= 2) healthPhase = "done";
      }
      const base = healthPayload();
      const pages = base.pages.map((p) => {
        if (p.page_number === 2 && healthPhase === "running") {
          return {
            ...p,
            parse: { status: "running" },
            figures: { status: "pending" },
            entities: { status: "pending", chunk_count: 0, failed_chunk_count: 0 },
          };
        }
        if (p.page_number === 2 && healthPhase === "done") {
          return {
            ...p,
            parse: { status: "ok" },
            figures: { status: "ok", count: 0 },
            entities: { status: "ok", chunk_count: 1, failed_chunk_count: 0 },
          };
        }
        return p;
      });
      healthParseStatus.push(
        pages.map((p) => [p.page_number, p.parse.status] as [number, string]),
      );
      await fulfillJson(route, 200, { ...base, pages });
    });

    await page.route(
      `**/api/v1/documents/${DOC_ID}/pages/reprocess**`,
      async (route) => {
        if (route.request().method() !== "POST") {
          await route.fallback();
          return;
        }
        const body = route.request().postDataJSON() as {
          dry_run?: boolean;
          stages?: string[];
          pages?: string;
        };
        if (!body.dry_run) {
          lastEnqueue = body;
          healthPhase = "running";
        }
        await route.fallback();
      },
    );

    await page.goto(`/documents/${DOC_ID}`, GOTO_OPTS);

    // Quiet header: no idle page-health strip / tiles.
    await expect(
      page.getByTestId("detail-page-reprocess-pages-button"),
    ).toBeVisible({ timeout: 30_000 });
    await expect(page.getByTestId("page-health-strip")).toHaveCount(0);
    await spec151Screenshot(page, "01-detail-header");

    await page.getByTestId("detail-page-reprocess-pages-button").click();
    const dialog = page.getByTestId("reprocess-pages-dialog");
    await expect(dialog).toBeVisible();
    await expect(dialog.getByTestId("page-picker-grid")).toBeVisible();
    await expect(dialog.getByTestId("page-health-tile-2")).toHaveAttribute(
      "data-status",
      "failed",
    );
    await expect(dialog.getByTestId("page-health-tile-2")).toHaveAttribute(
      "data-page",
      "2",
    );
    await spec151Screenshot(page, "02-dialog-picker");

    await dialog.getByTestId("select-failed-pages").click();
    await expect(dialog.getByTestId("reprocess-selected-count")).toContainText(
      "1",
    );
    await dialog.getByTestId("page-picker-filter-failed").click();
    await expect(dialog.getByTestId("page-health-tile-2")).toBeVisible();

    await dialog.getByTestId("stage-card-parse").click();
    await expect(dialog.getByTestId("stage-card-figures")).toHaveAttribute(
      "data-locked",
      "true",
    );
    await dialog.getByTestId("reprocess-pages-preview").click();
    await expect(dialog.getByTestId("reprocess-impact-preview")).toBeVisible();
    await expect(dialog.getByTestId("never-downgrade-note")).toBeVisible();
    await spec151Screenshot(page, "03-dialog-preview-locked-stages");

    await dialog.getByTestId("reprocess-pages-confirm").click();
    await expect(dialog).toBeHidden({ timeout: 10_000 });
    expect(lastEnqueue).toEqual({
      dry_run: false,
      stages: ["parse"],
      pages: "2",
    });

    await expect(page.getByTestId("detail-page-reprocess-progress")).toBeVisible(
      { timeout: 10_000 },
    );
    // Health poll must keep page 1 ok and advance page 2 running → ok.
    await expect
      .poll(() => healthPhase, { timeout: 15_000 })
      .toBe("done");
    expect(
      healthParseStatus.some((snap) =>
        snap.some(([n, s]) => n === 2 && s === "running"),
      ),
    ).toBe(true);
    expect(
      healthParseStatus.every((snap) =>
        snap.some(([n, s]) => n === 1 && s === "ok"),
      ),
    ).toBe(true);
    expect(
      healthParseStatus.some((snap) =>
        snap.some(([n, s]) => n === 2 && s === "ok"),
      ),
    ).toBe(true);
    await spec151Screenshot(page, "04-after-enqueue");
  });

  test("409 conflict surfaces as toast", async ({ page }) => {
    await mockSpec151Stack(page);
    await page.route(
      `**/api/v1/documents/${DOC_ID}/pages/reprocess**`,
      async (route) => {
        if (route.request().method() !== "POST") {
          await route.fallback();
          return;
        }
        const body = route.request().postDataJSON() as { dry_run?: boolean };
        if (body.dry_run) {
          await fulfillJson(route, 200, {
            dry_run: true,
            plan: {
              pages: [2],
              requested_stages: ["entities"],
              effective_stages: ["entities"],
              dirty_chunk_count: 1,
              reusable_chunk_count: 5,
              estimated_vision_calls: 0,
              warnings: [],
              suggest_full_reprocess: false,
            },
          });
          return;
        }
        await fulfillJson(route, 409, {
          code: "CONFLICT",
          message: "Another task is already running on this document",
        });
      },
    );

    await page.goto(`/documents/${DOC_ID}`, GOTO_OPTS);
    await page.getByTestId("detail-page-reprocess-pages-button").click();
    const dialog = page.getByTestId("reprocess-pages-dialog");
    await expect(dialog).toBeVisible();
    await dialog.getByTestId("page-health-tile-2").click();
    await dialog.getByTestId("stage-card-entities").click();
    await dialog.getByTestId("reprocess-pages-confirm").click();
    await expect(
      page.getByText("Another task is already running on this document"),
    ).toBeVisible({ timeout: 10_000 });
    await spec151Screenshot(page, "05-conflict-409");
  });

  test("422 missing snapshot surfaces as toast", async ({ page }) => {
    await mockSpec151Stack(page);
    const snapshotMsg =
      "Partial page reprocess requires a prior extraction snapshot to reuse clean pages. Run a full document reprocess once, then retry selected-page extract.";
    await page.route(
      `**/api/v1/documents/${DOC_ID}/pages/reprocess**`,
      async (route) => {
        if (route.request().method() !== "POST") {
          await route.fallback();
          return;
        }
        await fulfillJson(route, 422, {
          code: "VALIDATION_ERROR",
          kind: "validation",
          message: snapshotMsg,
        });
      },
    );

    await page.goto(`/documents/${DOC_ID}`, GOTO_OPTS);
    await page.getByTestId("detail-page-reprocess-pages-button").click();
    const dialog = page.getByTestId("reprocess-pages-dialog");
    await expect(dialog).toBeVisible();
    await dialog.getByTestId("page-health-tile-2").click();
    await dialog.getByTestId("stage-card-entities").click();
    await dialog.getByTestId("reprocess-pages-confirm").click();
    await expect(page.getByText(/prior extraction snapshot/i)).toBeVisible({
      timeout: 10_000,
    });
    await spec151Screenshot(page, "05b-snapshot-422");
  });

  test("list menu opens pages dialog", async ({ page }) => {
    await mockSpec151Stack(page);
    await page.goto("/documents", GOTO_OPTS);

    const row = page.getByTestId(`document-row-${DOC_ID}`);
    await expect(row).toBeVisible({ timeout: 30_000 });
    await row.getByLabel("More actions").click();
    await page.getByTestId("list-reprocess-pages-action").click();

    const dialog = page.getByTestId("reprocess-pages-dialog");
    await expect(dialog).toBeVisible();
    await expect(dialog.getByTestId("page-picker-grid")).toBeVisible();
    await expect(dialog.getByTestId("page-health-tile-2")).toBeVisible();
    await spec151Screenshot(page, "06-list-menu-dialog");
  });

  test("bulk Reprocess → pages handoff", async ({ page }) => {
    await mockSpec151Stack(page);
    await page.goto("/documents", GOTO_OPTS);

    const row = page.getByTestId(`document-row-${DOC_ID}`);
    await expect(row).toBeVisible({ timeout: 30_000 });
    await row.getByRole("checkbox").click();
    await expect(page.getByTestId("batch-actions-bar")).toBeVisible();

    await page.getByTestId("batch-actions-bar").getByRole("button", {
      name: /reprocess/i,
    }).click();

    const bulk = page.getByTestId("bulk-reprocess-dialog");
    await expect(bulk).toBeVisible();
    await bulk.getByTestId("bulk-reprocess-option-pages").click();
    await bulk.getByTestId("bulk-reprocess-confirm").click();
    await expect(bulk).toBeHidden({ timeout: 10_000 });

    const dialog = page.getByTestId("reprocess-pages-dialog");
    await expect(dialog).toBeVisible();
    await expect(dialog.getByTestId("page-picker-grid")).toBeVisible();
    await spec151Screenshot(page, "07-bulk-pages-handoff");
  });

  test("detail full Reprocess → pages option", async ({ page }) => {
    await mockSpec151Stack(page);
    await page.goto(`/documents/${DOC_ID}`, GOTO_OPTS);

    await expect(page.getByTestId("detail-page-reprocess-button")).toBeVisible({
      timeout: 30_000,
    });
    await page.getByTestId("detail-page-reprocess-button").click();

    const reprocess = page.getByTestId("reprocess-dialog");
    await expect(reprocess).toBeVisible();
    await reprocess.getByTestId("reprocess-option-pages").click();
    await reprocess.getByTestId("reprocess-dialog-confirm").click();

    const dialog = page.getByTestId("reprocess-pages-dialog");
    await expect(dialog).toBeVisible();
    await expect(dialog.getByTestId("page-picker-grid")).toBeVisible();
    await spec151Screenshot(page, "08-full-reprocess-to-pages");
  });

  test("range Apply selects pages in picker", async ({ page }) => {
    await mockSpec151Stack(page);
    await page.goto(`/documents/${DOC_ID}`, GOTO_OPTS);

    await page.getByTestId("detail-page-reprocess-pages-button").click();
    const dialog = page.getByTestId("reprocess-pages-dialog");
    await expect(dialog).toBeVisible();

    const input = dialog.getByTestId("reprocess-pages-range-input");
    await input.fill("2-3");
    await dialog.getByTestId("reprocess-pages-apply-range").click();

    await expect(dialog.getByTestId("reprocess-selected-count")).toContainText(
      "2",
    );
    await expect(dialog.getByTestId("page-health-tile-2")).toHaveAttribute(
      "aria-selected",
      "true",
    );
    await expect(dialog.getByTestId("page-health-tile-3")).toHaveAttribute(
      "aria-selected",
      "true",
    );
    await expect(dialog.getByTestId("reprocess-ready-summary")).toBeVisible();
    await spec151Screenshot(page, "09-range-apply");
  });

  test("list enqueue pins feedback-zone progress", async ({ page }) => {
    await mockSpec151Stack(page);
    await page.goto("/documents", GOTO_OPTS);

    const row = page.getByTestId(`document-row-${DOC_ID}`);
    await expect(row).toBeVisible({ timeout: 30_000 });
    await row.getByLabel("More actions").click();
    await page.getByTestId("list-reprocess-pages-action").click();

    const dialog = page.getByTestId("reprocess-pages-dialog");
    await dialog.getByTestId("page-health-tile-2").click();
    await dialog.getByTestId("stage-card-entities").click();
    await dialog.getByTestId("reprocess-pages-confirm").click();
    await expect(dialog).toBeHidden({ timeout: 10_000 });

    await expect(
      page.getByTestId("spec051-reprocess-progress-panels"),
    ).toBeVisible({ timeout: 10_000 });
    await expect(page.getByTestId("spec051-reprocess-panel")).toBeVisible();
    await spec151Screenshot(page, "10-list-progress-pin");
  });
});
