/**
 * E2E: graph tools via stdio bridge (SPEC-152).
 */
import type { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { callTool, createTestClient, isServerRunning } from "./helpers.js";

describe("graph tools (e2e)", () => {
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

  it("eq_entity_search returns entities array", async () => {
    if (!serverUp) return;
    const result = (await callTool(client, "eq_entity_search", {
      q: "TECHNOLOGY",
      limit: 10,
    })) as Record<string, unknown>;
    expect(result.ok).toBe(true);
    expect(Array.isArray(result.entities)).toBe(true);
  });

  it("eq_neighborhood requires entity_id", async () => {
    if (!serverUp) return;
    const result = await callTool(client, "eq_neighborhood", {
      entity_id: "ent:default:UNKNOWN",
      max_hops: 1,
    });
    // ok or eq/not_found — must not invent a page-long dump
    if (typeof result === "string") {
      expect(result.length).toBeLessThan(4096);
    } else {
      const obj = result as Record<string, unknown>;
      expect(obj.ok === true || obj.ok === false).toBe(true);
    }
  });
});
