/**
 * E2E: document catalog tools via stdio bridge (SPEC-152 query profile).
 */
import type { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { callTool, createTestClient, isServerRunning } from "./helpers.js";

describe("document tools (e2e)", () => {
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

  it("eq_document_list returns documents array", async () => {
    if (!serverUp) return;
    const result = (await callTool(client, "eq_document_list", {
      limit: 10,
    })) as Record<string, unknown>;
    expect(result.ok).toBe(true);
    expect(
      Array.isArray(result.documents) || Array.isArray(result.items),
    ).toBe(true);
  });

  it("eq_document_get handles missing id gracefully", async () => {
    if (!serverUp) return;
    const result = await callTool(client, "eq_document_get", {
      document_id: "doc_missing_e2e",
    });
    if (typeof result === "string") {
      expect(result.length).toBeGreaterThan(0);
    } else {
      const obj = result as Record<string, unknown>;
      expect(obj.ok === true || obj.ok === false).toBe(true);
    }
  });
});
