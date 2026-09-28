import { describe, expect, it } from "bun:test";
import {
  DEGRADED_HEALTH_POLL_MS,
  HEALTH_DETAILS_MIN_MS,
  getBackendReadyRefetchIntervalForState,
  getHealthDetailsRefetchIntervalForState,
  resolveHealthPollIntervals,
} from "../health-poll";

describe("resolveHealthPollIntervals", () => {
  it("disables both intervals by default (configured false)", () => {
    expect(resolveHealthPollIntervals(false, false)).toEqual({
      backendReady: false,
      healthDetails: false,
    });
  });

  it("uses the configured interval for backend-ready and a 30s floor for details", () => {
    expect(resolveHealthPollIntervals(10_000, false)).toEqual({
      backendReady: 10_000,
      healthDetails: HEALTH_DETAILS_MIN_MS,
    });
  });

  it("raises health details to match a slower configured interval", () => {
    expect(resolveHealthPollIntervals(60_000, false)).toEqual({
      backendReady: 60_000,
      healthDetails: 60_000,
    });
  });

  it("keeps both intervals off under Playwright even when configured", () => {
    expect(resolveHealthPollIntervals(10_000, true)).toEqual({
      backendReady: false,
      healthDetails: false,
    });
  });
});

describe("GH-400 degraded health poll", () => {
  it("polls backend-ready while degraded so Busy can clear", () => {
    expect(getBackendReadyRefetchIntervalForState("degraded")).toBe(
      DEGRADED_HEALTH_POLL_MS,
    );
  });

  it("does not force-poll when ready (honors default-off config)", () => {
    // Default config is off; ready must not invent a poll loop.
    expect(getBackendReadyRefetchIntervalForState("ready")).toBe(false);
  });

  it("keeps health details on a floor while degraded", () => {
    expect(getHealthDetailsRefetchIntervalForState("degraded")).toBe(
      Math.max(DEGRADED_HEALTH_POLL_MS, HEALTH_DETAILS_MIN_MS),
    );
  });
});
