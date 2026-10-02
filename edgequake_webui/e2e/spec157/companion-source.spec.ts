/**
 * SPEC-157 W2 — verify a citation beside the chat (Chat | Source).
 */
import { expect, test } from "@playwright/test";
import { askQuestion, cite, DOC_ID, openFirstCitation, openQuery, pane, shot, TEXT_DOC_ID } from "./helpers";

test.describe("SPEC-157 companion — source pane @spec157", () => {
  test("companion_open_source: citation click docks the PDF beside the chat", async ({ page }) => {
    await openQuery(page);
    await shot(page, "01-query-empty");
    await askQuestion(page);
    await shot(page, "02-answer-with-citations");

    await openFirstCitation(page);
    await expect(page).toHaveURL(/\/query\?.*pane=pdf/);
    await expect(page).toHaveURL(new RegExp(`doc=${DOC_ID}`));
    await expect(page).toHaveURL(/page=3/);
    // Chat never leaves: still on /query, answer + composer visible.
    await expect(page.getByTestId("query-composer")).toBeVisible();
    await expect(cite(page, 1)).toBeVisible();

    await expect(page.getByTestId("companion-source-title")).toContainText("LightRAG");
    await expect(page.getByTestId("companion-source-page")).toHaveText("p.3");
    await expect(page.getByTestId("pdf-page-indicator")).toContainText(`/ 12`, { timeout: 30_000 });
    await expect(page.getByTestId("pdf-page-indicator")).toHaveAttribute("data-page", "3");
    await expect(page.getByTestId("companion-passage-text")).toContainText("dual-level");
    await shot(page, "03-source-pane-pdf");
  });

  test("companion_switch_citation: another citation retargets the same pane", async ({ page }) => {
    await openQuery(page);
    await askQuestion(page);
    await openFirstCitation(page);
    await expect(page.getByTestId("pdf-page-indicator")).toHaveAttribute("data-page", "3", { timeout: 30_000 });

    await cite(page, 2).click();
    await expect(page).toHaveURL(/page=7/);
    await expect(page.getByTestId("pdf-page-indicator")).toHaveAttribute("data-page", "7", { timeout: 15_000 });
    await expect(pane(page)).toHaveCount(1);

    // Re-clicking the first citation after scrolling away re-navigates.
    await cite(page, 1).click();
    await expect(page.getByTestId("pdf-page-indicator")).toHaveAttribute("data-page", "3", { timeout: 15_000 });
  });

  test("companion_text_document: text sources show highlighted content", async ({ page }) => {
    await openQuery(page);
    await askQuestion(page);
    await cite(page, 3).click();
    await expect(page).toHaveURL(new RegExp(`doc=${TEXT_DOC_ID}`));
    await expect(page.getByTestId("companion-source-text")).toContainText("Release notes", { timeout: 15_000 });
    // No PDF toggle for text documents.
    await expect(page.getByTestId("companion-view-page")).toHaveCount(0);
    await shot(page, "04-source-pane-text");
  });

  test("companion_view_toggle: Page ↔ Text for PDFs", async ({ page }) => {
    await openQuery(page);
    await askQuestion(page);
    await openFirstCitation(page);
    await page.getByTestId("companion-view-text").click();
    await expect(page.getByTestId("companion-source-text")).toBeVisible();
    await page.getByTestId("companion-view-page").click();
    await expect(page.getByTestId("companion-source-pdf")).toBeVisible();
  });

  test("companion_url_roundtrip: reload restores the pane; Back closes it", async ({ page }) => {
    await openQuery(page);
    await askQuestion(page);
    await openFirstCitation(page);
    const url = page.url();

    await page.reload({ waitUntil: "domcontentloaded" });
    await expect(pane(page)).toBeVisible({ timeout: 30_000 });
    await expect(page.getByTestId("pdf-page-indicator")).toHaveAttribute("data-page", "3", { timeout: 30_000 });
    expect(page.url()).toBe(url);

    await page.goBack();
    await expect(pane(page)).toHaveCount(0);
  });

  test("companion_deeplink: shared URL opens the pane on a cold start", async ({ page }) => {
    await openQuery(page, `/query?pane=pdf&doc=${DOC_ID}&page=5`);
    await expect(pane(page)).toBeVisible({ timeout: 30_000 });
    await expect(page.getByTestId("pdf-page-indicator")).toHaveAttribute("data-page", "5", { timeout: 30_000 });
  });

  test("companion_bad_params: malformed pane params fail closed, chat intact", async ({ page }) => {
    await openQuery(page, "/query?pane=pdf&doc=../../etc/passwd&page=abc");
    await expect(page.getByTestId("query-composer")).toBeVisible({ timeout: 30_000 });
    await expect(pane(page)).toHaveCount(0);
    await openQuery(page, "/query?pane=graph");
    await expect(pane(page)).toHaveCount(0);
  });

  test("companion_missing_document: deleted source shows an honest state", async ({ page }) => {
    await openQuery(page, "/query?pane=pdf&doc=99999999-9999-4999-8999-999999999999&page=1");
    await expect(page.getByTestId("companion-source-gone")).toBeVisible({ timeout: 30_000 });
    await shot(page, "05-source-missing");
  });

  test("companion_close_restores_focus: Esc closes and returns focus to the chip", async ({ page }) => {
    await openQuery(page);
    await askQuestion(page);
    const chip = cite(page, 1);
    await chip.focus();
    await page.keyboard.press("Enter");
    await expect(pane(page)).toBeVisible();
    await pane(page).getByTestId("companion-close").focus();
    await page.keyboard.press("Escape");
    await expect(pane(page)).toHaveCount(0);
    await expect(chip).toBeFocused();
    await expect(page).not.toHaveURL(/pane=/);
  });

  test("companion_modifier_click: ctrl-click keeps the native new-tab behaviour", async ({ page }) => {
    await openQuery(page);
    await askQuestion(page);
    await page.evaluate(() => {
      const w = window as unknown as { __opened: string[] };
      w.__opened = [];
      window.open = ((url?: string | URL) => {
        w.__opened.push(String(url));
        return null;
      }) as typeof window.open;
    });
    await cite(page, 1).click({ modifiers: ["ControlOrMeta"] });
    const opened = await page.evaluate(() => (window as unknown as { __opened: string[] }).__opened);
    expect(opened).toHaveLength(1);
    expect(opened[0]).toMatch(new RegExp(`/documents/${DOC_ID}`));
    await expect(pane(page)).toHaveCount(0);
  });

  test("companion_flag_off: citations navigate to the full page as before", async ({ page }) => {
    await openQuery(page, "/query", { flagOff: true });
    await askQuestion(page);
    await cite(page, 1).click();
    await expect(page).toHaveURL(new RegExp(`/documents/${DOC_ID}`));
  });
});
