/**
 * GH-318 — Query remains available while documents are still ingesting.
 *
 * Product decision: no FE soft-gate / "Query anyway" banner. Users may query
 * mid-upload; track `expected_count` remains SSOT for batch completeness toasts.
 *
 * Strategy: mocked API only (no live backend).
 */

import { expect, test } from "@playwright/test";
import { GOTO_OPTS } from "./helpers/navigation";
import { mockBackendForUiOnly } from "./helpers/mock-backend";

function sseEvent(payload: Record<string, unknown>): string {
  return `data: ${JSON.stringify(payload)}\n\n`;
}

function mockStreamBody(answer: string): string {
  return [
    sseEvent({
      type: "conversation",
      conversation_id: "issue318-conv",
      user_message_id: "issue318-user",
    }),
    sseEvent({
      type: "context",
      sources: [],
      query_mode: "hybrid",
      retrieval_time_ms: 8,
    }),
    sseEvent({ type: "token", content: answer }),
    sseEvent({
      type: "done",
      stats: {
        embedding_time_ms: 1,
        retrieval_time_ms: 2,
        generation_time_ms: 3,
        total_time_ms: 6,
        sources_retrieved: 0,
        tokens_used: 4,
        query_mode: "hybrid",
      },
    }),
  ].join("");
}

test.describe("GH-318 Query during active ingest", () => {
  test("Send stays enabled and query succeeds while pending+processing", async ({
    page,
  }) => {
    await mockBackendForUiOnly(page);

    // Override documents list with active ingest counts (registered after
    // mockBackendForUiOnly so this handler wins).
    await page.route("**/api/v1/documents*", async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          documents: [],
          total: 2,
          page: 1,
          page_size: 1,
          total_pages: 2,
          has_more: true,
          status_counts: {
            pending: 1,
            processing: 1,
            completed: 0,
            failed: 0,
            cancelled: 0,
            partial_failure: 0,
          },
        }),
      });
    });

    let streamHit = false;
    await page.route("**/api/v1/chat/completions/stream", async (route) => {
      streamHit = true;
      await route.fulfill({
        status: 200,
        headers: { "Content-Type": "text/event-stream" },
        body: mockStreamBody("Mock answer while documents are still ingesting."),
      });
    });

    await page.goto("/query", GOTO_OPTS);
    await page.waitForLoadState("domcontentloaded");
    await page.locator("main").first().waitFor({ state: "visible", timeout: 20_000 });

    await expect(page.getByText("Query anyway")).toHaveCount(0);
    await expect(
      page.getByText(/still uploading or processing/i),
    ).toHaveCount(0);

    // SPEC-155 composer uses role=combobox (mention autocomplete), not textbox.
    const input = page.locator("textarea.query-input").first();
    await expect(input).toBeVisible({ timeout: 20_000 });
    await input.fill("What is in the knowledge graph so far?");

    const send = page.getByTestId("query-send");
    await expect(send).toBeEnabled();
    await send.click();

    await expect
      .poll(() => streamHit, { timeout: 15_000 })
      .toBe(true);
    await expect(
      page.getByText("Mock answer while documents are still ingesting."),
    ).toBeVisible({ timeout: 15_000 });
  });
});
