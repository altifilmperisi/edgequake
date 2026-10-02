/**
 * SPEC-099 — Documents scroll layout: chrome (dropzone) stays pinned;
 * inventory scrolls internally; no page-level white-band spacer blowup.
 */
import { expect, test } from "@playwright/test";
import { GOTO_OPTS } from "./helpers/app-ready";
import {
  mockSpec038AdmissionRoutes,
  seedSpec038TenantContext,
} from "./helpers/spec038-admission-mocks";
import {
  makeSpec086ListDoc,
  mockSpec086DocumentList,
  type Spec086ListDoc,
} from "./helpers/spec086-ingestion-mocks";

test.describe("SPEC-099 documents scroll layout", () => {
  test("dropzone stays in viewport after scrolling the inventory table", async ({
    page,
  }) => {
    await page.setViewportSize({ width: 1280, height: 800 });
    const docs: Spec086ListDoc[] = Array.from({ length: 24 }, (_, i) =>
      makeSpec086ListDoc({
        id: `doc-099-scroll-${i}`,
        file_name: `scroll-${String(i).padStart(2, "0")}.pdf`,
        status: "completed",
        current_stage: "completed",
        track_id: null,
        admission_staging: false,
        query_ready: true,
      }),
    );
    await mockSpec038AdmissionRoutes(page);
    await seedSpec038TenantContext(page);
    await mockSpec086DocumentList(page, docs);
    await page.goto("/documents", GOTO_OPTS);

    const dropzone = page.getByTestId("document-dropzone");
    const chrome = page.getByTestId("documents-chrome");
    const workspace = page.getByTestId("documents-workspace");
    const scroll = page.getByTestId("documents-table-scroll");

    await expect(dropzone).toBeVisible({ timeout: 20_000 });
    await expect(chrome).toBeVisible();
    await expect(workspace).toBeVisible();
    await expect(scroll).toBeVisible();
    await expect(page.getByTestId("documents-virtual-spacer")).toBeVisible();

    // Dropzone must stay in the Intake zone (not scrolled away with inventory)
    const dropBefore = await dropzone.boundingBox();
    expect(dropBefore).toBeTruthy();
    expect(dropBefore!.y).toBeGreaterThanOrEqual(0);
    expect(dropBefore!.y).toBeLessThan(500);
    expect(dropBefore!.width).toBeGreaterThan(120);

    // Document must not grow a page scroller (table padding leak → white band)
    const beforeScroll = await page.evaluate(() => ({
      docScrollH: document.documentElement.scrollHeight,
      innerH: window.innerHeight,
      htmlOY: getComputedStyle(document.documentElement).overflowY,
      bodyOY: getComputedStyle(document.body).overflowY,
    }));
    expect(beforeScroll.htmlOY === "hidden" || beforeScroll.htmlOY === "clip").toBe(
      true,
    );
    expect(beforeScroll.bodyOY === "hidden" || beforeScroll.bodyOY === "clip").toBe(
      true,
    );
    expect(beforeScroll.docScrollH).toBeLessThanOrEqual(beforeScroll.innerH + 2);

    // Scroll the inventory container — not the window
    await scroll.evaluate((el) => {
      el.scrollTop = el.scrollHeight;
    });

    await expect(dropzone).toBeVisible();
    const dropAfter = await dropzone.boundingBox();
    expect(dropAfter).toBeTruthy();
    // Still in the upper viewport (pinned chrome)
    expect(dropAfter!.y).toBeGreaterThanOrEqual(0);
    expect(dropAfter!.y).toBeLessThan(500);

    // Window must reject page scroll
    await page.evaluate(() => window.scrollTo(0, 10_000));
    const windowScrollY = await page.evaluate(() => window.scrollY);
    expect(windowScrollY).toBe(0);

    // Inventory section fills remaining height (no unbounded spacer growth)
    const inventory = page.getByTestId("documents-inventory-section");
    const invBox = await inventory.boundingBox();
    const shell = page.getByTestId("documents-page-shell");
    const shellBox = await shell.boundingBox();
    expect(invBox).toBeTruthy();
    expect(shellBox).toBeTruthy();
    expect(invBox!.height).toBeGreaterThan(shellBox!.height * 0.35);

    // Scrollport itself must be a real viewport
    const scrollMetrics = await scroll.evaluate((el) => ({
      clientHeight: el.clientHeight,
      scrollHeight: el.scrollHeight,
      scrollTop: el.scrollTop,
    }));
    expect(scrollMetrics.clientHeight).toBeGreaterThan(120);
    expect(scrollMetrics.scrollHeight).toBeGreaterThan(scrollMetrics.clientHeight);
    expect(scrollMetrics.scrollTop).toBeGreaterThan(0);
  });
});
