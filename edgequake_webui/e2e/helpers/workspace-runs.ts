/**
 * SPEC-155 Documents workspace — expand Runs / Working before ActiveRuns asserts.
 * Product default: Runs may be railed; Working is collapsed (density-first).
 */
import { expect, type Page } from "@playwright/test";

/** Expand the Runs zone when it is railed. */
export async function ensureRunsExpanded(page: Page) {
  const zone = page.getByTestId("workspace-zone-runs");
  if (await zone.isVisible().catch(() => false)) return;
  const rail = page.getByTestId("workspace-zone-rail-runs");
  if ((await rail.count()) > 0) {
    await rail.first().click({ force: true });
  }
  await expect(zone).toBeVisible({ timeout: 15_000 });
}

/** Expand Runs + Working so Active run cards / headlines / steppers are reachable. */
export async function expandIntakeWorking(page: Page) {
  await ensureRunsExpanded(page);
  const toggle = page.getByTestId("documents-intake-toggle");
  if ((await toggle.count()) > 0) {
    if ((await toggle.getAttribute("aria-expanded")) !== "true") {
      await toggle.click();
    }
  }
}

/** Prefer fresh timestamps so SPEC-155 stall detection does not paint Working as Stalled. */
export function freshIso(offsetMs = 0): string {
  return new Date(Date.now() + offsetMs).toISOString();
}
