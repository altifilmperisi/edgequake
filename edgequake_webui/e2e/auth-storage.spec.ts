/**
 * SPEC-154 Wave 5 — Playwright: no access/refresh secrets in localStorage after login.
 * Soft-skip when auth/login is unavailable in the environment.
 */
import { expect, test } from "@playwright/test";

test.describe("SPEC-154 auth storage", () => {
  test("login does not write accessToken/refreshToken to localStorage", async ({
    page,
  }) => {
    await page.goto("/login");
    const username = process.env.E2E_AUTH_USER;
    const password = process.env.E2E_AUTH_PASSWORD;
    if (!username || !password) {
      test.skip(true, "E2E_AUTH_USER/PASSWORD not set");
      return;
    }

    await page.getByLabel(/username|email/i).fill(username);
    await page.getByLabel(/password/i).fill(password);
    await page.getByRole("button", { name: /sign in|log in/i }).click();
    await page.waitForURL((url) => !url.pathname.includes("/login"), {
      timeout: 30_000,
    });

    const secrets = await page.evaluate(() => ({
      access: localStorage.getItem("accessToken"),
      refresh: localStorage.getItem("refreshToken"),
    }));
    expect(secrets.access).toBeNull();
    expect(secrets.refresh).toBeNull();
  });
});
