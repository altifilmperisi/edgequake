import { describe, expect, it } from "bun:test";
import { retryDelay, retryPolicy } from "../../providers/query-provider";

describe("GH-400 query retry for read_path_busy", () => {
  it("retries read_path_busy once", () => {
    const err = { status: 503, code: "read_path_busy", details: { retry_after_ms: 2500 } };
    expect(retryPolicy(0, err)).toBe(true);
    expect(retryPolicy(1, err)).toBe(false);
  });

  it("honors retry_after_ms within 500–8000ms", () => {
    const err = { status: 503, code: "read_path_busy", details: { retry_after_ms: 2500 } };
    expect(retryDelay(0, err)).toBe(2500);
  });

  it("clamps retry_after_ms to at least 500ms", () => {
    const err = { status: 503, code: "read_path_busy", details: { retry_after_ms: 100 } };
    expect(retryDelay(0, err)).toBe(500);
  });

  it("clamps retry_after_ms to at most 8000ms", () => {
    const err = { status: 503, code: "read_path_busy", details: { retry_after_ms: 60_000 } };
    expect(retryDelay(0, err)).toBe(8000);
  });

  it("falls back to exponential backoff without retry_after_ms", () => {
    expect(retryDelay(0, { status: 503, code: "read_path_busy" })).toBe(1000);
    expect(retryDelay(1)).toBe(2000);
  });
});
