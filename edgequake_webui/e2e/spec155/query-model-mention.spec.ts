/**
 * SPEC-155 W7Q — model picker, capability-aware streaming and `@` mentions (@spec155).
 */
import { expect, test, type Page } from "@playwright/test";
import {
  prepareSpec155Page,
  type ChatRequestCapture,
} from "./helpers/mock-api";

const LIGHTRAG_ID = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
const GNN_ID = "cccccccc-cccc-4ccc-8ccc-cccccccccccc";

const DOCS = [
  {
    id: LIGHTRAG_ID,
    title: "LightRAG Paper",
    file_name: "lightrag.pdf",
    status: "completed",
    created_at: "2026-01-15T10:00:00Z",
    updated_at: "2026-01-15T11:00:00Z",
    content_type: "application/pdf",
    size_bytes: 1_200_000,
  },
  {
    id: GNN_ID,
    title: "Graph Neural Networks Survey",
    file_name: "gnn.pdf",
    status: "completed",
    created_at: "2026-01-17T10:00:00Z",
    updated_at: "2026-01-17T11:00:00Z",
    content_type: "application/pdf",
    size_bytes: 900_000,
  },
];

async function openQuery(page: Page, chatRequests: ChatRequestCapture[] = []) {
  await prepareSpec155Page(page, { documents: DOCS, chatRequests });
  await page.goto("/query", { waitUntil: "domcontentloaded" });
  const input = page.locator("textarea.query-input").first();
  await expect(input).toBeVisible({ timeout: 20_000 });
  return input;
}

async function chooseModel(page: Page, name: RegExp) {
  await page.getByTestId("query-model-chip").click();
  await expect(page.getByTestId("query-model-menu")).toBeVisible();
  await page.getByTestId("query-model-option").filter({ hasText: name }).click();
  await expect(page.getByTestId("query-model-menu")).toBeHidden();
}

test.describe("SPEC-155 query model picker @spec155", () => {
  test("model_menu: lists providers, searches and selects", async ({ page }) => {
    await openQuery(page);
    await page.getByTestId("query-model-chip").click();
    const menu = page.getByTestId("query-model-menu");
    await expect(menu).toBeVisible();
    await expect(page.getByTestId("query-model-option")).toHaveCount(4);

    await page.getByTestId("query-model-search").fill("mini");
    await expect(page.getByTestId("query-model-option")).toHaveCount(1);
    await expect(page.getByTestId("query-model-option")).toContainText(
      "gpt-5-mini",
    );

    await page.getByTestId("query-model-option").click();
    await expect(menu).toBeHidden();
    await expect(page.getByTestId("query-model-chip")).toContainText(
      "gpt-5-mini",
    );
  });

  test("model_selection is sent with the request (streaming)", async ({
    page,
  }) => {
    const requests: ChatRequestCapture[] = [];
    const input = await openQuery(page, requests);
    await chooseModel(page, /GPT-5 Mini/);

    await input.fill("Which model answers?");
    await input.press("Enter");
    await expect(page.getByText(/Mock answer about/i).first()).toBeVisible({
      timeout: 20_000,
    });

    expect(requests).toHaveLength(1);
    expect(requests[0].stream).toBe(true);
    expect(requests[0].body).toMatchObject({
      provider: "openai",
      model: "gpt-5-mini",
    });
  });

  test("non-streaming model: toggle disabled and request falls back to blocking", async ({
    page,
  }) => {
    const requests: ChatRequestCapture[] = [];
    const input = await openQuery(page, requests);
    await chooseModel(page, /Batch only/);

    await page.getByTestId("query-model-chip").click();
    await expect(page.getByTestId("query-model-stream-toggle")).toBeDisabled();
    await page.keyboard.press("Escape");

    await input.fill("No streaming please");
    await input.press("Enter");
    await expect(page.getByText("Mock non-stream answer").first()).toBeVisible({
      timeout: 20_000,
    });
    expect(requests).toHaveLength(1);
    expect(requests[0].stream).toBe(false);
    expect(requests[0].body).toMatchObject({ model: "batch-only" });
  });

  test("stream toggle off sends a blocking request even for streaming models", async ({
    page,
  }) => {
    const requests: ChatRequestCapture[] = [];
    const input = await openQuery(page, requests);

    await page.getByTestId("query-model-chip").click();
    const toggle = page.getByTestId("query-model-stream-toggle");
    await expect(toggle).toBeEnabled();
    await toggle.click();
    await page.keyboard.press("Escape");

    await input.fill("Blocking mode");
    await input.press("Enter");
    await expect(page.getByText("Mock non-stream answer").first()).toBeVisible({
      timeout: 20_000,
    });
    expect(requests[0].stream).toBe(false);
  });

  test("server default entry clears the override", async ({ page }) => {
    await openQuery(page);
    await chooseModel(page, /Gemma 3/);
    await expect(page.getByTestId("query-model-chip")).toContainText("gemma3");
    await page.getByTestId("query-model-chip").click();
    await page.getByTestId("query-model-option-default").click();
    await expect(page.getByTestId("query-model-chip")).not.toContainText(
      "gemma3",
    );
  });

  test("model menu keyboard: type to search, arrows + Enter select", async ({
    page,
  }) => {
    await openQuery(page);
    await page.getByTestId("query-model-chip").click();
    await page.getByTestId("query-model-search").fill("gemma");
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("Enter");
    await expect(page.getByTestId("query-model-chip")).toContainText("gemma3");
  });
});

