"use client";

/**
 * IME-safe composer keyboard helpers (SPEC-155 Q02).
 * Safari fires compositionend before confirming Enter — guard that race.
 */
import { useRef, useCallback } from "react";

const IME_RACE_MS = 50;

export function useComposerKeys() {
  const composingRef = useRef(false);
  const compositionEndedAtRef = useRef(0);

  const onCompositionStart = useCallback(() => {
    composingRef.current = true;
  }, []);

  const onCompositionEnd = useCallback(() => {
    composingRef.current = false;
    compositionEndedAtRef.current = performance.now();
  }, []);

  const isImeBlocked = useCallback((native: KeyboardEvent) => {
    return (
      native.isComposing ||
      // eslint-disable-next-line @typescript-eslint/no-deprecated -- IME keyCode 229
      native.keyCode === 229 ||
      composingRef.current ||
      performance.now() - compositionEndedAtRef.current < IME_RACE_MS
    );
  }, []);

  return { onCompositionStart, onCompositionEnd, isImeBlocked };
}
