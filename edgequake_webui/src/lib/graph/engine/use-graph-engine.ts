/**
 * useGraphEngine — create/destroy once per mount (SPEC-155 LAW-155-3 / F-155-G01).
 * Returns a stable engine instance; filters/theme must not remount.
 */
"use client";

import { useEffect, useRef, useState } from "react";
import { createGraphEngine, type GraphEngine } from "./create-engine";
import type { GraphEngineOptions } from "./types";

export function useGraphEngine(options: GraphEngineOptions = {}) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const engineRef = useRef<GraphEngine | null>(null);
  const [engine, setEngine] = useState<GraphEngine | null>(null);
  const [engineId, setEngineId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const optionsRef = useRef(options);
  optionsRef.current = options;

  useEffect(() => {
    const el = containerRef.current;
    if (!el || engineRef.current) return;

    try {
      const instance = createGraphEngine(el, {
        ...optionsRef.current,
        onWebglError: (err) => {
          setError(err.message);
          optionsRef.current.onWebglError?.(err);
        },
      });
      // If Sigma failed silently, surface error
      if (!instance.getSigma()) {
        setError("WebGL graph renderer failed to start");
      }
      engineRef.current = instance;
      setEngine(instance);
      setEngineId(instance.id);
    } catch (err) {
      const msg =
        err instanceof Error
          ? err.message
          : "WebGL is unavailable in this browser.";
      setError(msg);
      // SPEC-155 F-155-G18: never window.location
    }

    return () => {
      engineRef.current?.destroy();
      engineRef.current = null;
      setEngine(null);
      setEngineId(null);
    };
    // Intentionally once — options applied via updateOptions / setTheme
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return {
    engine,
    engineId,
    error,
    containerRef,
  };
}
