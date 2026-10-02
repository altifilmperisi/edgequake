/**
 * SPEC-155 — Dashboard: honest recent activity, actionable stats, compact
 * actions, and a subtitle that adds information instead of repeating the cards.
 */
import { expect, test, type Page } from "@playwright/test";
import { prepareSpec155Page } from "./helpers/mock-api";

const ago = (m: number) => new Date(Date.now() - m * 60_000).toISOString();

const DOCS = [
  { id: "11111111-1111-4111-8111-111111111111", title: "tmeta_2608.38147v1.pdf", file_name: "tmeta_2608.38147v1.pdf", status: "completed", source_type: "pdf", created_at: ago(20), updated_at: ago(18) },
  { id: "22222222-2222-4222-8222-222222222222", title: "Enterprise-Agent-Chassis-Guide.pdf", file_name: "Enterprise-Agent-Chassis-Guide.pdf", status: "processing", current_stage: "extracting", stage_message: "Extracting Entities", stage_progress: 0.42, source_type: "pdf", track_id: "t1", created_at: ago(40), updated_at: new Date().toISOString() },
  { id: "33333333-3333-4333-8333-333333333333", title: "IA_et_infections__1_.pdf", file_name: "IA_et_infections__1_.pdf", status: "failed", error_message: "Knowledge graph persist failed: Storage error: Conflict: cannot ingest into a tombstoned document", source_type: "pdf", created_at: ago(60), updated_at: ago(55) },
  { id: "44444444-4444-4444-8444-444444444444", title: "memfold_2609_36435v1.md", file_name: "memfold_2609_36435v1.md", status: "delete_failed", source_type: "markdown", created_at: ago(300), updated_at: ago(200) },
  { id: "55555555-5555-4555-8555-555555555555", title: "a-very-long-document-name-that-keeps-going-and-going-for-truncation-checks-v2.pdf", file_name: "x.pdf", status: "pending", source_type: "pdf", created_at: ago(500), updated_at: ago(500) },
];

/** Honest workspace counts (the shared mock sends an empty object). */
async function withStatusCounts(page: Page, docs: typeof DOCS) {
  const count = (s: string) => docs.filter((d) => d.status === s).length;
  await page.route("**/api/v1/documents?**", async (route) => {
    if (route.request().method() !== "GET") return route.fallback();
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        items: docs, documents: docs, total: docs.length, page: 1, page_size: 10, has_more: false,
        status_counts: {
          pending: count("pending"), processing: count("processing"), completed: count("completed"),
          failed: count("failed"), partial_failure: 0, cancelled: 0,
        },
      }),
    });
  });
}

async function open(page: Page, docs = DOCS, size = { width: 1440, height: 900 }) {
  await prepareSpec155Page(page, { documents: docs });
  await withStatusCounts(page, docs);
  await page.setViewportSize(size);
  await page.goto("/", { waitUntil: "domcontentloaded" });
  await expect(page.getByTestId("stats-card").first()).toBeVisible({ timeout: 30_000 });
}

test.describe("SPEC-155 dashboard polish @spec155", () => {
  test("recent activity tells the truth about every state", async ({ page }) => {
    await open(page);
    const rows = page.getByTestId("dashboard-activity-row");
    await expect(rows).toHaveCount(5);

    // The half-deleted document is NOT "Completed" (old fallback bug).
    const del = rows.filter({ hasText: "memfold_2609_36435v1.md" });
    await expect(del).toHaveAttribute("data-status", "delete_failed");
    await expect(del.getByTestId("status-badge")).toContainText("Delete failed");

    // Exactly one live bar, with the server's real fraction.
    const bar = page.getByRole("progressbar");
    await expect(bar).toHaveCount(1);
    await expect(bar).toHaveAttribute("aria-valuenow", "42");

    // Failure reason is short and actionable, not a raw storage string.
    const failed = rows.filter({ hasText: "IA_et_infections__1_.pdf" });
    await expect(failed.getByTestId("dashboard-activity-note")).toContainText(
      "Deleted before it finished",
    );
    await expect(failed.getByTestId("dashboard-activity-note")).not.toContainText("Storage error");

    await expect(page.getByTestId("dashboard-activity-view-all")).toContainText("View all 5 documents");
  });

  test("subtitle adds information (no repeat of the stat cards)", async ({ page }) => {
    await open(page);
    const sub = page.getByTestId("spec100-dashboard-subtitle");
    await expect(sub).toContainText("1 processing");
    await expect(sub).toContainText("1 queued");
    await expect(sub).toContainText("1 failed");
    await expect(sub).not.toContainText("entities");
    await expect(sub).not.toContainText("relationships");
  });

  test("stat cards are links; actions are compact; Upload is not forced", async ({ page }) => {
    await open(page);
    await expect(page.getByRole("link", { name: /^Documents: 5$/ })).toHaveAttribute("href", "/documents");
    await expect(page.getByRole("link", { name: /^Entities: / })).toHaveAttribute("href", "/graph");
    await expect(page.getByTestId("dashboard-quick-actions").getByRole("link")).toHaveCount(3);
    await expect(page.getByTestId("dashboard-action-upload")).not.toHaveAttribute("data-emphasis", "primary");
    await page.screenshot({ path: "test-results/dash/after-light-1440.png", fullPage: true });
  });

  test("empty workspace: Upload becomes the one obvious next step", async ({ page }) => {
    await open(page, []);
    await expect(page.getByTestId("dashboard-action-upload")).toHaveAttribute("data-emphasis", "primary");
    await expect(page.getByTestId("spec100-dashboard-subtitle")).toContainText("Upload your first document");
    await expect(page.getByTestId("dashboard-activity-view-all")).toHaveCount(0);
  });

  test("no layout shift: activity card keeps its height from skeleton to rows", async ({ page }) => {
    await prepareSpec155Page(page, { documents: DOCS });
    await withStatusCounts(page, DOCS);
    // Slow documents so the skeleton is observable.
    await page.route("**/api/v1/documents?**", async (route) => {
      await new Promise((r) => setTimeout(r, 600));
      await route.fallback();
    });
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto("/", { waitUntil: "domcontentloaded" });
    const card = page.getByTestId("spec100-dashboard-activity");
    await expect(card).toBeVisible({ timeout: 30_000 });
    await expect.poll(async () => (await card.boundingBox())?.height ?? 0).toBeGreaterThanOrEqual(280);
    const during = (await card.boundingBox())?.height ?? 0;
    await expect(page.getByTestId("dashboard-activity-row").first()).toBeVisible({ timeout: 15_000 });
    const after = (await card.boundingBox())?.height ?? 0;
    // Skeleton is sized like real rows: the card must not visibly jump.
    expect(Math.abs(after - during)).toBeLessThanOrEqual(40);
  });

  for (const theme of ["light", "dark"] as const) {
    test(`mobile 390 ${theme}: no horizontal overflow, 2-up stats`, async ({ page }) => {
      await page.emulateMedia({ colorScheme: theme });
      await page.addInitScript((t) => localStorage.setItem("theme", t), theme);
      await open(page, DOCS, { width: 390, height: 844 });
      const overflow = await page.evaluate(
        () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
      );
      expect(overflow).toBeLessThanOrEqual(0);
      const cards = page.getByTestId("stats-card");
      const a = await cards.nth(0).boundingBox();
      const b = await cards.nth(1).boundingBox();
      expect(Math.abs((a?.y ?? 0) - (b?.y ?? 1))).toBeLessThan(2); // same row
      await page.waitForTimeout(500);
      await page.screenshot({ path: `test-results/dash/after-${theme}-390.png`, fullPage: true });
    });
  }
});
