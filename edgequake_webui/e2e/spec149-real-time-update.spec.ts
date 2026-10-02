/**
 * SPEC-149 — Real-time update E2E with mocked WebSocket (routeWebSocket).
 *
 * @implements E-149-01 token-bearing URL
 * @implements E-149-02 StageTransition updates row without extra list GET
 * @implements E-149-01 anonymous login has no WS
 */
import { expect, test, type Page, type WebSocketRoute } from "@playwright/test";

const FAKE_TOKEN = "spec149-e2e-access-token";
const TRACK_ID = "track-spec149";
const DOC_ID = "doc-spec149";

/**
 * Layout.tsx overwrites `window.__EDGEQUAKE_RUNTIME_CONFIG__` after init scripts.
 * Intercept assignment so SPEC-149 auth/WS assumptions always win.
 */
async function forceRuntimeConfig(
  page: Page,
  overrides: Record<string, unknown>,
) {
  await page.addInitScript((forced) => {
    let config: Record<string, unknown> = { ...forced };
    Object.defineProperty(window, "__EDGEQUAKE_RUNTIME_CONFIG__", {
      configurable: true,
      get() {
        return config;
      },
      set(value: Record<string, unknown>) {
        config = { ...(value ?? {}), ...forced };
      },
    });
  }, overrides);
}

async function seedAuthenticatedSession(page: Page) {
  await forceRuntimeConfig(page, {
    apiUrl: "",
    authEnabled: true,
    disableDemoLogin: true,
    healthPollIntervalMs: false,
  });
  await page.addInitScript(
    ({ token, tenantId, workspaceId }) => {
      // Spoof automation probes so ProgressWebSocket auto-connects (SPEC-149).
      Object.defineProperty(Navigator.prototype, "webdriver", {
        configurable: true,
        get: () => false,
      });
      Object.defineProperty(window, "__PLAYWRIGHT__", {
        configurable: true,
        value: false,
      });
      // SPEC-154: memory token adopted by AuthGuard (not localStorage).
      window.__eqE2ePendingToken = token;
      localStorage.setItem(
        "edgequake-auth",
        JSON.stringify({
          state: {
            isAuthenticated: true,
            user: { username: "spec149", user_id: "u-spec149" },
            expiresAt: Date.now() + 3_600_000,
          },
          version: 1,
        }),
      );
      localStorage.setItem("tenantId", tenantId);
      localStorage.setItem("workspaceId", workspaceId);
      localStorage.setItem(
        "edgequake-tenant",
        JSON.stringify({
          state: {
            selectedTenantId: tenantId,
            selectedWorkspaceId: workspaceId,
            workspaces: [
              {
                id: workspaceId,
                name: "default",
                tenant_id: tenantId,
              },
            ],
            tenants: [{ id: tenantId, name: "Default" }],
          },
          version: 1,
        }),
      );
    },
    {
      token: FAKE_TOKEN,
      tenantId: "00000000-0000-0000-0000-000000000002",
      workspaceId: "00000000-0000-0000-0000-000000000003",
    },
  );
}

async function stubBootstrapApis(page: Page) {
  await page.route("**/live", (route) =>
    route.fulfill({ status: 200, body: "ok" }),
  );
  await page.route("**/health", (route) =>
    route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ status: "healthy", components: {} }),
    }),
  );
  await page.route("**/api/v1/tenants/*/workspaces**", (route) =>
    route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        items: [
          {
            id: "00000000-0000-0000-0000-000000000003",
            name: "default",
            tenant_id: "00000000-0000-0000-0000-000000000002",
          },
        ],
        total: 1,
        offset: 0,
        limit: 100,
      }),
    }),
  );
  await page.route("**/api/v1/tenants*", (route) =>
    route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        items: [
          { id: "00000000-0000-0000-0000-000000000002", name: "Default" },
        ],
        total: 1,
        offset: 0,
        limit: 100,
      }),
    }),
  );
  await page.route("**/api/v1/workspaces**", (route) =>
    route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify([
        {
          id: "00000000-0000-0000-0000-000000000003",
          name: "default",
          tenant_id: "00000000-0000-0000-0000-000000000002",
        },
      ]),
    }),
  );
  await page.route("**/api/v1/auth/**", (route) =>
    route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        needs_setup: false,
        auth_enabled: true,
        user: { username: "spec149" },
      }),
    }),
  );
  await page.route("**/api/v1/pipeline/**", (route) =>
    route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        statistics: {
          processing: 1,
          pending: 0,
          indexed: 0,
          failed: 0,
        },
      }),
    }),
  );
  await page.route("**/api/v1/tasks**", (route) =>
    route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        tasks: [],
        statistics: {
          processing: 1,
          pending: 0,
          indexed: 0,
          failed: 0,
        },
      }),
    }),
  );
}