test.describe("SPEC-155 query @ mention @spec155", () => {
  test("typing @ opens a filtered document menu; Enter scopes the document", async ({
    page,
  }) => {
    const requests: ChatRequestCapture[] = [];
    const input = await openQuery(page, requests);

    await input.click();
    await input.pressSequentially("Explain @Light");
    const menu = page.getByTestId("query-mention-menu");
    await expect(menu).toBeVisible();
    const options = page.getByTestId("query-mention-option");
    await expect(options).toHaveCount(1);
    await expect(options.first()).toContainText("LightRAG Paper");

    await input.press("Enter");
    await expect(menu).toBeHidden();
    // Mention token is stripped, the doc becomes a removable chip.
    await expect(input).toHaveValue("Explain ");
    const chips = page.getByTestId("query-scope-chip-doc");
    await expect(chips).toHaveCount(1);
    await expect(chips.first()).toContainText("LightRAG Paper");

    await input.pressSequentially("how it works");
    await input.press("Enter");
    await expect(page.getByText(/Mock answer about/i).first()).toBeVisible({
      timeout: 20_000,
    });
    expect(requests[0].body).toMatchObject({
      document_filter: { document_ids: [LIGHTRAG_ID] },
    });
  });

  test("arrow keys navigate, Tab picks, chip can be removed", async ({
    page,
  }) => {
    const input = await openQuery(page);
    await input.click();
    await input.pressSequentially("@");
    await expect(page.getByTestId("query-mention-menu")).toBeVisible();
    await expect(page.getByTestId("query-mention-option")).toHaveCount(2);

    await input.press("ArrowDown");
    await input.press("Tab");
    await expect(page.getByTestId("query-scope-chip-doc")).toHaveCount(1);

    // Already-scoped docs are excluded from the next menu.
    await input.pressSequentially("@");
    await expect(page.getByTestId("query-mention-option")).toHaveCount(1);
    await input.press("Escape");
    await expect(page.getByTestId("query-mention-menu")).toBeHidden();

    await page
      .getByTestId("query-scope-chip-doc")
      .getByRole("button")
      .first()
      .click();
    await expect(page.getByTestId("query-scope-chip-doc")).toHaveCount(0);
  });

  test("email-like text does not open the menu", async ({ page }) => {
    const input = await openQuery(page);
    await input.click();
    await input.pressSequentially("write to me@example.com");
    await expect(page.getByTestId("query-mention-menu")).toHaveCount(0);
  });

  test("Esc dismisses and Enter then submits the message", async ({ page }) => {
    const requests: ChatRequestCapture[] = [];
    const input = await openQuery(page, requests);
    await input.click();
    await input.pressSequentially("hello @zzz-no-match");
    // No match → menu hidden (query has no whitespace and no items → empty state may show)
    await input.press("Escape");
    await expect(page.getByTestId("query-mention-menu")).toHaveCount(0);
    await input.press("Enter");
    await expect(page.getByText(/Mock answer about/i).first()).toBeVisible({
      timeout: 20_000,
    });
    expect(requests).toHaveLength(1);
  });

  test("visual: model menu and mention menu are borderless-soft", async ({
    page,
  }) => {
    const input = await openQuery(page);
    await page.getByTestId("query-model-chip").click();
    const menu = page.getByTestId("query-model-menu");
    await expect(menu).toBeVisible();
    const radius = await menu.evaluate((el) =>
      parseFloat(getComputedStyle(el.closest("[data-slot='popover-content']") ?? el).borderTopLeftRadius),
    );
    expect(radius).toBeGreaterThanOrEqual(12);
    await page.waitForTimeout(300); // let enter animation settle
    await page.screenshot({
      path: "e2e/spec155/screenshots/query-model-menu.png",
    });
    await page.keyboard.press("Escape");

    await input.click();
    await input.pressSequentially("@");
    await expect(page.getByTestId("query-mention-menu")).toBeVisible();
    await page.waitForTimeout(300);
    await page.screenshot({
      path: "e2e/spec155/screenshots/query-mention-menu.png",
    });
  });
});
