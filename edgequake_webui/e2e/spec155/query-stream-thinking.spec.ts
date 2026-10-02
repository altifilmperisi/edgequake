/**
 * SPEC-155 streaming thinking / CoT gates (@spec155).
 */
import { expect, test } from "@playwright/test";
import { prepareSpec155Page } from "./helpers/mock-api";
import { SCENARIO_THINKING } from "./helpers/mock-chat-sse";

test.describe("SPEC-155 query stream thinking @spec155", () => {
  test("query_stream_thinking: live panel then answer without leak", async ({
    page,
  }) => {
    await prepareSpec155Page(page, {
      chatScenario: {
        // Hold early so we can observe the live thinking state
        delayMs: 80,
        events: SCENARIO_THINKING.events,
      },
    });
    await page.goto("/query", { waitUntil: "domcontentloaded" });
    const input = page.locator("textarea.query-input").first();
    await expect(input).toBeVisible({ timeout: 20_000 });
    await input.fill("Explain graph RAG with reasoning");
    await input.press("Enter");

    const panel = page.getByTestId("query-reasoning-panel");
    await expect(panel).toBeVisible({ timeout: 15_000 });

    // While thinking is live, raw tags must not appear in the answer prose
    await expect
      .poll(async () => {
        const body = await page.locator(".prose-chat").first().textContent().catch(() => "");
        return body ?? "";
      }, { timeout: 15_000 })
      .not.toMatch(/<\/?think/i);

    // After stream completes, answer text is visible and think body is separate
    await expect(page.getByText(/Graph RAG retrieves/i).first()).toBeVisible({
      timeout: 20_000,
    });
    // Panel may be collapsed but must remain mounted with CoT content
    await expect(panel).toBeVisible({ timeout: 10_000 });
    await expect(panel).toHaveAttribute("data-live", "false");
    const answer = await page.locator(".prose-chat").first().textContent();
    expect(answer ?? "").not.toMatch(/<\/?think/i);
    expect(answer ?? "").toMatch(/Graph RAG retrieves/i);
  });

  test("query_stream_thinking: stage timeline can show Thinking", async ({
    page,
  }) => {
    await prepareSpec155Page(page, {
      chatScenario: {
        delayMs: 120,
        events: SCENARIO_THINKING.events,
      },
    });
    await page.goto("/query", { waitUntil: "domcontentloaded" });
    const input = page.locator("textarea.query-input").first();
    await expect(input).toBeVisible({ timeout: 20_000 });
    await input.fill("Think carefully");
    await input.press("Enter");

    // Either stage timeline (thinking) or live reasoning panel appears
    await expect(
      page
        .getByTestId("query-reasoning-panel")
        .or(page.getByTestId("query-stage-timeline")),
    ).toBeVisible({ timeout: 15_000 });
  });
});
