import { useCallback, useEffect, useRef, useState } from "react";

const OPEN_DELAY_MS = 120;
const CLOSE_DELAY_MS = 160;

/**
 * Open/close with intent delays so the pointer can travel from the trigger to the
 * floating card (and back) without the card flickering shut.
 */
export function useHoverIntent() {
  const [open, setOpenState] = useState(false);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const clear = useCallback(() => {
    if (timer.current) clearTimeout(timer.current);
    timer.current = null;
  }, []);

  const schedule = useCallback(
    (next: boolean, delay: number) => {
      clear();
      timer.current = setTimeout(() => setOpenState(next), delay);
    },
    [clear],
  );

  const show = useCallback(() => schedule(true, OPEN_DELAY_MS), [schedule]);
  const hide = useCallback(() => schedule(false, CLOSE_DELAY_MS), [schedule]);
  const setOpen = useCallback(
    (next: boolean) => {
      clear();
      setOpenState(next);
    },
    [clear],
  );

  useEffect(() => clear, [clear]);

  return { open, show, hide, setOpen };
}
