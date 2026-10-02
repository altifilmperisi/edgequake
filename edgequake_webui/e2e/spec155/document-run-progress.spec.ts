/**
 * SPEC-155 — a live run shows ONE progress bar (the segmented phase strip) with
 * the percentage and headline on a single caption line, and a real Cancel
 * button. The table row repeats only the status badge, never a second bar.
 *
 * Ledger (SPEC-155 run_progress): Prepare never collapses when figures start;
 * Extract fill tracks completed chunks; reload paints identical bars.
 */
import { expect, test, type Page } from "@playwright/test";
import { prepareSpec155Page } from "./helpers/mock-api";

const RUN_ID = "cccccccc-3333-4333-8333-cccccccccccc";

function extractLedger(done = 1, total = 107, inFlight = 4) {
  return {
    seq: 10,
    phases: [
      {
        id: "prepare",
        state: "done",
        tasks: [
          { id: "pages", unit: "pages", done: 92, total: 92 },
          { id: "figures", unit: "figures", done: 12, total: 12 },
        ],
      },
      {
        id: "extract",
        state: "active",
        tasks: [
          {
            id: "chunks",
            unit: "chunks",
            done,
            total,
            in_flight: inFlight,
          },
        ],
      },
      { id: "materialize", state: "pending", tasks: [] },
    ],
  };
}

function prepareFiguresLedger() {
  return {
    seq: 5,
    phases: [
      {
        id: "prepare",
        state: "active",
        tasks: [
          { id: "pages", unit: "pages", done: 92, total: 92 },
          { id: "figures", unit: "figures", done: 1, total: 12 },
        ],
      },
      { id: "extract", state: "pending", tasks: [] },
      { id: "materialize", state: "pending", tasks: [] },
    ],
  };
}

function liveDocs(): Record<string, unknown>[] {
  const now = new Date().toISOString();
  return [
    {
      id: RUN_ID,
      title: "001_2608.06377v1.pdf",
      file_name: "001_2608.06377v1.pdf",
      status: "processing",
      current_stage: "extracting",
      stage_message: "Extracting entities — 1/107 chunks, 4 in flight",
      stage_progress: 0.01,
      progress_counts: { unit: "chunks", current: 1, total: 107 },
      run_progress: extractLedger(1, 107, 4),
      source_type: "pdf",
      track_id: "track-155-live",
      created_at: now,
      updated_at: now,
    },
  ];
}

/** Expand the Runs zone (if railed) and the Working section when collapsed. */
async function expandIntakeWorking(page: Page) {
  const rail = page.getByTestId("workspace-zone-rail-runs");
  if (await rail.isVisible().catch(() => false)) {
    await rail.click();
  }
  await expect(page.getByTestId("workspace-zone-runs")).toBeVisible({
    timeout: 15_000,
  });
  const toggle = page.getByTestId("documents-intake-toggle");
  await expect(toggle).toBeVisible({ timeout: 30_000 });
  if ((await toggle.getAttribute("aria-expanded")) !== "true") {
    await toggle.click();
  }
  await expect(toggle).toHaveAttribute("aria-expanded", "true");
}

async function openLive(page: Page) {
  await prepareSpec155Page(page, { documents: liveDocs() });
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto("/documents", { waitUntil: "domcontentloaded" });
  await expandIntakeWorking(page);
  const card = page.getByTestId("spec048-active-run-card");
  await expect(card).toBeVisible({ timeout: 30_000 });
  return card;
}

