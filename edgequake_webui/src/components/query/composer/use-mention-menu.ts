"use client";

/**
 * Wires `detectMention` + nav reducer + document search into one view-model
 * for the composer's `@` menu.
 */
import { useDocumentSearch } from "@/hooks/use-document-search";
import {
  detectMention,
  initialMentionNav,
  isMentionOpen,
  reduceMentionNav,
  removeMention,
  type MentionMatch,
} from "@/lib/query/mention";
import type { DocumentSearchItem } from "@/types";
import {
  useCallback,
  useEffect,
  useMemo,
  useReducer,
  type KeyboardEvent,
} from "react";

interface Options {
  text: string;
  caret: number;
  scopedIds: readonly string[];
  onPick: (doc: DocumentSearchItem, next: { text: string; caret: number }) => void;
}

export function useMentionMenu({ text, caret, scopedIds, onPick }: Options) {
  const [nav, dispatch] = useReducer(reduceMentionNav, initialMentionNav);
  const match: MentionMatch | null = useMemo(
    () => detectMention(text, caret),
    [text, caret],
  );
  const open = isMentionOpen(match, nav);

  const { data, isLoading } = useDocumentSearch(match?.query ?? "", open);
  const items = useMemo(
    () => data.filter((d) => !scopedIds.includes(d.id)),
    [data, scopedIds],
  );

  // Titles contain spaces; once the query has a space and nothing matches, step aside.
  const visible =
    open && (isLoading || items.length > 0 || !/\s/.test(match?.query ?? ""));

  useEffect(() => {
    dispatch({ type: "reset" });
  }, [match?.query]);

  useEffect(() => {
    if (!match) dispatch({ type: "release" });
  }, [match]);

  const pick = useCallback(
    (doc: DocumentSearchItem) => {
      if (!match) return;
      onPick(doc, removeMention(text, match));
    },
    [match, onPick, text],
  );

  /** Returns true when the key was consumed by the menu. */
  const handleKeyDown = useCallback(
    (e: KeyboardEvent<HTMLTextAreaElement>): boolean => {
      if (!visible || !match) return false;
      switch (e.key) {
        case "ArrowDown":
        case "ArrowUp":
          e.preventDefault();
          dispatch({
            type: "move",
            delta: e.key === "ArrowDown" ? 1 : -1,
            count: items.length,
          });
          return true;
        case "Enter":
        case "Tab": {
          const doc = items[nav.activeIndex];
          if (!doc) return false;
          e.preventDefault();
          pick(doc);
          return true;
        }
        case "Escape":
          e.preventDefault();
          dispatch({ type: "dismiss", start: match.start });
          return true;
        default:
          return false;
      }
    },
    [items, match, nav.activeIndex, pick, visible],
  );

  return {
    visible,
    query: match?.query ?? "",
    items,
    isLoading,
    activeIndex: Math.min(nav.activeIndex, Math.max(items.length - 1, 0)),
    setActiveIndex: (index: number) => dispatch({ type: "set", index }),
    pick,
    handleKeyDown,
  };
}
