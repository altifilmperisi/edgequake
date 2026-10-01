/**
 * SPEC-155 graph interaction contract (@spec155)
 * hover vs selection, drag-vs-click, cursors, double-click, keyboard, neighbourhood.
 */
import { expect, test, type Page } from "@playwright/test";
import { FIXTURE_100 } from "../../src/lib/fixtures/graph";
import { prepareSpec155Page } from "./helpers/mock-api";

type Pt = { x: number; y: number };

async function openGraph(page: Page) {
  await page.addInitScript(() => {
    (window as unknown as { __EQ_E2E__: boolean }).__EQ_E2E__ = true;
  });
  await prepareSpec155Page(page, { graph: FIXTURE_100 });
  await page.goto("/graph?stream=0", { waitUntil: "domcontentloaded" });
  await expect(page.locator("[data-graph-engine-id]").first()).toBeVisible({ timeout: 60_000 });
  await page.waitForFunction(
    () => Boolean((window as unknown as { __eqSigma?: unknown }).__eqSigma),
    undefined,
    { timeout: 60_000 },
  );
  await page.waitForTimeout(1500); // initial layout settle
}

/** Screen position of the node with the given rank by degree (0 = hub). */
async function nodeByDegree(page: Page, rank: number): Promise<{ id: string; pt: Pt; neighbours: string[] }> {
  return page.evaluate((r) => {
    const sigma = (window as any).__eqSigma;
    const g = sigma.getGraph();
    const rect = sigma.getContainer().getBoundingClientRect();
    const ids = g.nodes().filter((n: string) => g.degree(n) >= 2 && !g.getNodeAttribute(n, "hidden"));
    const onScreen = ids.filter((n: string) => {
      const p = sigma.graphToViewport({ x: g.getNodeAttribute(n, "x"), y: g.getNodeAttribute(n, "y") });
      return p.x > 40 && p.y > 40 && p.x < rect.width - 40 && p.y < rect.height - 40;
    });
    onScreen.sort((a: string, b: string) => g.degree(b) - g.degree(a));
    const id = onScreen[Math.min(r, onScreen.length - 1)];
    const p = sigma.graphToViewport({ x: g.getNodeAttribute(id, "x"), y: g.getNodeAttribute(id, "y") });
    return { id, pt: { x: rect.left + p.x, y: rect.top + p.y }, neighbours: g.neighbors(id) };
  }, rank);
}

const nodeColor = (page: Page, id: string) =>
  page.evaluate((n) => (window as any).__eqSigma.getNodeDisplayData(n)?.color, id);

