/**
 * SPEC-157 — kill switch for the Query companion pane (rollback in one flip).
 *
 * Off when `NEXT_PUBLIC_QUERY_COMPANION=0` (build-time) or localStorage
 * `edgequake.query.companion.enabled` is "0" (per-browser, e2e-friendly).
 * When off, citations and "Show on graph" navigate exactly as before SPEC-157.
 */
export const COMPANION_FLAG_KEY = "edgequake.query.companion.enabled";

export function isCompanionEnabled(): boolean {
  if (process.env.NEXT_PUBLIC_QUERY_COMPANION === "0") return false;
  if (typeof window === "undefined") return true;
  try {
    return window.localStorage.getItem(COMPANION_FLAG_KEY) !== "0";
  } catch {
    return true;
  }
}
