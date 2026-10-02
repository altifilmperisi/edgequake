/**
 * Locale-aware relative date for conversation history rows (SPEC-155 W7Q DRY).
 * Replaces three copy-pasted blocks in history v1/v2/mobile.
 */

export type ConversationDateLabels = {
  yesterday: string;
};

const DEFAULT_LABELS: ConversationDateLabels = {
  yesterday: "Yesterday",
};

/**
 * Format an ISO / epoch-ms date for history list rows.
 * - < 24h → local time (HH:MM)
 * - yesterday → translated "Yesterday"
 * - else → short date (Intl)
 */
export function formatConversationDate(
  dateInput: string | number | Date,
  locale = "en",
  labels: ConversationDateLabels = DEFAULT_LABELS,
): string {
  const date =
    dateInput instanceof Date
      ? dateInput
      : typeof dateInput === "number"
        ? new Date(dateInput)
        : new Date(dateInput);

  if (Number.isNaN(date.getTime())) return "";

  const now = new Date();
  const startOfToday = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  const dayMs = 24 * 60 * 60 * 1000;
  const startOfYesterday = new Date(startOfToday.getTime() - dayMs);

  if (date >= startOfToday) {
    return date.toLocaleTimeString(locale, {
      hour: "2-digit",
      minute: "2-digit",
    });
  }

  if (date >= startOfYesterday && date < startOfToday) {
    return labels.yesterday;
  }

  return date.toLocaleDateString(locale, {
    month: "short",
    day: "numeric",
    year: date.getFullYear() !== now.getFullYear() ? "numeric" : undefined,
  });
}

/** Bucket a date into history group keys. */
export type HistoryDateGroup = "today" | "yesterday" | "last7" | "older";

export function historyDateGroup(
  dateInput: string | number | Date,
  now = new Date(),
): HistoryDateGroup {
  const date =
    dateInput instanceof Date
      ? dateInput
      : typeof dateInput === "number"
        ? new Date(dateInput)
        : new Date(dateInput);

  if (Number.isNaN(date.getTime())) return "older";

  const startOfToday = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  const dayMs = 24 * 60 * 60 * 1000;
  const startOfYesterday = new Date(startOfToday.getTime() - dayMs);
  const startOfLast7 = new Date(startOfToday.getTime() - 7 * dayMs);

  if (date >= startOfToday) return "today";
  if (date >= startOfYesterday) return "yesterday";
  if (date >= startOfLast7) return "last7";
  return "older";
}
