/**
 * Live-stack query smoke (SPEC-155 W7Q) — requires E2E_LIVE_STACK=1 + running backend.
 */
import { expect, test } from "@playwright/test";
import { gotoQueryPage, submitQueryAndWait } from "../helpers/qc-query";

test.describe("SPEC-155 query live @live", () => {
  test.skip(
    process.env.E2E_LIVE_STACK !== "1",
    "Set E2E_LIVE_STACK=1 with make dev-bg for live query proof",
  );

  test("streamed query shows composer + answer", async ({ page }) => {
    await gotoQueryPage(page);
    await expect(page.getByTestId("query-composer")).toBeVisible();
    await expect(page.getByTestId("query-mode-selector")).toBeVisible();
    const text = await submitQueryAndWait(page, "What is EdgeQuake?", {
      answerTimeoutMs: 180_000,
    });
    expect(text.length).toBeGreaterThan(10);
  });
});
