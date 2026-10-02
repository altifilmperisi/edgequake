/**
 * SPEC-155 — Layout presets via workspace menu (replaces Focus/Split/Stack strip).
 * Full docking coverage: documents-workspace.spec.ts.
 */
import { expect, test, type Page } from "@playwright/test";
import { prepareSpec155Page } from "./helpers/mock-api";

const RUN_A = "cccccccc-3333-4333-8333-cccccccccccc";

function livePlusIdle(): Record<string, unknown>[] {
  const now = new Date().toISOString();
  return [
    {
      id: RUN_A,
      title: "algo_2608.pdf",
      file_name: "algo_2608.pdf",
      status: "processing",
      current_stage: "converting",
      stage_progress: 0.04,
      source_type: "pdf",
      track_id: "track-layout-modes",
      created_at: now,
      updated_at: now,
    },
    ...Array.from({ length: 4 }, (_, i) => ({
      id: `f0000000-0000-4000-8000-00000000000${i}`,
      title: `idle-${i}.pdf`,
      file_name: `idle-${i}.pdf`,
      status: "completed",
      current_stage: "completed",
      source_type: "pdf",
      entity_count: 12,
      created_at: now,
      updated_at: now,
    })),
  ];
}

async function open(page: Page) {
  await prepareSpec155Page(page, { documents: livePlusIdle() });
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.goto("/documents", { waitUntil: "domcontentloaded" });
  await expect(page.getByTestId("documents-workspace")).toBeVisible({
    timeout: 30_000,
  });
}

test.describe("SPEC-155 documents layout modes @spec155", () => {
  test("classic default; library-center preset via menu", async ({ page }) => {
    await open(page);
    await expect(page.getByTestId("documents-page-shell")).toHaveAttribute(
      "data-layout-preset",
      "classic",
    );
    await page.getByTestId("workspace-layout-menu").click();
    await page.getByTestId("workspace-layout-preset-library-center").click();
    await expect(page.getByTestId("documents-page-shell")).toHaveAttribute(
      "data-layout-preset",
      "library-center",
    );
    const intake = await page.getByTestId("workspace-zone-intake").boundingBox();
    const library = await page.getByTestId("workspace-zone-library").boundingBox();
    expect(intake && library).toBeTruthy();
    expect(intake!.x).toBeLessThan(library!.x);
  });

  test("Alt+1..4 cycle presets", async ({ page }) => {
    await open(page);
    await page.keyboard.press("Alt+1");
    await expect(page.getByTestId("documents-page-shell")).toHaveAttribute(
      "data-layout-preset",
      "classic",
    );
    await page.keyboard.press("Alt+3");
    await expect(page.getByTestId("documents-page-shell")).toHaveAttribute(
      "data-layout-preset",
      "library-right",
    );
    await page.keyboard.press("Alt+4");
    await expect(page.getByTestId("documents-page-shell")).toHaveAttribute(
      "data-layout-preset",
      "library-center",
    );
  });
});
