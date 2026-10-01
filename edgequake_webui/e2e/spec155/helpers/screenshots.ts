/**
 * Screenshot helper for SPEC-155 visual inspection loop.
 * Writes under specs/155-improve-ux-ui/e2e/screenshots/{subdir}/
 */
import fs from "node:fs";
import path from "node:path";
import type { Page } from "@playwright/test";

const REPO_ROOT = path.resolve(__dirname, "../../../../");
export const SPEC155_SHOTS_ROOT = path.join(
  REPO_ROOT,
  "specs/155-improve-ux-ui/e2e/screenshots",
);

export type ShotTheme = "light" | "dark";
export type ShotViewport = 375 | 768 | 1280;

export const SPEC155_ROUTES = [
  { name: "dashboard", path: "/" },
  { name: "documents", path: "/documents" },
  { name: "query", path: "/query" },
  { name: "graph", path: "/graph?stream=0" },
  { name: "pipeline", path: "/pipeline" },
  { name: "costs", path: "/costs" },
  { name: "workspace", path: "/workspace" },
  { name: "settings", path: "/settings" },
  { name: "knowledge", path: "/knowledge" },
  { name: "api-explorer", path: "/api-explorer" },
  { name: "login", path: "/login" },
] as const;

export function shotPath(
  subdir: "baseline" | "current" | "round",
  route: string,
  theme: ShotTheme,
  viewport: ShotViewport,
  round?: number,
): string {
  const dir =
    subdir === "round"
      ? path.join(SPEC155_SHOTS_ROOT, `round-${String(round ?? 1).padStart(2, "0")}`)
      : path.join(SPEC155_SHOTS_ROOT, subdir);
  fs.mkdirSync(dir, { recursive: true });
  return path.join(dir, `${route}-${theme}-${viewport}.png`);
}

export async function applyTheme(page: Page, theme: ShotTheme): Promise<void> {
  await page.evaluate((t) => {
    const root = document.documentElement;
    if (t === "dark") {
      root.classList.add("dark");
      localStorage.setItem("theme", "dark");
    } else {
      root.classList.remove("dark");
      localStorage.setItem("theme", "light");
    }
  }, theme);
}

export async function captureRoute(
  page: Page,
  opts: {
    routeName: string;
    routePath: string;
    theme: ShotTheme;
    viewport: ShotViewport;
    subdir?: "baseline" | "current" | "round";
    round?: number;
    settleMs?: number;
  },
): Promise<string> {
  const {
    routeName,
    routePath,
    theme,
    viewport,
    subdir = "current",
    round,
    settleMs = 800,
  } = opts;
  await page.setViewportSize({
    width: viewport,
    height: viewport === 375 ? 812 : viewport === 768 ? 1024 : 800,
  });
  await page.goto(routePath, { waitUntil: "domcontentloaded" });
  await applyTheme(page, theme);
  await page.waitForTimeout(settleMs);
  const file = shotPath(subdir, routeName, theme, viewport, round);
  await page.screenshot({ path: file, fullPage: true });
  return file;
}
