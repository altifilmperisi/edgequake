/**
 * Remember what had focus when a surface opened and give it back on close
 * (SPEC-157 LAW-157-7 / EC-157-24). Falls back when the opener is gone.
 */
"use client";

import { useEffect, useRef } from "react";

export function useReturnFocus(isOpen: boolean, fallback?: () => void) {
  const opener = useRef<HTMLElement | null>(null);
  const wasOpen = useRef(false);
  const fallbackRef = useRef(fallback);

  useEffect(() => {
    fallbackRef.current = fallback;
  });

  useEffect(() => {
    if (isOpen && !wasOpen.current) {
      const active = document.activeElement;
      opener.current =
        active instanceof HTMLElement && active !== document.body ? active : null;
    }
    if (!isOpen && wasOpen.current) {
      const el = opener.current;
      if (el?.isConnected) el.focus();
      else fallbackRef.current?.();
      opener.current = null;
    }
    wasOpen.current = isOpen;
  }, [isOpen]);
}
