"use client";

import { useEffect, type RefObject } from "react";

function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  const tag = target.tagName;
  if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return true;
  return target.isContentEditable;
}

interface Options {
  inputRef: RefObject<HTMLTextAreaElement | null>;
  input: string;
  /** Start an `@document` mention in the composer. */
  onStartMention: () => void;
  onOpenSlashMenu: () => void;
  onFocusComposer: () => void;
}

/**
 * Global `/` and `@` shortcuts on the query page (SPEC-155).
 * Typing inside the composer is left alone — the composer owns its own menus.
 */
export function useQueryComposerShortcuts({
  inputRef,
  input,
  onStartMention,
  onOpenSlashMenu,
  onFocusComposer,
}: Options) {
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.defaultPrevented || event.metaKey || event.ctrlKey || event.altKey) {
        return;
      }
      if (event.key !== "@" && event.key !== "/") return;

      const inComposer = document.activeElement === inputRef.current;
      if (inComposer) {
        // `/` as first char opens the slash menu; `@` is handled by the mention hook.
        return;
      }
      if (isEditableTarget(event.target)) return;

      event.preventDefault();
      onFocusComposer();
      if (event.key === "@") {
        onStartMention();
      } else if (input.length === 0) {
        onOpenSlashMenu();
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [input, inputRef, onFocusComposer, onOpenSlashMenu, onStartMention]);
}
