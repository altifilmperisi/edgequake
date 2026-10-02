import { expect, test } from "@playwright/test";
import { prepareSpec155Page } from "./helpers/mock-api";

test.describe("SPEC-155 upload fills zone @spec155", () => {
  test("dropzone stretches and adapts layout to panel size", async ({ page }) => {
    await prepareSpec155Page(page, { emptyDocs: true });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/documents", { waitUntil: "domcontentloaded" });
    const zone = page.getByTestId("workspace-zone-intake");
    const slot = page.getByTestId("documents-intake-dropzone-slot");
    const dz = page.getByTestId("document-dropzone");
    await expect(dz).toHaveAttribute("data-fill", "true", { timeout: 30_000 });
    await expect(zone).toBeVisible();
    await expect(dz).toBeVisible();

    const zb = await zone.boundingBox();
    const sb = await slot.boundingBox();
    const db = await dz.boundingBox();
    expect(zb, "zone box").toBeTruthy();
    expect(sb, "slot box").toBeTruthy();
    expect(db, "dropzone box").toBeTruthy();
    expect(Math.abs(db!.width - sb!.width)).toBeLessThanOrEqual(2);
    expect(Math.abs(db!.height - sb!.height)).toBeLessThanOrEqual(2);

    const layout = await dz.getAttribute("data-fill-layout");
    expect(["row", "stack", "hero"]).toContain(layout);
    // Classic tools band is short → row; roomy → hero.
    if (db!.height < 120) {
      expect(layout).toBe("row");
    }

    // Zone respects intake min height when expanded.
    expect(zb!.height).toBeGreaterThanOrEqual(100);

    await page.screenshot({
      path: "test-results/spec155-upload-fill.png",
      fullPage: false,
    });
  });
});
