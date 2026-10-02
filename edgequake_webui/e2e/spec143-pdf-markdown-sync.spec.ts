/**
 * SPEC-143 E2E — PDF continuous scroll + Markdown page sync.
 *
 * Unfakable: mocked routes + fixture PDF/markdown with page markers.
 * Asserts data-page / data-eq-page / URL — not screenshots alone.
 *
 * Run:
 *   cd edgequake_webui && pnpm exec playwright test e2e/spec143-pdf-markdown-sync.spec.ts --project=chromium
 */

import { expect, test, type Page, type Route } from "@playwright/test";
import * as path from "node:path";
import { GOTO_OPTS } from "./helpers/app-ready";
import { buildBlankPdf } from "./helpers/blank-pdf";
import {
  mockSpec038AdmissionRoutes,
  seedSpec038TenantContext,
} from "./helpers/spec038-admission-mocks";

const DOC_ID = "dddddddd-0143-0143-0143-dddddddddddd";
const DOC_NO_MARKERS = "eeeeeeee-0143-0143-0143-eeeeeeeeeeee";
/** Separate id so React Query cannot reuse a 4-page fixture for the windowed case. */
const DOC_WINDOWED = "ffffffff-0143-0143-0143-ffffffffffff";
const PAGE_SYNC_MODE_STORAGE_KEY = "eq-page-sync-mode";

/** Scroll markdown pane so page N sits at the reading line (gesture + scrollTop). */
async function scrollMdToPage(
  viewer: ReturnType<Page["getByTestId"]>,
  pageNum: number,
): Promise<void> {
  await viewer.getByTestId("md-scroll-container").evaluate((root, n) => {
    root.dispatchEvent(
      new PointerEvent("pointerdown", { bubbles: true, pointerId: 1 }),
    );
    root.dispatchEvent(new WheelEvent("wheel", { deltaY: 40, bubbles: true }));
    const el =
      (root.querySelector(`#eq-md-page-${n}`) as HTMLElement | null) ??
      (root.querySelector(`[data-eq-page="${n}"]`) as HTMLElement | null);
    if (!el) return;
    const rootTop = root.getBoundingClientRect().top;
    root.scrollTop += el.getBoundingClientRect().top - rootTop;
  }, pageNum);
}

/** Scroll PDF stack so sheet N sits at the reading line. */
async function scrollPdfToSheet(
  viewer: ReturnType<Page["getByTestId"]>,
  pageNum: number,
): Promise<void> {
  const scroll = viewer.getByTestId("pdf-scroll-container");
  await scroll.evaluate((root, n) => {
    root.dispatchEvent(
      new PointerEvent("pointerdown", { bubbles: true, pointerId: 1 }),
    );
    root.dispatchEvent(new WheelEvent("wheel", { deltaY: 40, bubbles: true }));
    const el = root.querySelector(
      `[data-testid="pdf-page-sheet"][data-page="${n}"]`,
    ) as HTMLElement | null;
    if (!el) return;
    const rootTop = root.getBoundingClientRect().top;
    root.scrollTop += el.getBoundingClientRect().top - rootTop;
  }, pageNum);
}

/** Poll that a pane indicator stays on `pageNum` across a settle window. */
async function expectPageStable(
  locator: ReturnType<Page["getByTestId"]>,
  pageNum: number,
  settleMs = 1000,
): Promise<void> {
  const deadline = Date.now() + settleMs;
  while (Date.now() < deadline) {
    await expect(locator).toHaveAttribute("data-page", String(pageNum));
    await new Promise((r) => setTimeout(r, 100));
  }
  await expect(locator).toHaveAttribute("data-page", String(pageNum));
}

async function fulfillJson(route: Route, status: number, body: unknown) {
  await route.fulfill({
    status,
    contentType: "application/json",
    body: JSON.stringify(body),
  });
}

function fixtureMarkdown(
  withMarkers: boolean,
  pageCount = 4,
  /** Extra paragraph repeats per page (8 default; raise to force virtualization). */
  padRepeats = 8,
): string {
  if (!withMarkers) {
    return ["# Fixture", "No page markers here.", "Still readable."].join("\n");
  }
  const parts = ["# Fixture"];
  for (let n = 1; n <= pageCount; n++) {
    parts.push(`<!-- edgequake-page:${n} -->`);
    parts.push(`## Page ${n} section UNIQUE_MARKER_PAGE_${n}`);
    // Pad so each page section is tall enough to scroll independently.
    parts.push(`Content for page ${n}.\n\n`.repeat(padRepeats));
  }
  return parts.join("\n");
}

