/**
 * SPEC-155 Documents workspace — expand Runs / Intake before ActiveRuns asserts.
 * Product default: idle Runs auto-rails; Working is collapsed (density-first).
 *
 * Do not use locator.click() on zone rails — auto-rail re-renders detach the
 * button and Playwright retries until the test timeout (CI hang).
 *
 * Never click workspace-zone-toggle-* while the zone is open: that collapses
 * it. An earlier helper did that when width < 180 and left Intake railed.
 */
import { expect, type Page } from "@playwright/test";

async function clickByTestId(page: Page, testId: string) {
  const loc = page.getByTestId(testId);
  if ((await loc.count()) === 0) return;
  await loc
    .first()
    .evaluate((el) => (el as HTMLElement).click())
    .catch(() => undefined);
}

async function clickRail(page: Page, testId: string) {
  await clickByTestId(page, testId);
}

/** Restore Classic so Upload is a top-band panel, not a 16% library-center column. */
export async function applyClassicLayout(page: Page) {
  await clickByTestId(page, "workspace-layout-menu");
  await clickByTestId(page, "workspace-layout-preset-classic");
}

export async function waitForDocumentsWorkspace(page: Page) {
  await expect(page.getByTestId("documents-workspace")).toBeVisible({
    timeout: 30_000,
  });
}

/** True when the Runs panel is more than a 28px clipped rail. */
async function runsPanelUsable(page: Page): Promise<boolean> {
  const zone = page.getByTestId("workspace-zone-runs");
  const box = await zone.boundingBox().catch(() => null);
  if (box && box.width >= 80 && box.height >= 40) return true;
  const panel = page.getByTestId("spec048-active-runs-panel");
  const pBox = await panel.boundingBox().catch(() => null);
  if (pBox && pBox.width >= 80 && pBox.height >= 24) return true;
  const feedback = page.getByTestId("spec051-feedback-zone");
  const fBox = await feedback.boundingBox().catch(() => null);
  return Boolean(fBox && fBox.width >= 80 && fBox.height >= 24);
}

/** Expand the Runs zone when it is railed. Do not apply Classic here —
 * applyPreset clears the Runs user-override and idle auto-rail wins. */
export async function ensureRunsExpanded(page: Page) {
  await waitForDocumentsWorkspace(page);
  if (await runsPanelUsable(page)) return;

  const zone = page.getByTestId("workspace-zone-runs");
  const rail = page.getByTestId("workspace-zone-rail-runs");
  await expect(zone.or(rail)).toBeVisible({ timeout: 20_000 });

  await expect
    .poll(
      async () => {
        if (await runsPanelUsable(page)) return true;
        await clickRail(page, "workspace-zone-rail-runs");
        return false;
      },
      { timeout: 20_000 },
    )
    .toBe(true);
}

/** Expand Intake so the fill dropzone is a real panel, not a 28px rail. */
export async function ensureIntakeExpanded(page: Page) {
  await waitForDocumentsWorkspace(page);
  const zone = page.getByTestId("workspace-zone-intake");
  const rail = page.getByTestId("workspace-zone-rail-intake");
  const dropzone = page.getByTestId("document-dropzone");
  await expect(zone.or(rail)).toBeVisible({ timeout: 20_000 });

  let askedClassic = false;
  await expect
    .poll(
      async () => {
        const box = await dropzone.boundingBox().catch(() => null);
        if (box && box.width >= 180) return true;
        if (await rail.isVisible().catch(() => false)) {
          await clickRail(page, "workspace-zone-rail-intake");
          return false;
        }
        if (!askedClassic) {
          askedClassic = true;
          await applyClassicLayout(page);
        }
        return false;
      },
      { timeout: 20_000 },
    )
    .toBe(true);
}

/** Expand Runs + Working so Active run cards / headlines / steppers are reachable. */
export async function expandIntakeWorking(page: Page) {
  await ensureRunsExpanded(page);
  const toggle = page.getByTestId("documents-intake-toggle");
  if ((await toggle.count()) === 0) return;
  await expect
    .poll(
      async () => {
        if ((await toggle.getAttribute("aria-expanded")) === "true") {
          return true;
        }
        await toggle
          .evaluate((el) => (el as HTMLButtonElement).click())
          .catch(() => undefined);
        return (await toggle.getAttribute("aria-expanded")) === "true";
      },
      { timeout: 10_000 },
    )
    .toBe(true);
}

/** Prefer fresh timestamps so SPEC-155 stall detection does not paint Working as Stalled. */
export function freshIso(offsetMs = 0): string {
  return new Date(Date.now() + offsetMs).toISOString();
}
