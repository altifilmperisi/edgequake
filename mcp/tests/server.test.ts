/**
 * Unit tests for SPEC-152 stdio bridge registration.
 */
import { describe, expect, it } from "vitest";
import { createServer } from "../src/server.js";

describe("createServer", () => {
  it("creates server without throwing", () => {
    const server = createServer();
    expect(server).toBeTruthy();
  });

  it("bridge tool catalog constant includes L1 names", async () => {
    // Source-level contract: server.ts registers these names.
    const { readFile } = await import("node:fs/promises");
    const src = await readFile(
      new URL("../src/server.ts", import.meta.url),
      "utf8",
    );
    for (const name of [
      "eq_document_list",
      "eq_search",
      "eq_fetch",
      "eq_retrieve",
      "eq_entity_search",
      "eq_neighborhood",
      "eq_workspace_list",
      "eq_workspace_stats",
      "eq_document_get",
      "eq_entity_get",
    ]) {
      expect(src).toContain(`"${name}"`);
    }
    expect(src).not.toContain('registerBridgeTool(\n    server,\n    "query"');
  });
});
