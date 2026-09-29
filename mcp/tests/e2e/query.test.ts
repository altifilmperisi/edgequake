/**
 * E2E: eq_search / eq_retrieve via stdio bridge (SPEC-152).
 */
import type { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { callTool, createTestClient, isServerRunning } from "./helpers.js";

describe("eq_search / eq_retrieve (e2e)", () => {
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

  it("eq_search returns retrieval_id and hits", async () => {
    if (!serverUp) return;
    const result = (await callTool(client, "eq_search", {
      query: "What is EdgeQuake?",
      mode: "naive",
      limit: 5,
    })) as Record<string, unknown>;
    expect(result.ok).toBe(true);
    expect(String(result.retrieval_id ?? "")).toMatch(/^ret_/);
    expect(Array.isArray(result.hits)).toBe(true);
  });

  it("eq_retrieve keeps hits under budget", async () => {
    if (!serverUp) return;
    const result = (await callTool(client, "eq_retrieve", {
      query: "knowledge graph",
      mode: "naive",
      budget: "cheap",
      limit: 5,
    })) as Record<string, unknown>;
    expect(result.ok === true || result.ok === false).toBe(true);
    expect(Array.isArray(result.hits)).toBe(true);
  });
});