test.describe("SPEC-155 graph interactions @spec155", () => {
  test("hover keeps selection; drag never selects; cursors; keyboard", async ({ page }) => {
    await openGraph(page);
    const a = await nodeByDegree(page, 0);
    const far = await page.evaluate((a0) => {
      const sigma = (window as any).__eqSigma;
      const g = sigma.getGraph();
      const keep = new Set([a0.id, ...a0.neighbours]);
      const rect = sigma.getContainer().getBoundingClientRect();
      for (const n of g.nodes()) {
        if (keep.has(n)) continue;
        const p = sigma.graphToViewport({ x: g.getNodeAttribute(n, "x"), y: g.getNodeAttribute(n, "y") });
        if (p.x > 40 && p.y > 40 && p.x < rect.width - 40 && p.y < rect.height - 40) {
          return { id: n, pt: { x: rect.left + p.x, y: rect.top + p.y } };
        }
      }
      return null;
    }, a);
    expect(far).not.toBeNull();

    // Hover → grab cursor
    await page.mouse.move(a.pt.x, a.pt.y);
    await expect
      .poll(() => page.evaluate(() => (window as any).__eqSigma.getContainer().style.cursor))
      .toBe("grab");

    // Click selects → neighbourhood status appears
    await page.mouse.click(a.pt.x, a.pt.y);
    await expect(page.getByTestId("ego-clear")).toBeVisible();
    const label = await page.getByTestId("ego-status").getAttribute("title");
    expect(label).toBeTruthy();
    const neighbour = a.neighbours[0]!;
    const before = await nodeColor(page, neighbour);

    // Hover a far node: the selection's neighbourhood must stay lit
    await page.mouse.move(far!.pt.x, far!.pt.y, { steps: 5 });
    await page.waitForTimeout(200);
    expect(await nodeColor(page, neighbour)).toBe(before);

    // Drag the far node: moves it, does NOT change the selection
    const posBefore = await page.evaluate(
      (id) => { const g = (window as any).__eqSigma.getGraph(); return [g.getNodeAttribute(id, "x"), g.getNodeAttribute(id, "y")]; }, far!.id);
    await page.mouse.down();
    await page.mouse.move(far!.pt.x + 60, far!.pt.y + 40, { steps: 8 });
    await page.mouse.up();
    await page.waitForTimeout(100);
    const posAfter = await page.evaluate(
      (id) => { const g = (window as any).__eqSigma.getGraph(); return [g.getNodeAttribute(id, "x"), g.getNodeAttribute(id, "y")]; }, far!.id);
    expect(posAfter).not.toEqual(posBefore);
    await expect(page.getByTestId("ego-status")).toHaveAttribute("title", label!);

    // Keyboard (canvas focused by the pointer press)
    await page.keyboard.press("3");
    await expect(page.getByTestId("ego-depth-3")).toHaveAttribute("aria-checked", "true");
    await page.keyboard.press("1");
    await expect(page.getByTestId("ego-depth-1")).toHaveAttribute("aria-checked", "true");
    await page.keyboard.press("ArrowRight");
    await expect
      .poll(() => page.getByTestId("ego-status").getAttribute("title"))
      .not.toBe(label);
    await page.keyboard.press("Escape");
    await expect(page.getByTestId("ego-clear")).toHaveCount(0);
  });

  test("double-click fits the neighbourhood", async ({ page }) => {
    await openGraph(page);
    const n = await nodeByDegree(page, 1);
    const ratio0 = await page.evaluate(() => (window as any).__eqSigma.getCamera().ratio);
    await page.mouse.dblclick(n.pt.x, n.pt.y);
    await expect(page.getByTestId("ego-clear")).toBeVisible();
    await expect
      .poll(() => page.evaluate(() => (window as any).__eqSigma.getCamera().ratio), { timeout: 5000 })
      .not.toBe(ratio0);
  });

  test("hub edge labels stay limited", async ({ page }) => {
    await openGraph(page);
    const hub = await nodeByDegree(page, 0);
    await page.mouse.click(hub.pt.x, hub.pt.y);
    await page.waitForTimeout(300);
    const labelled = await page.evaluate((id) => {
      const sigma = (window as any).__eqSigma;
      const g = sigma.getGraph();
      return g.edges(id).filter((e: string) => sigma.getEdgeDisplayData(e)?.label).length;
    }, hub.id);
    const degree = await page.evaluate((id) => (window as any).__eqSigma.getGraph().degree(id), hub.id);
    if (degree > 6) expect(labelled).toBe(0);
  });

  test("right-click opens the menu, never drags, and leaves selection alone", async ({ page }) => {
    await openGraph(page);
    const n = await nodeByDegree(page, 1);
    const pos = () =>
      page.evaluate((id) => {
        const g = (window as any).__eqSigma.getGraph();
        return [g.getNodeAttribute(id, "x"), g.getNodeAttribute(id, "y")];
      }, n.id);
    const before = await pos();

    await page.mouse.move(n.pt.x, n.pt.y);
    await page.mouse.click(n.pt.x, n.pt.y, { button: "right" });
    await expect(page.getByTestId("node-context-menu")).toBeVisible();
    // Right-click is not a selection
    await expect(page.getByTestId("ego-clear")).toHaveCount(0);

    // "Sticky drag" regression: the pointer travelling afterwards must not move the node
    await page.mouse.move(n.pt.x + 120, n.pt.y + 90, { steps: 10 });
    await page.mouse.move(n.pt.x + 200, n.pt.y + 150, { steps: 10 });
    expect(await pos()).toEqual(before);

    // Esc closes the menu and returns focus to the canvas
    await page.keyboard.press("Escape");
    await expect(page.getByTestId("node-context-menu")).toHaveCount(0);
    await expect
      .poll(() => page.evaluate(() => document.activeElement?.hasAttribute("data-graph-engine")))
      .toBe(true);
    expect(await pos()).toEqual(before);

    // A normal drag still works afterwards (Radix releases body pointer-events async).
    // Whatever node sits under the pointer is the one that must move.
    await expect
      .poll(() => page.evaluate(() => getComputedStyle(document.body).pointerEvents))
      .not.toBe("none");
    await page.evaluate(() => {
      const s = (window as any).__eqSigma;
      (window as any).__pressed = null;
      s.once("downNode", ({ node }: { node: string }) => {
        const g = s.getGraph();
        (window as any).__pressed = { node, x: g.getNodeAttribute(node, "x") };
      });
    });
    await page.mouse.move(n.pt.x, n.pt.y);
    await page.mouse.down();
    await page.mouse.move(n.pt.x + 50, n.pt.y + 30, { steps: 8 });
    await page.mouse.up();
    const moved = await page.evaluate(() => {
      const p = (window as any).__pressed;
      const g = (window as any).__eqSigma.getGraph();
      return p && g.getNodeAttribute(p.node, "x") !== p.x;
    });
    expect(moved).toBe(true);
  });

  test("keyboard twin: Shift+F10 opens the menu for the selected node", async ({ page }) => {
    await openGraph(page);
    const n = await nodeByDegree(page, 1);
    await page.mouse.click(n.pt.x, n.pt.y);
    await page.keyboard.press("Shift+F10");
    await expect(page.getByTestId("node-context-menu")).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(page.getByTestId("node-context-menu")).toHaveCount(0);
    // selection survives the menu round-trip
    await expect(page.getByTestId("ego-clear")).toBeVisible();
  });
});
