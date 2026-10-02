/** Track an element's content width (px) with ResizeObserver. */
"use client";

import { useLayoutEffect, useState } from "react";

export function useElementWidth<T extends HTMLElement>() {
  const [node, setNode] = useState<T | null>(null);
  const [width, setWidth] = useState(0);

  useLayoutEffect(() => {
    if (!node) return;
    // Intentional: one synchronous measure before paint, then observe.
    // eslint-disable-next-line react-hooks/set-state-in-effect
    setWidth(Math.round(node.getBoundingClientRect().width));
    const observer = new ResizeObserver((entries) => {
      const w = entries[0]?.contentRect.width;
      if (w !== undefined) setWidth(Math.round(w));
    });
    observer.observe(node);
    return () => observer.disconnect();
  }, [node]);

  return [setNode, width] as const;
}
