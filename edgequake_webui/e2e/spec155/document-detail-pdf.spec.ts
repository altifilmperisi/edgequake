/**
 * SPEC-155 — document detail page: PDF loads on demand (hermetic).
 *
 * Contract:
 *  - the viewer never downloads the whole file up front; pdf.js issues `Range`
 *    requests and first paint transfers a fraction of the bytes;
 *  - only a small window of pages around the reading position is rasterised,
 *    even for short documents;
 *  - a server that ignores `Range` still renders (graceful 200 fallback).
 */
import { expect, test } from "@playwright/test";
import { prepareSpec155Page } from "./helpers/mock-api";
import { buildMockPdf, mockRangePdfRoute } from "./helpers/mock-pdf";

const DOC_ID = "cccccccc-cccc-4ccc-8ccc-cccccccccccc";
const PAGES = 30;

const DOC = {
  id: DOC_ID,
  pdf_id: DOC_ID,
  title: "mock-long-report.pdf",
  file_name: "mock-long-report.pdf",
  status: "completed",
  source_type: "pdf",
  mime_type: "application/pdf",
  page_count: PAGES,
  content: "",
  created_at: "2026-10-01T06:00:00Z",
  updated_at: "2026-10-01T06:30:00Z",
};

async function openDetail(
  page: import("@playwright/test").Page,
  opts: { supportRange?: boolean; query?: string } = {},
) {
  await prepareSpec155Page(page, { documents: [DOC] });
  await page.route(/\/api\/v1\/documents\/pdf\/[^/?]+$/, (route) =>
    route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        pdf_id: DOC_ID,
        document_id: DOC_ID,
        filename: DOC.file_name,
        file_size_bytes: 1,
        content_type: "application/pdf",
        markdown_content: "# Mock\n\nBody text.",
        is_processed: true,
      }),
    }),
  );
  const pdf = buildMockPdf(PAGES);
  const stats = await mockRangePdfRoute(page, pdf, {
    supportRange: opts.supportRange ?? true,
  });
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto(`/documents/${DOC_ID}${opts.query ?? ""}`, {
    waitUntil: "domcontentloaded",
  });
  await expect(page.getByTestId("pdf-page-indicator")).toContainText(`/ ${PAGES}`, {
    timeout: 30_000,
  });
  return stats;
}

test.describe("SPEC-155 document detail — PDF on demand @spec155", () => {
  test("first paint uses range requests and a fraction of the file", async ({ page }) => {
    const stats = await openDetail(page);
    await expect(page.locator("canvas").first()).toBeVisible({ timeout: 30_000 });
    // Let any eager prefetch show itself before asserting.
    await page.waitForTimeout(1500);

    expect(stats.rangeRequests, "pdf.js must use Range").toBeGreaterThan(0);
    expect(
      stats.bytesServed,
      `served ${stats.bytesServed} of ${stats.totalBytes}: ${stats.log.join(" ")}`,
    ).toBeLessThan(stats.totalBytes * 0.4);
  });

  test("only a window of pages is rasterised", async ({ page }) => {
    await openDetail(page);
    await expect(page.locator("canvas").first()).toBeVisible({ timeout: 30_000 });
    await page.waitForTimeout(1000);
    const canvases = await page.locator("[data-testid='pdf-page-sheet'] canvas").count();
    expect(canvases).toBeGreaterThan(0);
    expect(canvases).toBeLessThanOrEqual(5);
    expect(await page.getByTestId("pdf-page-sheet").count()).toBe(PAGES);
  });

  test("deep link to a late page does not pull the whole file", async ({ page }) => {
    const stats = await openDetail(page, { query: "?page=25" });
    await expect(page.getByTestId("pdf-page-indicator")).toHaveAttribute("data-page", "25", {
      timeout: 30_000,
    });
    await expect(
      page.locator("[data-testid='pdf-page-sheet'][data-page='25'] canvas"),
    ).toBeVisible({ timeout: 30_000 });
    await page.waitForTimeout(1000);
    expect(stats.bytesServed).toBeLessThan(stats.totalBytes * 0.55);
  });

  test("renders when the server ignores Range (200 fallback)", async ({ page }) => {
    const stats = await openDetail(page, { supportRange: false });
    await expect(page.locator("canvas").first()).toBeVisible({ timeout: 30_000 });
    expect(stats.rangeRequests).toBe(0);
  });

  test("PDF and Markdown share one toolbar row; view modes stay reachable", async ({ page }) => {
    await openDetail(page);
    const mdHeader = page.getByTestId("markdown-pane-header");
    const pdfToolbar = page.getByTestId("pdf-prev-page").locator("xpath=ancestor::div[contains(@class,'h-12')][1]");
    await expect(mdHeader).toBeVisible();
    await expect(pdfToolbar).toBeVisible();
    const [a, b] = await Promise.all([mdHeader.boundingBox(), pdfToolbar.boundingBox()]);
    expect(Math.abs(a!.y - b!.y), "headers share the same top edge").toBeLessThanOrEqual(1);
    expect(Math.abs(a!.height - b!.height), "headers share the same height").toBeLessThanOrEqual(1);

    // PDF only: Markdown header disappears, a floating switcher keeps the way back.
    await mdHeader.getByRole("button", { name: "PDF Only" }).click();
    await expect(mdHeader).toBeHidden();
    const floating = page.getByTestId("viewer-mode-toggle");
    await expect(floating).toBeVisible();
    await floating.getByRole("button", { name: "Split View" }).click();
    await expect(mdHeader).toBeVisible();
  });
});