function attachProgressWebSocketMock(
  page: Page,
): {
  routed: Promise<{ url: string; route: WebSocketRoute }>;
  register: () => Promise<void>;
} {
  let resolveRouted!: (value: { url: string; route: WebSocketRoute }) => void;
  const routed = new Promise<{ url: string; route: WebSocketRoute }>(
    (resolve) => {
      resolveRouted = resolve;
    },
  );

  const register = async () => {
    await page.routeWebSocket(
      (url) => url.toString().includes("/ws/pipeline/progress"),
      (ws) => {
        resolveRouted({ url: ws.url(), route: ws });

        ws.send(
          JSON.stringify({
            type: "Connected",
            data: { message: "Connected to pipeline progress stream" },
          }),
        );

        ws.onMessage((message) => {
          const text = typeof message === "string" ? message : String(message);
          let parsed: { type?: string; track_ids?: string[] } = {};
          try {
            parsed = JSON.parse(text);
          } catch {
            return;
          }
          if (parsed.type === "subscribe") {
            ws.send(
              JSON.stringify({
                type: "SubscribedAck",
                data: {
                  accepted: parsed.track_ids ?? [TRACK_ID],
                  requested: parsed.track_ids?.length ?? 1,
                },
              }),
            );
            ws.send(
              JSON.stringify({
                type: "StageTransition",
                data: {
                  document_id: DOC_ID,
                  task_id: TRACK_ID,
                  stage: "extracting",
                  stage_message: "Extracting entities",
                  stage_progress: 0.55,
                },
              }),
            );
          }
          if (parsed.type === "ping") {
            ws.send(
              JSON.stringify({
                type: "Heartbeat",
                data: { timestamp: new Date().toISOString() },
              }),
            );
          }
        });
      },
    );
  };

  return { routed, register };
}

test.describe("SPEC-149 real-time updates", () => {
  test("E-149-01/02 WS token + StageTransition patches row without list refetch", async ({
    page,
  }) => {
    test.setTimeout(90_000);
    await seedAuthenticatedSession(page);
    await stubBootstrapApis(page);

    let documentsGets = 0;
    await page.route("**/api/v1/documents**", async (route) => {
      if (route.request().method() !== "GET") {
        await route.fulfill({ status: 200, body: "{}" });
        return;
      }
      documentsGets += 1;
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          items: [
            {
              id: DOC_ID,
              title: "spec149.pdf",
              status: "processing",
              track_id: TRACK_ID,
              current_stage: "chunking",
              display_status: "chunking",
              stage_message: "Chunking document",
              ui_phase: "running",
              entities_count: 0,
              created_at: new Date().toISOString(),
              updated_at: new Date().toISOString(),
            },
          ],
          documents: [
            {
              id: DOC_ID,
              title: "spec149.pdf",
              status: "processing",
              track_id: TRACK_ID,
              current_stage: "chunking",
              display_status: "chunking",
              stage_message: "Chunking document",
              ui_phase: "running",
              entities_count: 0,
              created_at: new Date().toISOString(),
              updated_at: new Date().toISOString(),
            },
          ],
          total: 1,
          page: 1,
          page_size: 50,
          status_counts: { processing: 1 },
        }),
      });
    });

    await page.route("**/api/v1/**", async (route) => {
      const url = route.request().url();
      if (
        url.includes("/documents") ||
        url.includes("/tenants") ||
        url.includes("/workspaces") ||
        url.includes("/auth") ||
        url.includes("/pipeline") ||
        url.includes("/tasks")
      ) {
        await route.fallback();
        return;
      }
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: "{}",
      });
    });

    // Register the mock fully before navigation (await routeWebSocket).
    const mock = attachProgressWebSocketMock(page);
    await mock.register();

    await page.goto("/documents", { waitUntil: "domcontentloaded" });
    // Do not expand Runs here — idle expand can burn the WS handshake budget.

    const routed = await Promise.race([
      mock.routed,
      page
        .waitForTimeout(45_000)
        .then(() => Promise.reject(new Error("WS route never matched"))),
    ]);

    expect(routed.url).not.toContain("token=");
    // SPEC-154: access token must not appear in the WS URL (protocol carries JWT).
    expect(routed.url).not.toContain(FAKE_TOKEN);

    await expect(page.getByText("spec149.pdf").first()).toBeVisible({
      timeout: 20_000,
    });

    const getsAfterVisible = documentsGets;

    await expect
      .poll(
        async () => {
          const body = await page.locator("body").innerText();
          return /extracting|Extracting entities/i.test(body);
        },
        { timeout: 15_000, message: "StageTransition should update UI copy" },
      )
      .toBe(true);

    // No additional documents list GET required for the push update.
    expect(documentsGets).toBe(getsAfterVisible);
  });

  test("E-149-01 no anonymous WS handshake on login", async ({ page }) => {
    await forceRuntimeConfig(page, {
      apiUrl: "",
      authEnabled: true,
      disableDemoLogin: true,
      healthPollIntervalMs: false,
    });
    await page.addInitScript(() => {
      localStorage.clear();
    });
    await stubBootstrapApis(page);
    await page.route("**/api/v1/**", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ needs_setup: false, auth_enabled: true }),
      }),
    );

    const sockets: string[] = [];
    page.on("websocket", (ws) => {
      if (ws.url().includes("/ws/pipeline/progress")) {
        sockets.push(ws.url());
      }
    });

    // Also catch mocked routes if any stray connect occurs.
    let routedAnonymous = false;
    await page.routeWebSocket(
      (url) => url.toString().includes("/ws/pipeline/progress"),
      () => {
        routedAnonymous = true;
      },
    );

    await page.goto("/login", { waitUntil: "domcontentloaded" });
    await expect(page.locator("input#username")).toBeVisible({
      timeout: 10_000,
    });
    await page.waitForTimeout(1500);
    expect(sockets).toHaveLength(0);
    expect(routedAnonymous).toBe(false);
  });
});
