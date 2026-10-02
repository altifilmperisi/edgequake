/**
 * SPEC-155 — a "processing" document the server stopped talking about is
 * shown honestly (Stalled, not a live 1% Active run) and can always be
 * cancelled — even with no track_id (hermetic).
 */
import { expect, test, type Page, type Route } from "@playwright/test";
import { prepareSpec155Page } from "./helpers/mock-api";

const STALLED_ID = "aaaaaaaa-1111-4111-8111-aaaaaaaaaaaa";
const DONE_ID = "bbbbbbbb-2222-4222-8222-bbbbbbbbbbbb";
const DAY_MS = 24 * 3_600_000;

type Doc = Record<string, unknown>;

function seedDocs(): Doc[] {
  const stale = new Date(Date.now() - 3 * DAY_MS).toISOString();
  return [
    {
      id: STALLED_ID,
      title: "spec129-dual",
      file_name: "spec129-dual.md",
      status: "processing",
      current_stage: "preprocessing",
      stage_progress: 0.01,
      source_type: "markdown",
      created_at: stale,
      updated_at: stale,
    },
    {
      id: DONE_ID,
      title: "finished-report",
      file_name: "finished-report.md",
      status: "completed",
      source_type: "markdown",
      entity_count: 4,
      created_at: stale,
      updated_at: stale,
    },
  ];
}

const json = (body: unknown, status = 200) => ({
  status,
  contentType: "application/json",
  body: JSON.stringify(body),
});

/** Stateful documents API: cancel-by-document flips the row to cancelled. */
async function openDocuments(page: Page) {
  const docs = seedDocs();
  const cancelCalls: string[] = [];
  await prepareSpec155Page(page, { documents: docs });

  await page.route(/\/api\/v1\/documents\/([^/?]+)\/cancel$/, async (route: Route) => {
    const id = route.request().url().match(/documents\/([^/?]+)\/cancel/)![1];
    cancelCalls.push(id);
    const doc = docs.find((d) => d.id === id);
    if (!doc) return route.fulfill(json({ error: "not found" }, 404));
    Object.assign(doc, {
      status: "cancelled",
      current_stage: "cancelled",
      updated_at: new Date().toISOString(),
    });
    return route.fulfill(
      json({ document_id: id, status: "cancelled", track_id: null, task_cancelled: false }),
    );
  });
  // Any task-keyed cancel would be the OLD (broken) path for an orphan row.
  await page.route(/\/api\/v1\/tasks\/[^/]+\/cancel$/, (route) =>
    route.fulfill(json({ error: "unexpected task cancel" }, 500)),
  );

  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto("/documents", { waitUntil: "domcontentloaded" });
  const rail = page.getByTestId("workspace-zone-rail-runs");
  if (await rail.isVisible().catch(() => false)) {
    await rail.click();
  }
  return { docs, cancelCalls };
}

test.describe("SPEC-155 stalled document — honest state + Cancel @spec155", () => {
  test("shows Stalled (not a live run) with Cancel + Reprocess", async ({ page }) => {
    await openDocuments(page);
    const card = page.getByTestId("spec155-stalled-run-card");
    await expect(card).toBeVisible({ timeout: 30_000 });
    await expect(page.getByTestId("spec155-stalled-headline")).toHaveText(
      /Stalled · no progress for 3 days/,
    );
    await expect(card).toContainText("Last step:");
    await expect(card.getByTestId("spec086-run-cancel")).toBeVisible();
    await expect(card.getByTestId("spec155-stalled-reprocess")).toBeVisible();

    // No fake live progress, no "Active run".
    await expect(page.getByTestId("spec048-active-runs-working")).toHaveCount(0);
    await expect(page.getByTestId("spec048-run-overall-pct")).toHaveCount(0);
    await expect(page.getByTestId("spec155-attention-hint")).toContainText(
      "stopped reporting progress",
    );
  });

  test("table row and banner agree: Stalled, no duplicate red banner", async ({ page }) => {
    await openDocuments(page);
    await expect(page.getByTestId("spec155-stalled-run-card")).toBeVisible({ timeout: 30_000 });

    const row = page.getByTestId(`document-row-${STALLED_ID}`);
    await expect(row.getByTestId("status-cell")).toHaveAttribute("data-stalled", "true");
    await expect(row.getByTestId("status-badge")).toHaveText("Stalled");
    // The card owns Cancel/Reprocess; a second red banner would repeat it.
    await expect(page.getByTestId("ingestion-status-banner")).toHaveCount(0);
  });

  test("Cancel works for a row with no track_id (document-level cancel)", async ({ page }) => {
    const { cancelCalls } = await openDocuments(page);
    await page.getByTestId("spec155-stalled-run-card").getByTestId("spec086-run-cancel").click();

    await expect.poll(() => cancelCalls).toEqual([STALLED_ID]);
    await expect(page.getByTestId("spec155-stalled-run-card")).toHaveCount(0);
    await expect(page.getByTestId("spec048-active-runs-working")).toContainText("Cancelled");
  });

  test("preview panel offers Cancel for the stalled row", async ({ page }) => {
    const { cancelCalls } = await openDocuments(page);
    await expect(page.getByTestId("spec155-stalled-run-card")).toBeVisible({ timeout: 30_000 });

    await page
      .getByRole("row")
      .filter({ hasText: "spec129-dual" })
      .getByText("spec129-dual", { exact: true })
      .first()
      .click();
    await expect(page.getByTestId("spec155-preview-stalled-notice")).toBeVisible();
    const cancel = page.getByTestId("spec155-preview-cancel");
    await expect(cancel).toContainText("Cancel stalled run");
    await cancel.click();
    await expect.poll(() => cancelCalls).toEqual([STALLED_ID]);
    await expect(page.getByTestId("spec155-preview-cancel")).toHaveCount(0);
  });

  test("selection bar offers Cancel only when the selection is in flight", async ({ page }) => {
    await openDocuments(page);
    await expect(page.getByTestId("spec155-stalled-run-card")).toBeVisible({ timeout: 30_000 });

    const rows = page.getByRole("row");
    await rows.filter({ hasText: "finished-report" }).getByRole("checkbox").check();
    await expect(page.getByTestId("batch-actions-bar")).toBeVisible();
    await expect(page.getByTestId("spec155-bulk-cancel")).toHaveCount(0);

    await rows.filter({ hasText: "spec129-dual" }).getByRole("checkbox").check();
    await expect(page.getByTestId("batch-actions-bar")).toContainText("2 selected");
    await expect(page.getByTestId("spec155-bulk-cancel")).toBeVisible();
  });
});