/** PDF sheet flush to scrollport top. */
const ALIGN_EPS_PDF_PX = 28;
/**
 * Markdown allows sticky page badge (~28px) plus prose margin above the
 * injected 1px anchor.
 */
const ALIGN_EPS_MD_PX = 48;

/**
 * Unfakable: indicators + URL + geometry (sheet/anchor at scrollport top).
 */
async function assertPanesAligned(page: Page, n: number): Promise<void> {
  const viewer = page.getByTestId("side-by-side-viewer");
  await expect(viewer.getByTestId("pdf-page-indicator")).toHaveAttribute(
    "data-page",
    String(n),
    { timeout: 20_000 },
  );
  await expect(viewer.getByTestId("md-page-indicator")).toHaveAttribute(
    "data-page",
    String(n),
    { timeout: 20_000 },
  );
  await expect(page).toHaveURL(new RegExp(`[?&]page=${n}(?:&|$)`), {
    timeout: 10_000,
  });

  const pdfScroll = viewer.getByTestId("pdf-scroll-container");
  const mdScroll = viewer.getByTestId("md-scroll-container");
  const sheet = viewer.locator(
    `[data-testid="pdf-page-sheet"][data-page="${n}"]`,
  );
  await expect(sheet).toBeAttached();
  // Scope each alternative — a comma selector after getByTestId unscopes the
  // second branch and races the wrong node.
  const anchorById = viewer.locator(`#eq-md-page-${n}`);
  const anchorByAttr = viewer.locator(`[data-eq-page="${n}"]`).first();
  const markerText = viewer.getByText(`UNIQUE_MARKER_PAGE_${n}`, {
    exact: false,
  });
  await expect
    .poll(
      async () => {
        if (await anchorById.count()) return "id";
        if (await anchorByAttr.count()) return "attr";
        if (await markerText.count()) return "text";
        return "";
      },
      { timeout: 30_000 },
    )
    .not.toBe("");

  await expect
    .poll(
      async () => {
        return page.evaluate(
          ({ scrollSel, sheetSel }) => {
            const root = document.querySelector(scrollSel) as HTMLElement | null;
            const el = document.querySelector(sheetSel) as HTMLElement | null;
            if (!root || !el) return Number.POSITIVE_INFINITY;
            const rootTop = root.getBoundingClientRect().top;
            return Math.abs(el.getBoundingClientRect().top - rootTop);
          },
          {
            scrollSel:
              '[data-testid="side-by-side-viewer"] [data-testid="pdf-scroll-container"]',
            sheetSel: `[data-testid="side-by-side-viewer"] [data-testid="pdf-page-sheet"][data-page="${n}"]`,
          },
        );
      },
      { timeout: 20_000 },
    )
    .toBeLessThanOrEqual(ALIGN_EPS_PDF_PX);

  await expect
    .poll(
      async () => {
        return page.evaluate(
          ({ scrollSel, pageNum }) => {
            const root = document.querySelector(scrollSel) as HTMLElement | null;
            if (!root) return Number.POSITIVE_INFINITY;
            const el =
              (root.querySelector(
                `#eq-md-page-${pageNum}`,
              ) as HTMLElement | null) ??
              (root.querySelector(
                `[data-eq-page="${pageNum}"]`,
              ) as HTMLElement | null);
            if (!el) return Number.POSITIVE_INFINITY;
            const rootTop = root.getBoundingClientRect().top;
            return Math.abs(el.getBoundingClientRect().top - rootTop);
          },
          {
            scrollSel:
              '[data-testid="side-by-side-viewer"] [data-testid="md-scroll-container"]',
            pageNum: n,
          },
        );
      },
      { timeout: 20_000 },
    )
    .toBeLessThanOrEqual(ALIGN_EPS_MD_PX);

  await expect(pdfScroll).toBeVisible();
  await expect(mdScroll).toBeVisible();
}

