/**
 * SPEC-157 W4/W5 — accessibility of the companion and the AI affordances
 * (quote-to-ask, scope chip).
 */
import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import { askQuestion, openFirstCitation, openQuery, pane, shot } from "./helpers";

const composerInput = (page: Page) => page.locator("textarea.query-input").first();

async function expectNoCriticalAxe(page: Page, include: string) {
  const results = await new AxeBuilder({ page })
    .include(include)
    .withTags(["wcag2a", "wcag2aa"])
    .analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad, JSON.stringify(bad.map((v) => ({ id: v.id, nodes: v.nodes.map((n) => n.target) })), null, 2)).toEqual([]);
}

test.describe("SPEC-157 companion — a11y @spec157", () => {
  test("a11y_source_pane: landmarks, tabs and axe", async ({ page }) => {
    await openQuery(page);
    await askQuestion(page);
    await openFirstCitation(page);

    await expect(page.getByRole("complementary", { name: "Companion pane" })).toBeVisible();
    const tablist = page.getByRole("tablist");
    await expect(tablist).toBeVisible();
    await expect(page.getByRole("tab", { name: /Source/ })).toHaveAttribute("aria-selected", "true");
    await expect(page.getByRole("tabpanel")).toBeVisible();
    await expectNoCriticalAxe(page, '[data-testid="query-companion"]');
  });

  test("a11y_graph_pane: axe on canvas and list views", async ({ page }) => {
    await openQuery(page);
    await askQuestion(page);
    await page.getByTestId("show-on-graph").first().click();
    await expect(page.getByTestId("companion-graph")).toBeVisible();
    await expectNoCriticalAxe(page, '[data-testid="query-companion"]');
    await page.getByTestId("companion-graph-view-list").click();
    await expect(page.getByTestId("companion-graph-view-list")).toHaveAttribute("aria-pressed", "true");
    await expectNoCriticalAxe(page, '[data-testid="query-companion"]');
  });

  test("a11y_live_region: opening and closing is announced politely", async ({ page }) => {
    await openQuery(page);
    await askQuestion(page);
    const live = (text: string) =>
      page.locator('[role="status"][aria-live="polite"]').filter({ hasText: text });
    await openFirstCitation(page);
    await expect(live("Source opened beside the chat")).toHaveCount(1);
    await pane(page).getByTestId("companion-close").click();
    await expect(live("Pane closed")).toHaveCount(1);
  });

  test("a11y_tabs_keyboard: arrow keys move between Source and Graph", async ({ page }) => {
    await openQuery(page);
    await askQuestion(page);
    await openFirstCitation(page);
    await page.getByTestId("show-on-graph").first().click();
    await expect(page).toHaveURL(/pane=graph/);

    const graphTab = page.getByTestId("companion-tab-graph");
    await graphTab.focus();
    await page.keyboard.press("ArrowLeft");
    await expect(page).toHaveURL(/pane=pdf/);
    await page.keyboard.press("ArrowRight");
    await expect(page).toHaveURL(/pane=graph/);
  });
});

test.describe("SPEC-157 companion — AI affordances @spec157", () => {
  test("ai_quote_to_ask: Ask about this quotes the passage into the composer", async ({ page }) => {
    await openQuery(page);
    await askQuestion(page);
    await openFirstCitation(page);

    await page.getByTestId("companion-quote-ask").click();
    const input = composerInput(page);
    await expect(input).toHaveValue(/^> LightRAG integrates graph structures/);
    await expect(input).toBeFocused();
    // Nothing was sent: the conversation still has a single exchange.
    await expect(page.getByTestId("query-inline-citation-1").first()).toBeVisible();
    await shot(page, "12-quote-to-ask");
  });

  test("ai_scope_toggle: scope this document adds then removes a composer chip", async ({ page }) => {
    await openQuery(page);
    await askQuestion(page);
    await openFirstCitation(page);

    const toggle = page.getByTestId("companion-scope-toggle");
    await expect(toggle).toHaveAttribute("aria-pressed", "false");
    await toggle.click();
    await expect(toggle).toHaveAttribute("aria-pressed", "true");
    await expect(page.getByTestId("query-scope-chip-doc")).toHaveCount(1);
    await shot(page, "13-scope-to-document");

    await toggle.click();
    await expect(toggle).toHaveAttribute("aria-pressed", "false");
    await expect(page.getByTestId("query-scope-chip-doc")).toHaveCount(0);
  });
});
