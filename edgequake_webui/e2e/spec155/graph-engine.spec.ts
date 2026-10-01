/**
 * SPEC-155 W4 GraphEngine e2e (@spec155)
 * - filter toggles must not change data-graph-engine-id
 * - truncation banner truthful with FIXTURE_500
 */
import { expect, test } from "@playwright/test";
import { FIXTURE_500 } from "../../src/lib/fixtures/graph";
import { prepareSpec155Page } from "./helpers/mock-api";

test.describe("SPEC-155 GraphEngine @spec155", () => {
  test("graph_filter_no_rebuild keeps data-graph-engine-id stable", async ({
    page,
  }) => {
    await prepareSpec155Page(page, { graph: FIXTURE_500 });
    await page.goto("/graph?stream=0", { waitUntil: "domcontentloaded" });

    const engine = page.locator("[data-graph-engine-id]").first();
    await expect(engine).toBeVisible({ timeout: 60_000 });
    const engineId = await engine.getAttribute("data-graph-engine-id");
    expect(engineId).toBeTruthy();

    // Toggle an entity type filter if present
    const typeToggle = page
      .locator('[data-testid="spec100-graph-filters"] button, [data-testid="spec100-graph-filters"] [role="checkbox"]')
      .first();
    if (await typeToggle.count()) {
      await typeToggle.click();
      await page.waitForTimeout(300);
    }

    // Search filter (debounced) — must not remount engine
    const search = page.getByTestId("graph-filter-search");
    if (await search.count()) {
      await search.fill("Node 1");
      await page.waitForTimeout(400);
    }

    const afterId = await page
      .locator("[data-graph-engine-id]")
      .first()
      .getAttribute("data-graph-engine-id");
    expect(afterId).toBe(engineId);
  });

  test("truncation banner truthful with FIXTURE_500", async ({ page }) => {
    await prepareSpec155Page(page, { graph: FIXTURE_500 });
    await page.goto("/graph?stream=0", { waitUntil: "domcontentloaded" });

    // Banner / indicator should report loaded vs total (200 of 500)
    await expect
      .poll(
        async () => {
          const text = await page.locator("body").innerText();
          return text;
        },
        { timeout: 60_000 },
      )
      .toMatch(/200[\s/]+.*500|200\s+of\s+500/i);

    // Explicit totals from fixture
    expect(FIXTURE_500.is_truncated).toBe(true);
    expect(FIXTURE_500.total_nodes).toBe(500);
    expect(FIXTURE_500.nodes.length).toBe(200);
  });
});
