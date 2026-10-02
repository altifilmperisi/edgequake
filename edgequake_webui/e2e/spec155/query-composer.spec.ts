/**
 * SPEC-155 W7Q query screen gates (@spec155).
 */
import { expect, test } from "@playwright/test";
import { prepareSpec155Page } from "./helpers/mock-api";
import {
  SCENARIO_EMPTY_SOURCES,
  SCENARIO_ERROR,
  scenarioLongStream,
  scenarioManySources,
} from "./helpers/mock-chat-sse";

test.describe("SPEC-155 query composer @spec155", () => {
  test("query_compose_while_stream: textarea stays enabled", async ({
    page,
  }) => {
    await prepareSpec155Page(page, {
      chatScenario: scenarioLongStream(80),
    });
    await page.goto("/query", { waitUntil: "domcontentloaded" });
    const input = page.locator("textarea.query-input").first();
    await expect(input).toBeVisible({ timeout: 20_000 });
    await input.fill("Hello stream");
    await input.press("Enter");
    const stop = page.getByTestId("query-stop");
    await expect(stop).toBeVisible({ timeout: 10_000 });
    // Q01/Q07: composer never locks
    await expect(input).toBeEnabled();
    await input.fill("still typing while streaming");
    await expect(input).toHaveValue("still typing while streaming");
  });

  test("query_stop_keeps_partial", async ({ page }) => {
    await prepareSpec155Page(page, {
      chatScenario: {
        // Hold connection before body so Stop is clickable
        delayMs: 400,
        events: scenarioLongStream(40).events,
      },
    });
    await page.goto("/query", { waitUntil: "domcontentloaded" });
    const input = page.locator("textarea.query-input").first();
    await expect(input).toBeVisible({ timeout: 20_000 });
    await input.fill("Long answer please");

    const stopClick = (async () => {
      const stop = page.getByTestId("query-stop");
      await stop.waitFor({ state: "visible", timeout: 15_000 });
      await stop.click();
    })();

    await input.press("Enter");
    await stopClick;

    await expect(page.getByTestId("query-stopped-banner")).toBeVisible({
      timeout: 10_000,
    });
  });

  test("query_mode_menu in composer", async ({ page }) => {
    await prepareSpec155Page(page);
    await page.goto("/query", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("query-mode-selector")).toBeVisible({
      timeout: 20_000,
    });
    await page.getByTestId("query-mode-selector").click();
    await expect(page.getByTestId("query-mode-mix")).toBeVisible();
    await expect(page.getByTestId("query-mode-local")).toBeVisible();
  });
});

test.describe("SPEC-155 query stream phases @spec155", () => {
  test("query_stream_phases: stage + source chips before tokens", async ({
    page,
  }) => {
    await prepareSpec155Page(page);
    await page.goto("/query", { waitUntil: "domcontentloaded" });
    const input = page.locator("textarea.query-input").first();
    await expect(input).toBeVisible({ timeout: 20_000 });
    await input.fill("Summarize relationships");
    await input.press("Enter");
    await expect(
      page
        .getByTestId("query-stage-timeline")
        .or(page.getByTestId("query-source-chips")),
    ).toBeVisible({ timeout: 15_000 });
    await expect(page.getByText(/Mock answer about/i).first()).toBeVisible({
      timeout: 20_000,
    });
  });

  test("query_empty_sources shows no-sources hint", async ({ page }) => {
    await prepareSpec155Page(page, {
      chatScenario: SCENARIO_EMPTY_SOURCES,
    });
    await page.goto("/query", { waitUntil: "domcontentloaded" });
    const input = page.locator("textarea.query-input").first();
    await input.fill("Anything?");
    await input.press("Enter");
    await expect(page.getByTestId("query-no-sources")).toBeVisible({
      timeout: 15_000,
    });
  });
});

test.describe("SPEC-155 query source chips @spec155", () => {
  test("query_sources_more: +N more opens the panel every time", async ({
    page,
  }) => {
    await prepareSpec155Page(page, { chatScenario: scenarioManySources(9) });
    await page.goto("/query", { waitUntil: "domcontentloaded" });
    const input = page.locator("textarea.query-input").first();
    await expect(input).toBeVisible({ timeout: 20_000 });
    await input.fill("Many sources?");
    await input.press("Enter");

    const more = page.getByRole("button", { name: /\+5 more/ });
    await expect(more).toBeVisible({ timeout: 15_000 });
    const panel = page.getByTestId("source-citations");
    const trigger = panel.getByRole("button", { name: /source citations/i });
    await expect(trigger).toBeVisible({ timeout: 15_000 });
    await expect(trigger).toHaveAttribute("aria-expanded", "false");

    await more.click();
    await expect(trigger).toHaveAttribute("aria-expanded", "true");

    // Collapse manually, then click "+N more" again — must re-open (regression).
    await trigger.click();
    await expect(trigger).toHaveAttribute("aria-expanded", "false");
    await more.click();
    await expect(trigger).toHaveAttribute("aria-expanded", "true");
  });
});