async function mockPdfDocumentStack(
  page: Page,
  opts: {
    docId: string;
    withMarkers: boolean;
    pageCount?: number;
    /** Paragraph repeats per page in fixture markdown. */
    padRepeats?: number;
  },
) {
  const { docId, withMarkers } = opts;
  const pageCount = opts.pageCount ?? 4;
  const padRepeats = opts.padRepeats ?? 8;
  await mockSpec038AdmissionRoutes(page);
  await seedSpec038TenantContext(page);

  // TenantGuard calls GET /tenants?limit=&offset= — glob without ** misses query strings
  // and the SPEC-038 catch-all returns { items: [] } → "Create Tenant" gate.
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
    if (/\/tenants\/[^/?]+(?:\?|$)/.test(url)) {
      await fulfillJson(route, 200, {
        id: "tenant-spec038-aaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
        name: "SPEC038Tenant",
        slug: "spec038-tenant",
        created_at: "2026-01-01T00:00:00Z",
        updated_at: "2026-01-01T00:00:00Z",
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
      offset: 0,
      limit: 50,
    });
  });

  await page.reload(GOTO_OPTS);

  const pdfBytes = buildBlankPdf(pageCount);
  const markdown = fixtureMarkdown(withMarkers, pageCount, padRepeats);

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
  }, pdfBytes.toString("base64"));

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

  await page.route(`**/api/v1/documents/${docId}**`, async (route) => {
    if (route.request().method() !== "GET") {
      await route.fallback();
      return;
    }
    const url = route.request().url();
    if (url.includes("/lineage")) {
      await fulfillJson(route, 200, {
        document_id: docId,
        metadata: { title: "Fixture.pdf" },
        lineage: {
          document_name: "Fixture.pdf",
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
              chunk_id: "page4-chunk",
              chunk_index: 3,
              start_line: 10,
              end_line: 14,
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
    // SPEC-151: /pages/health must not be satisfied by the layout /pages mock.
    if (url.includes("/pages/health")) {
      await fulfillJson(route, 200, {
        document_id: docId,
        page_count: pageCount,
        source: "stored",
        summary: { parse_failed: 0, figures_failed: 0, entities_failed: 0 },
        pages: Array.from({ length: pageCount }, (_, i) => ({
          page_number: i + 1,
          parse: { status: "ok" },
          figures: { status: "ok", count: 0 },
          entities: { status: "ok", chunk_count: 1, failed_chunk_count: 0 },
        })),
      });
      return;
    }
    if (url.includes("/pages")) {
      await fulfillJson(route, 200, {
        document_id: docId,
        pages: Array.from({ length: pageCount }, (_, i) => ({
          page_number: i + 1,
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
      id: docId,
      title: "Fixture.pdf",
      file_name: "Fixture.pdf",
      status: "completed",
      source_type: "pdf",
      mime_type: "application/pdf",
      pdf_id: docId,
      content: markdown,
      chunk_count: 2,
      entity_count: 0,
      relationship_count: 0,
      created_at: "2026-01-01T00:00:00Z",
      updated_at: "2026-01-01T00:00:00Z",
      track_id: null,
    });
  });

  await page.route(`**/api/v1/documents/pdf/${docId}/content`, async (route) => {
    await fulfillJson(route, 200, {
      pdf_id: docId,
      document_id: docId,
      filename: "Fixture.pdf",
      file_size_bytes: pdfBytes.length,
      content_type: "application/pdf",
      markdown_content: markdown,
      is_processed: true,
    });
  });
}

test.describe("SPEC-143 PDF / Markdown sync", () => {
  test.beforeEach(async ({ page }, testInfo) => {
    // Isolate mode between tests. Persistence case keeps storage across reload.
    if (testInfo.title.includes("E-143-persist")) return;
    await page.addInitScript((key) => {
      try {
        localStorage.removeItem(key);
      } catch {
        /* private mode */
      }
    }, PAGE_SYNC_MODE_STORAGE_KEY);
  });

  test("E-143-01: side-by-side shows page indicator and MD anchors", async ({
    page,
  }) => {
    test.setTimeout(90_000);
    await page.setViewportSize({ width: 1400, height: 900 });
    await mockPdfDocumentStack(page, { docId: DOC_ID, withMarkers: true });

    await page.goto(`/documents/${DOC_ID}`, GOTO_OPTS);

    await expect(page.getByTestId("side-by-side-viewer")).toBeVisible({
      timeout: 30_000,
    });
    await expect(page.getByTestId("pdf-page-indicator")).toHaveAttribute(
      "data-page",
      "1",
      { timeout: 45_000 },
    );
    await expect(page.locator('[data-eq-page="1"]').first()).toBeAttached({
      timeout: 30_000,
    });
    await expect(
      page.getByTestId("side-by-side-viewer").getByTestId("pdf-md-sync-mode"),
    ).toHaveAttribute("data-sync", "pdf-to-md");
  });

  test("E-143-02/06: toolbar next updates data-page and URL", async ({ page }) => {
    test.setTimeout(90_000);
    await page.setViewportSize({ width: 1400, height: 900 });
    await mockPdfDocumentStack(page, { docId: DOC_ID, withMarkers: true });

    await page.goto(`/documents/${DOC_ID}`, GOTO_OPTS);
    await expect(page.getByTestId("pdf-page-indicator")).toHaveAttribute(
      "data-page",
      "1",
      { timeout: 45_000 },
    );

    await page.getByTestId("pdf-next-page").click();
    await expect(page.getByTestId("pdf-page-indicator")).toHaveAttribute(
      "data-page",
      "2",
      { timeout: 15_000 },
    );
    await expect(page).toHaveURL(/[?&]page=2(?:&|$)/, { timeout: 10_000 });
  });

  test("E-143-03: PDF→MD toolbar next settles both panes on page 4", async ({
    page,
  }) => {
    test.setTimeout(90_000);
    await page.setViewportSize({ width: 1400, height: 900 });
    await mockPdfDocumentStack(page, { docId: DOC_ID, withMarkers: true });

    await page.goto(`/documents/${DOC_ID}`, GOTO_OPTS);
    const viewer = page.getByTestId("side-by-side-viewer");
    await expect(viewer.getByTestId("pdf-md-sync-mode")).toHaveAttribute(
      "data-sync",
      "pdf-to-md",
      { timeout: 45_000 },
    );
    await expect(viewer.getByTestId("pdf-page-indicator")).toHaveAttribute(
      "data-page",
      "1",
      { timeout: 45_000 },
    );

    await viewer.getByTestId("pdf-next-page").click();
    await viewer.getByTestId("pdf-next-page").click();
    await viewer.getByTestId("pdf-next-page").click();
    await assertPanesAligned(page, 4);
  });

  test("E-143-04: MD→PDF markdown scroll drives PDF to page 4", async ({
    page,
  }) => {
    test.setTimeout(90_000);
    await page.setViewportSize({ width: 1400, height: 900 });
    await mockPdfDocumentStack(page, { docId: DOC_ID, withMarkers: true });

    await page.goto(`/documents/${DOC_ID}`, GOTO_OPTS);
    const viewer = page.getByTestId("side-by-side-viewer");
    await expect(viewer.getByTestId("pdf-page-indicator")).toHaveAttribute(
      "data-page",
      "1",
      { timeout: 45_000 },
    );
    await viewer.getByTestId("pdf-md-sync-mode-md-to-pdf").click();
    await expect(viewer.getByTestId("pdf-md-sync-mode")).toHaveAttribute(
      "data-sync",
      "md-to-pdf",
    );
    await expect(viewer.locator("#eq-md-page-4")).toBeAttached({
      timeout: 30_000,
    });

    await scrollMdToPage(viewer, 4);
    await assertPanesAligned(page, 4);
  });

  test("E-143-05: None keeps markdown scrollTop when PDF page changes", async ({
    page,
  }) => {
    test.setTimeout(90_000);
    await page.setViewportSize({ width: 1400, height: 900 });
    await mockPdfDocumentStack(page, { docId: DOC_ID, withMarkers: true });

    await page.goto(`/documents/${DOC_ID}`, GOTO_OPTS);
    const viewer = page.getByTestId("side-by-side-viewer");
    await expect(viewer.getByTestId("pdf-md-sync-mode")).toBeVisible({
      timeout: 45_000,
    });

    const mdScroll = viewer.getByTestId("md-scroll-container");
    await mdScroll.evaluate((el) => {
      el.scrollTop = 0;
    });
    const before = await mdScroll.evaluate((el) => el.scrollTop);

    await viewer.getByTestId("pdf-md-sync-mode-none").click();
    await expect(viewer.getByTestId("pdf-md-sync-mode")).toHaveAttribute(
      "data-sync",
      "none",
    );

    await viewer.getByTestId("pdf-next-page").click();
    await viewer.getByTestId("pdf-next-page").click();
    await expect(viewer.getByTestId("pdf-page-indicator")).toHaveAttribute(
      "data-page",
      "3",
      { timeout: 15_000 },
    );

    await expect
      .poll(async () => mdScroll.evaluate((el) => el.scrollTop), {
        timeout: 2_000,
      })
      .toBeLessThan(before + 8);
  });

  test("E-143-05b: PDF→MD ignores markdown scroll", async ({ page }) => {
    test.setTimeout(90_000);
    await page.setViewportSize({ width: 1400, height: 900 });
    await mockPdfDocumentStack(page, { docId: DOC_ID, withMarkers: true });

    await page.goto(`/documents/${DOC_ID}`, GOTO_OPTS);
    const viewer = page.getByTestId("side-by-side-viewer");
    await expect(viewer.getByTestId("pdf-md-sync-mode")).toHaveAttribute(
      "data-sync",
      "pdf-to-md",
      { timeout: 45_000 },
    );
    await expect(viewer.getByTestId("pdf-page-indicator")).toHaveAttribute(
      "data-page",
      "1",
      { timeout: 45_000 },
    );
    await expect(viewer.locator("#eq-md-page-4")).toBeAttached({
      timeout: 30_000,
    });

    await scrollMdToPage(viewer, 4);
    await expectPageStable(viewer.getByTestId("pdf-page-indicator"), 1);
    await expect(page).not.toHaveURL(/[?&]page=4(?:&|$)/);
  });

  test("E-143-05c: MD→PDF ignores PDF toolbar navigation", async ({ page }) => {
    test.setTimeout(90_000);
    await page.setViewportSize({ width: 1400, height: 900 });
    await mockPdfDocumentStack(page, { docId: DOC_ID, withMarkers: true });

    await page.goto(`/documents/${DOC_ID}`, GOTO_OPTS);
    const viewer = page.getByTestId("side-by-side-viewer");
    await expect(viewer.getByTestId("pdf-page-indicator")).toHaveAttribute(
      "data-page",
      "1",
      { timeout: 45_000 },
    );
    await viewer.getByTestId("pdf-md-sync-mode-md-to-pdf").click();
    await expect(viewer.getByTestId("pdf-md-sync-mode")).toHaveAttribute(
      "data-sync",
      "md-to-pdf",
    );
    await expect(viewer.getByTestId("md-page-indicator")).toHaveAttribute(
      "data-page",
      "1",
      { timeout: 15_000 },
    );

    // Controlled PDF: toolbar clicks must not publish. Indicator stays on shared page.
    await viewer.getByTestId("pdf-next-page").click();
    await viewer.getByTestId("pdf-next-page").click();
    await expectPageStable(viewer.getByTestId("pdf-page-indicator"), 1);
    await expectPageStable(viewer.getByTestId("md-page-indicator"), 1);
    await expect(page).not.toHaveURL(/[?&]page=(2|3)(?:&|$)/);
  });

  test("E-143-05d: None does not write ?page= when PDF navigates", async ({
    page,
  }) => {
    test.setTimeout(90_000);
    await page.setViewportSize({ width: 1400, height: 900 });
    await mockPdfDocumentStack(page, { docId: DOC_ID, withMarkers: true });

    await page.goto(`/documents/${DOC_ID}`, GOTO_OPTS);
    const viewer = page.getByTestId("side-by-side-viewer");
    await viewer.getByTestId("pdf-md-sync-mode-none").click();
    await expect(viewer.getByTestId("pdf-md-sync-mode")).toHaveAttribute(
      "data-sync",
      "none",
      { timeout: 15_000 },
    );

    const mdScroll = viewer.getByTestId("md-scroll-container");
    await mdScroll.evaluate((el) => {
      el.scrollTop = 0;
    });
    const before = await mdScroll.evaluate((el) => el.scrollTop);

    // Wait for page 2 before the second click — a double-click race left CI
    // stuck at data-page=2 on E-143-05d retries.
    await viewer.getByTestId("pdf-next-page").click();
    await expect(viewer.getByTestId("pdf-page-indicator")).toHaveAttribute(
      "data-page",
      "2",
      { timeout: 15_000 },
    );
    await viewer.getByTestId("pdf-next-page").click();
    await expect(viewer.getByTestId("pdf-page-indicator")).toHaveAttribute(
      "data-page",
      "3",
      { timeout: 15_000 },
    );

    await expect
      .poll(() => page.url(), { timeout: 2_000 })
      .not.toMatch(/[?&]page=\d+/);
    await expect
      .poll(async () => mdScroll.evaluate((el) => el.scrollTop), {
        timeout: 2_000,
      })
      .toBeLessThan(before + 8);
  });

  test("E-143-persist: MD→PDF survives reload", async ({ page }) => {
    test.setTimeout(90_000);
    await page.setViewportSize({ width: 1400, height: 900 });
    await mockPdfDocumentStack(page, { docId: DOC_ID, withMarkers: true });

    await page.goto(`/documents/${DOC_ID}`, GOTO_OPTS);
    const viewer = page.getByTestId("side-by-side-viewer");
    await expect(viewer.getByTestId("pdf-md-sync-mode")).toHaveAttribute(
      "data-sync",
      "pdf-to-md",
      { timeout: 45_000 },
    );
    await viewer.getByTestId("pdf-md-sync-mode-md-to-pdf").click();
    await expect(viewer.getByTestId("pdf-md-sync-mode")).toHaveAttribute(
      "data-sync",
      "md-to-pdf",
    );
    await expect
      .poll(
        () =>
          page.evaluate(
            (key) => localStorage.getItem(key),
            PAGE_SYNC_MODE_STORAGE_KEY,
          ),
        { timeout: 5_000 },
      )
      .toBe("md-to-pdf");

    // Remock re-seeds tenant storage but preserves eq-page-sync-mode, then a
    // fresh document navigation remounts the controller (hydration read).
    await mockPdfDocumentStack(page, { docId: DOC_ID, withMarkers: true });
    await expect
      .poll(
        () =>
          page.evaluate(
            (key) => localStorage.getItem(key),
            PAGE_SYNC_MODE_STORAGE_KEY,
          ),
        { timeout: 5_000 },
      )
      .toBe("md-to-pdf");
    await page.goto(`/documents/${DOC_ID}`, GOTO_OPTS);
    await expect
      .poll(
        async () =>
          page
            .getByTestId("side-by-side-viewer")
            .getByTestId("pdf-md-sync-mode")
            .getAttribute("data-sync"),
        { timeout: 45_000 },
      )
      .toBe("md-to-pdf");
  });

  test("E-143-07: deeplink ?page=4 aligns both panes", async ({ page }) => {
    test.setTimeout(90_000);
    await page.setViewportSize({ width: 1400, height: 900 });
    await mockPdfDocumentStack(page, { docId: DOC_ID, withMarkers: true });

    await page.goto(`/documents/${DOC_ID}?page=4`, GOTO_OPTS);
    await expect(page).toHaveURL(/[?&]page=4(?:&|$)/);
    await assertPanesAligned(page, 4);
  });

  test("E-143-scroll: PDF scroll container to sheet 3 aligns markdown + URL", async ({
    page,
  }) => {
    test.setTimeout(90_000);
    await page.setViewportSize({ width: 1400, height: 900 });
    await mockPdfDocumentStack(page, { docId: DOC_ID, withMarkers: true });

    await page.goto(`/documents/${DOC_ID}`, GOTO_OPTS);
    const viewer = page.getByTestId("side-by-side-viewer");
    await expect(viewer.getByTestId("pdf-page-indicator")).toHaveAttribute(
      "data-page",
      "1",
      { timeout: 45_000 },
    );
    await expect(viewer.getByTestId("pdf-page-indicator")).toContainText("1 / 4", {
      timeout: 45_000,
    });

    const scroll = viewer.getByTestId("pdf-scroll-container");
    await expect
      .poll(
        () =>
          scroll.evaluate((el) => el.scrollHeight > el.clientHeight + 40),
        { timeout: 15_000 },
      )
      .toBe(true);
    await scrollPdfToSheet(viewer, 3);
    await assertPanesAligned(page, 3);
  });

  test("E-143-window: ?page=22 aligns both panes on 25-page windowed stack", async ({
    page,
  }) => {
    test.setTimeout(120_000);
    await page.setViewportSize({ width: 1400, height: 900 });
    await mockPdfDocumentStack(page, {
      docId: DOC_WINDOWED,
      withMarkers: true,
      pageCount: 25,
    });

    await page.goto(`/documents/${DOC_WINDOWED}?page=22`, GOTO_OPTS);
    await expect(page).toHaveURL(/[?&]page=22(?:&|$)/);
    const viewer = page.getByTestId("side-by-side-viewer");
    await expect(viewer.getByTestId("pdf-page-sheet")).toHaveCount(25, {
      timeout: 60_000,
    });
    // Windowed stack still mounts placeholder sheets for out-of-window pages.
    await expect(
      viewer.locator('[data-testid="pdf-page-sheet"][data-page="1"]'),
    ).toBeAttached();
    await expect(
      viewer.locator('[data-testid="pdf-page-sheet"][data-page="22"]'),
    ).toBeAttached();
    await expect(viewer.getByTestId("pdf-page-indicator")).toHaveAttribute(
      "data-page",
      "22",
      { timeout: 45_000 },
    );
    await expect(viewer.getByTestId("md-page-indicator")).toHaveAttribute(
      "data-page",
      "22",
      { timeout: 30_000 },
    );

    await expect(viewer.locator("#eq-md-page-22")).toBeAttached({
      timeout: 20_000,
    });
    await assertPanesAligned(page, 22);
  });

  test("E-143-08: no markers disables sync control; PDF still navigable", async ({
    page,
  }) => {
    test.setTimeout(90_000);
    await page.setViewportSize({ width: 1400, height: 900 });
    await mockPdfDocumentStack(page, {
      docId: DOC_NO_MARKERS,
      withMarkers: false,
    });

    await page.goto(`/documents/${DOC_NO_MARKERS}`, GOTO_OPTS);
    const viewer = page.getByTestId("side-by-side-viewer");
    await expect(viewer.getByTestId("pdf-md-sync-mode")).toHaveAttribute(
      "data-sync",
      "none",
      { timeout: 45_000 },
    );
    await expect(viewer.getByTestId("pdf-md-sync-mode-pdf-to-md")).toBeDisabled();
    await expect(viewer.getByTestId("pdf-page-indicator")).toHaveAttribute(
      "data-page",
      "1",
      { timeout: 45_000 },
    );
    await viewer.getByTestId("pdf-next-page").click();
    await expect(viewer.getByTestId("pdf-page-indicator")).toHaveAttribute(
      "data-page",
      "2",
      { timeout: 15_000 },
    );
  });

  test("E-143-stack: continuous stack mounts all page sheets", async ({
    page,
  }) => {
    test.setTimeout(90_000);
    await page.setViewportSize({ width: 1400, height: 900 });
    await mockPdfDocumentStack(page, { docId: DOC_ID, withMarkers: true });

    await page.goto(`/documents/${DOC_ID}`, GOTO_OPTS);
    const viewer = page.getByTestId("side-by-side-viewer");
    await expect(viewer.getByTestId("pdf-scroll-container")).toBeVisible({
      timeout: 45_000,
    });
    await expect(viewer.getByTestId("pdf-page-sheet")).toHaveCount(4, {
      timeout: 45_000,
    });
    await expect(viewer.getByTestId("pdf-page-indicator")).toContainText("/ 4", {
      timeout: 45_000,
    });
    for (const n of [1, 2, 3, 4]) {
      await expect(
        viewer.locator(`[data-testid="pdf-page-sheet"][data-page="${n}"]`),
      ).toBeAttached();
    }

    const scroll = viewer.getByTestId("pdf-scroll-container");
    await expect
      .poll(
        () =>
          scroll.evaluate((el) => el.scrollHeight > el.clientHeight + 40),
        { timeout: 15_000 },
      )
      .toBe(true);
    await scroll.evaluate((el) => {
      el.dispatchEvent(
        new PointerEvent("pointerdown", { bubbles: true, pointerId: 1 }),
      );
      el.dispatchEvent(new WheelEvent("wheel", { deltaY: 80, bubbles: true }));
      el.scrollTop = el.scrollHeight;
    });
    await expect
      .poll(
        async () =>
          Number(
            await viewer.getByTestId("pdf-page-indicator").getAttribute("data-page"),
          ),
        { timeout: 15_000 },
      )
      .toBeGreaterThanOrEqual(2);
  });
});
