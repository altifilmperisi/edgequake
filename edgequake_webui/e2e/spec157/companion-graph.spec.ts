/**
 * SPEC-157 W3 — verify the answer subgraph beside the chat (Chat | Graph).
 */
import { expect, test, type Page } from "@playwright/test";
import { askQuestion, MSG_ID, openFirstCitation, openQuery, pane, shot } from "./helpers";

const showOnGraph = (page: Page) => page.getByTestId("show-on-graph").first();

async function openGraphList(page: Page) {
  await openQuery(page);
  await askQuestion(page);
  await showOnGraph(page).click();
  await page.getByTestId("companion-graph-view-list").click();
}

test.describe("SPEC-157 companion — graph pane @spec157", () => {
  test("graph_open: Show on graph docks the answer subgraph beside the chat", async ({ page }) => {
    await openQuery(page);
    await askQuestion(page);
    await showOnGraph(page).click();

    await expect(pane(page)).toBeVisible();
    await expect(page).toHaveURL(/\/query\?.*pane=graph/);
    await expect(page).toHaveURL(new RegExp(`msg=${MSG_ID}`));
    await expect(page.getByTestId("query-composer")).toBeVisible();
    await expect(page.getByTestId("companion-graph-summary")).toContainText("6 entities");
    await expect(page.getByTestId("companion-graph-summary")).toContainText("6 relationships");
    await expect(page.getByTestId("companion-graph-canvas").locator("canvas").first()).toBeVisible({
      timeout: 20_000,
    });
    await shot(page, "06-graph-pane-canvas");
  });

  test("graph_list: list view selects a node and shows its card", async ({ page }) => {
    await openGraphList(page);
    const list = page.getByTestId("companion-graph-list");
    await expect(list).toContainText("Lightrag");
    await expect(list).toContainText("Graph Indexing");
    await expect(list).toContainText("COMBINES WITH");

    await list.getByRole("button", { name: /^Lightrag/ }).click();
    const card = page.getByTestId("companion-node-card");
    await expect(card).toBeVisible();
    await expect(card).toContainText("Lightrag");
    await shot(page, "07-graph-pane-list-selected");

    await card.getByRole("button", { name: "Clear selection" }).click();
    await expect(card).toHaveCount(0);
  });

  test("graph_expand: Show neighbours merges new context nodes", async ({ page }) => {
    await openGraphList(page);
    await page.getByTestId("companion-graph-list").getByRole("button", { name: /^Lightrag/ }).click();
    await page.getByTestId("companion-expand").click();
    await expect(page.getByTestId("companion-graph-summary")).toContainText("8 entities", {
      timeout: 15_000,
    });
    await expect(page.getByTestId("companion-graph-list")).toContainText("Rag Baseline");
    await expect(page.getByTestId("companion-expand")).toBeDisabled();
    await expect(page.getByText("No new neighbours to show")).toHaveCount(0);
    await shot(page, "08-graph-pane-expanded");
  });

  test("graph_studio_link: escape hatch carries the answer to Graph Studio", async ({ page }) => {
    await openQuery(page);
    await askQuestion(page);
    await showOnGraph(page).click();
    await expect(page.getByTestId("companion-open-studio")).toHaveAttribute(
      "href",
      new RegExp(`/graph\\?.*${MSG_ID}`),
    );
  });

  test("graph_tab_switch: Source and Graph keep each pane's last target", async ({ page }) => {
    await openQuery(page);
    await askQuestion(page);
    await openFirstCitation(page);
    await showOnGraph(page).click();
    await expect(page).toHaveURL(/pane=graph/);
    await expect(page.getByTestId("companion-graph")).toBeVisible();

    await page.getByTestId("companion-tab-pdf").click();
    await expect(page).toHaveURL(/pane=pdf/);
    await expect(page).toHaveURL(/page=3/);
    await expect(page.getByTestId("companion-source")).toBeVisible();

    await page.getByTestId("companion-tab-graph").click();
    await expect(page).toHaveURL(new RegExp(`msg=${MSG_ID}`));
    await expect(pane(page)).toHaveCount(1);
  });

  test("graph_url_roundtrip: reload restores the graph from the persisted answer", async ({ page }) => {
    await openQuery(page);
    await askQuestion(page);
    await showOnGraph(page).click();
    await expect(page).toHaveURL(/pane=graph/);

    await page.reload({ waitUntil: "domcontentloaded" });
    await expect(pane(page)).toBeVisible({ timeout: 30_000 });
    await expect(page.getByTestId("companion-graph-summary")).toContainText("6 entities", {
      timeout: 30_000,
    });
  });

  test("graph_unknown_message: a stale msg id shows an honest empty state", async ({ page }) => {
    await openQuery(page, "/query?pane=graph&msg=does-not-exist");
    await expect(page.getByTestId("companion-graph-empty")).toBeVisible({ timeout: 30_000 });
    await shot(page, "09-graph-empty");
  });

  test("graph_flag_off: Show on graph keeps the legacy navigation", async ({ page }) => {
    await openQuery(page, "/query", { flagOff: true });
    await askQuestion(page);
    await showOnGraph(page).click();
    await expect(page).toHaveURL(/\/graph/);
  });
});
