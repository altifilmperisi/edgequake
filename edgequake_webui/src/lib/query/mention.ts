/**
 * `@document` mention detection for the query composer (SPEC-155 W7Q).
 * Pure functions — the composer owns caret/text; this owns the rules.
 */

export interface MentionMatch {
  /** Index of the `@` character. */
  start: number;
  /** Caret index (exclusive end of the token). */
  end: number;
  /** Text typed after `@`. */
  query: string;
}

/** Document titles contain spaces; cap so a stray `@` doesn't stay open forever. */
export const MENTION_MAX_QUERY = 40;

/**
 * Find an active `@mention` token ending at the caret.
 * Rules: `@` must start the text or follow whitespace (so emails don't trigger),
 * the query may contain spaces but no newline, and is capped in length.
 */
export function detectMention(text: string, caret: number): MentionMatch | null {
  if (caret < 0 || caret > text.length) return null;
  const upto = text.slice(0, caret);
  const at = upto.lastIndexOf("@");
  if (at < 0) return null;
  if (at > 0 && !/\s/.test(upto[at - 1]!)) return null;
  const query = upto.slice(at + 1);
  if (query.includes("\n")) return null;
  if (query.length > MENTION_MAX_QUERY) return null;
  return { start: at, end: caret, query };
}

/** Remove the `@query` token (the doc becomes a chip, not inline text). */
export function removeMention(
  text: string,
  match: MentionMatch,
): { text: string; caret: number } {
  const before = text.slice(0, match.start);
  let after = text.slice(match.end);
  // Avoid leaving a double space where the token was.
  if (before.endsWith(" ") && after.startsWith(" ")) after = after.slice(1);
  return { text: before + after, caret: before.length };
}

/** Add an id once, preserving order. */
export function addScopedId(ids: ReadonlyArray<string>, id: string): string[] {
  return ids.includes(id) ? [...ids] : [...ids, id];
}

/** Drop an id and its cached title. */
export function removeScopedId(
  ids: ReadonlyArray<string>,
  titles: Readonly<Record<string, string>>,
  id: string,
): { ids: string[]; titles: Record<string, string> } {
  const nextTitles = { ...titles };
  delete nextTitles[id];
  return { ids: ids.filter((x) => x !== id), titles: nextTitles };
}

/* ── Menu navigation state machine ─────────────────────────────────────── */

export interface MentionNavState {
  activeIndex: number;
  /** `start` of a mention the user dismissed with Escape; stays closed until it changes. */
  dismissedStart: number | null;
}

export type MentionNavEvent =
  | { type: "move"; delta: 1 | -1; count: number }
  | { type: "set"; index: number }
  | { type: "reset" }
  | { type: "dismiss"; start: number }
  | { type: "release" };

export const initialMentionNav: MentionNavState = {
  activeIndex: 0,
  dismissedStart: null,
};

export function reduceMentionNav(
  state: MentionNavState,
  event: MentionNavEvent,
): MentionNavState {
  switch (event.type) {
    case "move": {
      if (event.count <= 0) return { ...state, activeIndex: 0 };
      const next = (state.activeIndex + event.delta + event.count) % event.count;
      return { ...state, activeIndex: next };
    }
    case "set":
      return state.activeIndex === event.index
        ? state
        : { ...state, activeIndex: Math.max(0, event.index) };
    case "reset":
      return state.activeIndex === 0 ? state : { ...state, activeIndex: 0 };
    case "dismiss":
      return { activeIndex: 0, dismissedStart: event.start };
    case "release":
      return state.dismissedStart === null
        ? state
        : { ...state, dismissedStart: null };
  }
}

/** Menu is visible when a mention exists and wasn't dismissed. */
export function isMentionOpen(
  match: MentionMatch | null,
  nav: MentionNavState,
): boolean {
  return match !== null && nav.dismissedStart !== match.start;
}
