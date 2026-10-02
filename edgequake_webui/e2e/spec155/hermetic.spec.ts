/**
 * SPEC-155 reliability gate (@spec155): a mocked page load must be hermetic.
 *
 * - no API call reaches the live backend un-mocked (would be 400/404 noise),
 * - no request is sent without the tenant header,
 * - the pipeline WebSocket is opened a bounded number of times (no reconnect storm).
 */
import { expect, test } from "@playwright/test";
import { prepareSpec155Page } from "./helpers/mock-api";

test.describe("SPEC-155 hermetic mocked load @spec155", () => {
  test("query page boots with tenant headers, no unmocked calls, bounded sockets", async ({
    page,
  }) => {
    const unmocked: string[] = [];
    const missingTenant: string[] = [];
    let sockets = 0;

    page.on("websocket", () => {
      sockets += 1;
    });
    page.on("request", (req) => {
      const { pathname } = new URL(req.url());
      const isTenantScoped = /\/api\/v1\/(folders|conversations)/.test(pathname);
      if (isTenantScoped && !req.headers()["x-tenant-id"]) {
        missingTenant.push(`${req.method()} ${pathname}`);
      }
    });

    await prepareSpec155Page(page, { unmocked });
    await page.goto("/query", { waitUntil: "domcontentloaded" });
    await expect(page.locator("textarea.query-input").first()).toBeVisible({
      timeout: 20_000,
    });
    // Let background queries and the socket settle.
    await page.waitForTimeout(3_000);

    expect(missingTenant, "tenant-scoped calls sent without X-Tenant-ID").toEqual([]);
    expect(unmocked, "API calls not covered by the mock").toEqual([]);
    expect(sockets, "WebSocket opened repeatedly").toBeLessThanOrEqual(3);
  });

  test("guard answers unknown API calls with a recorded 404 (never the live backend)", async ({
    page,
  }) => {
    const unmocked: string[] = [];
    await prepareSpec155Page(page, { unmocked });
    await page.goto("/query", { waitUntil: "domcontentloaded" });
    const status = await page.evaluate(
      async () => (await fetch("/api/v1/__not_mocked__")).status,
    );
    expect(status).toBe(404);
    expect(unmocked).toContain("GET /api/v1/__not_mocked__");
  });
});
