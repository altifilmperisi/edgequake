/**
 * SSOT for WebUI `/live`+`/health` refetch intervals.
 *
 * Default is off (one probe on mount). Opt in with EDGEQUAKE_HEALTH_POLL_MS.
 * Playwright/Cypress always disable the loop.
 *
 * GH-400: while readiness is `degraded` (header Busy), poll until `/health`
 * returns healthy so the Busy pill clears without a full page reload.
 */

import { getRuntimeConfig } from "@/lib/runtime-config";
import { isAutomatedBrowser } from "@/lib/runtime/browser-detection";
import type { BackendReadinessState } from "@/lib/api/backend-readiness";

export const HEALTH_DETAILS_MIN_MS = 30_000;

/** Poll interval while `/health` reports degraded (GH-400 Busy pill). */
export const DEGRADED_HEALTH_POLL_MS = 5_000;

export function resolveHealthPollIntervals(
  configuredMs: number | false,
  isAutomated: boolean,
): { backendReady: number | false; healthDetails: number | false } {
  if (isAutomated || configuredMs === false) {
    return { backendReady: false, healthDetails: false };
  }
  return {
    backendReady: configuredMs,
    healthDetails: Math.max(configuredMs, HEALTH_DETAILS_MIN_MS),
  };
}

export function getBackendReadyRefetchInterval(): number | false {
  return resolveHealthPollIntervals(
    getRuntimeConfig().healthPollIntervalMs,
    isAutomatedBrowser(),
  ).backendReady;
}

/**
 * Refetch interval for the `['backend-ready']` query.
 *
 * - Automated browsers: always off
 * - `degraded`: poll every {@link DEGRADED_HEALTH_POLL_MS} until healthy (GH-400)
 * - otherwise: honor `EDGEQUAKE_HEALTH_POLL_MS` (default off)
 */
export function getBackendReadyRefetchIntervalForState(
  state: BackendReadinessState | undefined,
): number | false {
  if (isAutomatedBrowser()) {
    return false;
  }
  if (state === "degraded") {
    return DEGRADED_HEALTH_POLL_MS;
  }
  return getBackendReadyRefetchInterval();
}

export function getHealthDetailsRefetchInterval(): number | false {
  return resolveHealthPollIntervals(
    getRuntimeConfig().healthPollIntervalMs,
    isAutomatedBrowser(),
  ).healthDetails;
}

/**
 * Health-details poll: keep probing while the process is reachable but
 * degraded so SystemStatus can flip off Busy without a reload (GH-400).
 */
export function getHealthDetailsRefetchIntervalForState(
  state: BackendReadinessState | undefined,
): number | false {
  if (isAutomatedBrowser()) {
    return false;
  }
  if (state === "degraded") {
    return Math.max(DEGRADED_HEALTH_POLL_MS, HEALTH_DETAILS_MIN_MS);
  }
  return getHealthDetailsRefetchInterval();
}
