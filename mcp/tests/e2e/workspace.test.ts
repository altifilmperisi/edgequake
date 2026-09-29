/**
 * E2E: workspace catalog tools via stdio bridge (SPEC-152).
 */
import type { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { callTool, createTestClient, isServerRunning } from "./helpers.js";

describe("workspace tools (e2e)", () => {
  let client: Client;
  let cleanup: () => Promise<void>;
  let serverUp: boolean;

  beforeAll(async () => {
    serverUp = await isServerRunning();
    if (!serverUp) return;
    const ctx = await createTestClient();
    client = ctx.client;
    cleanup = ctx.cleanup;
  });

  afterAll(async () => {
    if (cleanup) await cleanup();
  });

  it("eq_workspace_list returns workspaces", async () => {
    if (!serverUp) return;
    const result = (await callTool(client, "eq_workspace_list", {})) as Record<
      string,
      unknown
    >;
    expect(result.ok).toBe(true);
    expect(
      Array.isArray(result.workspaces) || Array.isArray(result.items),
    ).toBe(true);
  });

  it("eq_workspace_stats returns counts", async () => {
    if (!serverUp) return;
    const result = (await callTool(client, "eq_workspace_stats", {})) as Record<
      string,
      unknown
    >;
    expect(result.ok).toBe(true);
  });
});