test.describe("SPEC-155 query citation link @spec155", () => {
  const DOC = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";

  async function answerWithSources(page: import("@playwright/test").Page) {
    await prepareSpec155Page(page, { chatScenario: scenarioManySources(6) });
    await page.goto("/query", { waitUntil: "domcontentloaded" });
    const input = page.locator("textarea.query-input").first();
    await expect(input).toBeVisible({ timeout: 20_000 });
    await input.fill("Cite please");
    await input.press("Enter");
    const chip = page.getByTestId("query-inline-citation-2").first();
    await expect(chip).toBeVisible({ timeout: 15_000 });
    return chip;
  }

  test("query_citation_hover_card: title, page and Open source link", async ({
    page,
  }) => {
    const chip = await answerWithSources(page);
    await chip.hover();
    const card = page.getByTestId("query-citation-card");
    await expect(card).toBeVisible();
    await page.waitForTimeout(250);
    await page.screenshot({ path: "e2e/spec155/screenshots/query-citation-card.png" });
    await expect(card).toContainText("lightrag");
    await expect(card).toContainText("p.2");
    // Raw score never exceeds 100%
    await expect(card).not.toContainText(/\b(1[0-9]{2}|[2-9][0-9]{2})%/);
    const link = page.getByTestId("query-citation-open-source");
    await expect(link).toHaveAttribute("href", new RegExp(`^/documents/${DOC}\\?.*page=2`));
    // Pointer can travel from chip to card without it closing.
    await link.hover();
    await expect(card).toBeVisible();
  });

  test("query_citation_click_navigates to the source", async ({ page }) => {
    const chip = await answerWithSources(page);
    await chip.click();
    await expect(page).toHaveURL(new RegExp(`/documents/${DOC}\\?.*page=2`), {
      timeout: 15_000,
    });
  });
});

test.describe("SPEC-155 query errors @spec155", () => {
  test("query_inline_retry on error", async ({ page }) => {
    await prepareSpec155Page(page, { chatScenario: SCENARIO_ERROR });
    await page.goto("/query", { waitUntil: "domcontentloaded" });
    const input = page.locator("textarea.query-input").first();
    await input.fill("This will fail");
    await input.press("Enter");
    await expect(page.getByTestId("query-message-error")).toBeVisible({
      timeout: 15_000,
    });
    await expect(
      page.getByRole("button", { name: /try again/i }),
    ).toBeVisible();
  });
});

test.describe("SPEC-155 query history layout @spec155", () => {
  test("query_history_xl_dock: no panel squeeze at 768", async ({ page }) => {
    await page.setViewportSize({ width: 768, height: 1024 });
    await prepareSpec155Page(page);
    await page.goto("/query", { waitUntil: "domcontentloaded" });
    const input = page.locator("textarea.query-input").first();
    await expect(input).toBeVisible({ timeout: 20_000 });
    // Composer card present and reasonably wide (history not docked below xl)
    await expect(page.getByTestId("query-composer")).toBeVisible();
    // Poll: the box is 0-wide for a frame while the page lays out after mount.
    await expect
      .poll(async () => (await page.getByTestId("query-composer").boundingBox())?.width ?? 0)
      .toBeGreaterThan(300);
    // History toggle is xl-only
    await expect(page.getByTestId("query-history-toggle")).toHaveCount(0);
  });

  test("query_history_single_mount at 1280", async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 800 });
    await prepareSpec155Page(page);
    await page.goto("/query", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("query-history-toggle")).toBeVisible({
      timeout: 20_000,
    });
  });
});

test.describe("SPEC-155 query a11y live region @spec155 @a11y", () => {
  test("query_aria_live: no role=log on message list", async ({ page }) => {
    await prepareSpec155Page(page);
    await page.goto("/query", { waitUntil: "domcontentloaded" });
    await expect(page.locator("textarea.query-input").first()).toBeVisible({
      timeout: 20_000,
    });
    const logCount = await page.locator('[role="log"]').count();
    expect(logCount).toBe(0);
    await expect(page.locator('[role="status"]').first()).toBeAttached();
  });
});
