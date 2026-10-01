/**
 * SPEC-155 W0 baseline capture (@audit @spec155)
 * Writes screenshots under specs/155-improve-ux-ui/e2e/screenshots/baseline
 */
import { test } from "@playwright/test";
import { prepareSpec155Page } from "./helpers/mock-api";
import {
  SPEC155_ROUTES,
  captureRoute,
  type ShotTheme,
  type ShotViewport,
} from "./helpers/screenshots";

const THEMES: ShotTheme[] = ["light", "dark"];
const VIEWPORTS: ShotViewport[] = [375, 768, 1280];

test.describe("SPEC-155 baseline capture @audit @spec155", () => {
  test.describe.configure({ mode: "serial" });
  test.setTimeout(180_000);

  test("capture all routes themes viewports", async ({ page, browserName }) => {
    test.skip(browserName !== "chromium", "SPEC-155 capture is chromium-only");
    await prepareSpec155Page(page);

    for (const route of SPEC155_ROUTES) {
      for (const theme of THEMES) {
        // Full matrix is expensive — capture 1280 for all, 375/768 light only
        const vps: ShotViewport[] =
          theme === "light" ? VIEWPORTS : ([1280] as ShotViewport[]);
        for (const viewport of vps) {
          await captureRoute(page, {
            routeName: route.name,
            routePath: route.path,
            theme,
            viewport,
            subdir: "baseline",
            settleMs: 600,
          });
        }
      }
    }
  });
});
