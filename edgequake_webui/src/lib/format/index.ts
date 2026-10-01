/** Intl formatters — SPEC-155 DRY (no per-screen formatCost copies). */

export function formatCost(
  value: number | null | undefined,
  locale = typeof navigator !== "undefined" ? navigator.language : "en",
  currency = "USD",
): string {
  if (value == null || Number.isNaN(value)) return "—";
  return new Intl.NumberFormat(locale, {
    style: "currency",
    currency,
    maximumFractionDigits: 4,
  }).format(value);
}

export function formatNumber(
  value: number | null | undefined,
  locale = typeof navigator !== "undefined" ? navigator.language : "en",
  options?: Intl.NumberFormatOptions,
): string {
  if (value == null || Number.isNaN(value)) return "—";
  return new Intl.NumberFormat(locale, options).format(value);
}

export function formatDuration(
  ms: number | null | undefined,
  locale = typeof navigator !== "undefined" ? navigator.language : "en",
): string {
  if (ms == null || Number.isNaN(ms)) return "—";
  const seconds = ms / 1000;
  if (seconds < 60) {
    return new Intl.NumberFormat(locale, {
      style: "unit",
      unit: "second",
      maximumFractionDigits: 1,
    }).format(seconds);
  }
  const minutes = seconds / 60;
  if (minutes < 60) {
    return new Intl.NumberFormat(locale, {
      style: "unit",
      unit: "minute",
      maximumFractionDigits: 1,
    }).format(minutes);
  }
  return new Intl.NumberFormat(locale, {
    style: "unit",
    unit: "hour",
    maximumFractionDigits: 1,
  }).format(minutes / 60);
}
