/**
 * Hermetic guard for the SPEC-155 mocked Playwright project.
 *
 * WHY: the mock project proxies `/api` and `/ws` to the dev backend. Any call
 * the mock does not cover used to leak through to the live stack, which (a)
 * made tests depend on whatever the backend returned, and (b) spammed the
 * backend log with 400 "Missing X-Tenant-ID" and WebSocket connect/disconnect
 * storms. This guard makes a mocked test self-contained: unknown API calls get
 * a deterministic 404 (and are recorded), and the pipeline WebSocket is
 * answered locally and kept open so the client never enters a reconnect loop.
 *
 * Install FIRST: Playwright runs the most recently registered matching route
 * first, so specific mocks registered afterwards still win over the catch-all.
 */

import type { Page, Route } from "@playwright/test";

/** Same-origin blank document used to seed storage without booting the app. */
export const SEED_PATH = "/__spec155_seed__";

const json = (body: unknown, status = 200) => ({
  status,
  contentType: "application/json",
  body: JSON.stringify(body),
});

function recordAndReject(unmocked: string[] | undefined) {
  return (route: Route) => {
    const request = route.request();
    unmocked?.push(`${request.method()} ${new URL(request.url()).pathname}`);
    return route.fulfill(
      json({ error: { code: "spec155_unmocked", message: "No mock registered" } }, 404),
    );
  };
}

export async function installHermeticGuard(
  page: Page,
  unmocked?: string[],
): Promise<void> {
  await page.route("**/api/v1/**", recordAndReject(unmocked));

  // Tenant-scoped endpoints the app touches on every Query page load.
  await page.route("**/api/v1/folders**", (route) => route.fulfill(json([])));

  // Local WebSocket: accept, never push, never close (no reconnect storm).
  await page.routeWebSocket(/\/ws\//, () => {});

  await page.route(`**${SEED_PATH}`, (route) =>
    route.fulfill({
      status: 200,
      contentType: "text/html",
      body: "<!doctype html><title>seed</title>",
    }),
  );
}
