import { describe, expect, it } from "vitest";
import {
  formatSilence,
  silentForMs,
  STALL_AFTER_MS,
  stalledForMs,
} from "@/lib/pipeline/run-liveness";

const NOW = Date.parse("2026-10-01T12:00:00Z");
const ago = (ms: number) => new Date(NOW - ms).toISOString();

describe("run-liveness", () => {
  it("is live while the server wrote recently", () => {
    expect(stalledForMs(ago(60_000), NOW)).toBeNull();
    expect(stalledForMs(ago(STALL_AFTER_MS - 1), NOW)).toBeNull();
  });

  it("is stalled at and beyond the threshold", () => {
    expect(stalledForMs(ago(STALL_AFTER_MS), NOW)).toBe(STALL_AFTER_MS);
    expect(stalledForMs(ago(3 * 24 * 3_600_000), NOW)).toBe(3 * 24 * 3_600_000);
  });

  it("never claims stalled for unknown or invalid timestamps", () => {
    expect(stalledForMs(undefined, NOW)).toBeNull();
    expect(stalledForMs(null, NOW)).toBeNull();
    expect(stalledForMs("not-a-date", NOW)).toBeNull();
    expect(silentForMs("", NOW)).toBeNull();
  });

  it("clamps future timestamps (clock skew) to zero silence", () => {
    expect(silentForMs(new Date(NOW + 60_000).toISOString(), NOW)).toBe(0);
    expect(stalledForMs(new Date(NOW + 60_000).toISOString(), NOW)).toBeNull();
  });

  it("formats silence coarsely with correct plurals", () => {
    expect(formatSilence(15 * 60_000)).toBe("15 minutes");
    expect(formatSilence(60_000)).toBe("1 minute");
    expect(formatSilence(10_000)).toBe("1 minute");
    expect(formatSilence(3_600_000)).toBe("1 hour");
    expect(formatSilence(5 * 3_600_000 + 1)).toBe("5 hours");
    expect(formatSilence(24 * 3_600_000)).toBe("1 day");
    expect(formatSilence(3 * 24 * 3_600_000 + 5)).toBe("3 days");
  });
});