test.describe("SPEC-155 single progress bar @spec155", () => {
  test("segmented strip is the bar; % and headline share one caption", async ({ page }) => {
    const card = await openLive(page);

    // Exactly one progressbar in the card: the active segment.
    await expect(card.getByRole("progressbar")).toHaveCount(1);
    const strip = card.getByTestId("spec091-phase-strip");
    await expect(strip).toHaveAttribute("data-variant", "segmented");
    await expect(card.getByTestId("spec091-phase-extract")).toHaveAttribute(
      "data-state",
      "active",
    );

    // Caption: headline + percentage together; no full-width second meter.
    const caption = card.getByTestId("spec048-stage-progress");
    await expect(caption).toContainText("1/107");
    await expect(caption.getByTestId("spec048-run-stage-pct")).toHaveText("1%");
    await expect(card.getByTestId("spec048-overall-progress").first()).toHaveAttribute(
      "data-collapsed",
      "true",
    );

    await card.screenshot({ path: "test-results/spec155-run-progress-card.png" });
  });

  test("no N/M yet: indeterminate segment + estimated overall, no fake number", async ({
    page,
  }) => {
    const docs = liveDocs();
    delete docs[0].progress_counts;
    delete docs[0].run_progress;
    docs[0].current_stage = "preprocessing";
    docs[0].stage_message = "Preparing document";
    docs[0].stage_progress = undefined;
    await prepareSpec155Page(page, { documents: docs });
    await page.goto("/documents", { waitUntil: "domcontentloaded" });
    await expandIntakeWorking(page);
    const card = page.getByTestId("spec048-active-run-card");
    await expect(card).toBeVisible({ timeout: 30_000 });

    await expect(card.getByTestId("spec048-run-progress-indeterminate")).toHaveCount(1);
    await expect(card.getByRole("progressbar")).toHaveCount(0);
    await expect(card.getByTestId("spec048-run-overall-pct")).toHaveText(/^~\d+%$/);
    await expect(card.getByTestId("spec048-overall-progress")).toHaveAttribute(
      "data-collapsed",
      "false",
    );
  });

  test("Cancel is a real button with an icon", async ({ page }) => {
    const card = await openLive(page);
    const cancel = card.getByRole("button", { name: "Cancel" });
    await expect(cancel).toBeVisible();
    const box = await cancel.boundingBox();
    expect(box?.height ?? 0).toBeGreaterThanOrEqual(24);
  });

  test("table row shows only the status badge (no second bar)", async ({ page }) => {
    await openLive(page);
    const row = page.getByTestId(`document-row-${RUN_ID}`);
    await expect(row.getByTestId("status-badge")).toBeVisible();
    await expect(row.getByRole("progressbar")).toHaveCount(0);
  });

  test("Prepare fill stays high when figures start (ledger)", async ({ page }) => {
    const now = new Date().toISOString();
    await prepareSpec155Page(page, {
      documents: [
        {
          id: RUN_ID,
          title: "figures.pdf",
          file_name: "figures.pdf",
          status: "processing",
          current_stage: "converting",
          stage_message: "Preparing — pages 92/92, figures 1/12",
          // Legacy figure-only pct that used to collapse Prepare to ~8%.
          stage_progress: 0.08,
          progress_counts: { unit: "figures", current: 1, total: 12 },
          run_progress: prepareFiguresLedger(),
          source_type: "pdf",
          track_id: "track-155-figures",
          created_at: now,
          updated_at: now,
        },
      ],
    });
    await page.goto("/documents", { waitUntil: "domcontentloaded" });
    await expandIntakeWorking(page);
    const card = page.getByTestId("spec048-active-run-card");
    await expect(card).toBeVisible({ timeout: 30_000 });
    await expect(card.getByTestId("spec091-phase-prepare")).toHaveAttribute(
      "data-state",
      "active",
    );
    await expect(card.getByTestId("spec048-stage-progress")).toContainText("figures");
    // Prepare segment fill must stay well above the legacy 8%.
    const bar = card.getByRole("progressbar");
    await expect(bar).toBeVisible();
    const value = Number(await bar.getAttribute("aria-valuenow"));
    expect(value).toBeGreaterThan(40);
  });

  test("reload paints identical segment fills and caption", async ({ page }) => {
    const card = await openLive(page);
    const fillBefore = await card.getByRole("progressbar").getAttribute("aria-valuenow");
    const prepareState = await card
      .getByTestId("spec091-phase-prepare")
      .getAttribute("data-state");
    const caption = card.getByTestId("spec048-stage-progress");
    await expect(caption).toContainText("1/107");
    await expect(caption.getByTestId("spec048-run-stage-pct")).toHaveText("1%");

    // Re-seed + navigate (bare reload drops hermetic mock boot order).
    await prepareSpec155Page(page, { documents: liveDocs() });
    await page.goto("/documents", { waitUntil: "domcontentloaded" });
    await expandIntakeWorking(page);
    const card2 = page.getByTestId("spec048-active-run-card");
    await expect(card2).toBeVisible({ timeout: 30_000 });
    const caption2 = card2.getByTestId("spec048-stage-progress");
    await expect(caption2).toContainText("1/107");
    await expect(caption2.getByTestId("spec048-run-stage-pct")).toHaveText("1%");
    await expect(card2.getByRole("progressbar")).toHaveAttribute(
      "aria-valuenow",
      fillBefore ?? "",
    );
    await expect(card2.getByTestId("spec091-phase-prepare")).toHaveAttribute(
      "data-state",
      prepareState ?? "done",
    );
  });
});
