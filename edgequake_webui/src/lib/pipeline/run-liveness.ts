/**
 * SPEC-155 — honest run liveness (pure).
 *
 * A document can sit on a "processing" status long after its worker died
 * (restart, purged task, seeded fixture). Painting that as a live run with a
 * progress bar lies to the user. This module answers one question from one
 * signal: how long has the server been silent about this document?
 *
 * WHY a single age threshold (no queue-coverage input): healthy stages write
 * progress at least every few minutes (server vision watchdog: 5 min, recover
 * -stuck default: 10 min). 15 min of silence is therefore "dead", regardless
 * of what other workers are doing — and keeps every surface consistent.
 */

export const STALL_AFTER_MS = 15 * 60_000;

/** Milliseconds since the last server write, or null when unknown. */
export function silentForMs(
  updatedAt: string | null | undefined,
  now: number = Date.now(),
): number | null {
  if (!updatedAt) return null;
  const parsed = Date.parse(updatedAt);
  if (Number.isNaN(parsed)) return null;
  return Math.max(0, now - parsed);
}

/**
 * Silence duration when the run counts as stalled, else null.
 * Unknown timestamps are never reported as stalled (don't claim what we can't know).
 */
export function stalledForMs(
  updatedAt: string | null | undefined,
  now: number = Date.now(),
): number | null {
  const silent = silentForMs(updatedAt, now);
  return silent !== null && silent >= STALL_AFTER_MS ? silent : null;
}

const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

function plural(n: number, unit: string): string {
  return `${n} ${unit}${n === 1 ? "" : "s"}`;
}

/** "42 minutes", "3 hours", "3 days" — coarse on purpose. */
export function formatSilence(ms: number): string {
  if (ms >= DAY) return plural(Math.floor(ms / DAY), "day");
  if (ms >= HOUR) return plural(Math.floor(ms / HOUR), "hour");
  return plural(Math.max(1, Math.floor(ms / MINUTE)), "minute");
}
