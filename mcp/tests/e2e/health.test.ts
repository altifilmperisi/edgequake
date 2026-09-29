/**
 * E2E: eq_document_list via stdio bridge (replaces legacy health tool).
 */
import type { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { callTool, createTestClient, isServerRunning } from "./helpers.js";

describe("eq_document_list tool (e2e)", () => {
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

  it("lists documents with ok envelope summary", async () => {
    if (!serverUp) return;
    const result = await callTool(client, "eq_document_list", {});
    if (typeof result === "string") {
      expect(result.startsWith("ok ") || result.includes("document")).toBe(true);
    } else {
      const obj = result as Record<string, unknown>;
      expect(obj.ok === true || Array.isArray(obj.documents)).toBe(true);
    }
  });
});
