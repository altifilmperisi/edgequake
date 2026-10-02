/**
 * SPEC-157 W1 — two-way sync between the companion store and the URL.
 *
 * - URL → store: mount (deep link / reload) and Back/Forward.
 * - store → URL: push when a pane first opens, replace otherwise.
 *
 * Loop safety: URLs we wrote ourselves are tracked in `inflight` and ignored
 * once when they land, so rapid successive opens never bounce the store back.
 */
"use client";

import {
  CLOSED_TARGET,
  decodeCompanionSearch,
  encodeCompanionSearch,
  targetKey,
} from "@/lib/query/companion-pane";
import { isCompanionEnabled } from "@/lib/query/companion-flag";
import { useCompanionPaneStore } from "@/stores/use-companion-pane-store";
import { usePathname, useRouter, useSearchParams } from "next/navigation";
import { useEffect, useRef } from "react";

export function CompanionUrlSync() {
  const router = useRouter();
  const pathname = usePathname();
  const searchParams = useSearchParams();
  const target = useCompanionPaneStore((s) => s.target);
  const applyTarget = useCompanionPaneStore((s) => s.applyTarget);

  const inflight = useRef(new Set<string>());
  // Key of the URL as it will be once pending router writes land. Comparing
  // against it (not the rendered URL) keeps a quick open→close from racing.
  const urlIntent = useRef<string | null>(null);
  // Last store key we reacted to. Seeded with the mount value so neither the
  // first run nor React StrictMode's replayed effect can wipe a deep link.
  const seenStoreKey = useRef<string | null>(null);

  const urlTarget = isCompanionEnabled()
    ? decodeCompanionSearch(searchParams)
    : CLOSED_TARGET;
  const urlKey = targetKey(urlTarget);
  const storeKey = targetKey(target);

  // URL → store
  useEffect(() => {
    if (urlIntent.current === null) urlIntent.current = urlKey;
    if (inflight.current.delete(urlKey)) return;
    urlIntent.current = urlKey;
    if (urlKey !== targetKey(useCompanionPaneStore.getState().target)) {
      applyTarget(urlTarget);
    }
    // urlTarget is derived from urlKey; keying on the string avoids loops.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [urlKey, applyTarget]);

  // store → URL (only when the store itself changed after mount)
  useEffect(() => {
    if (seenStoreKey.current === null) seenStoreKey.current = storeKey;
    if (seenStoreKey.current === storeKey) return;
    seenStoreKey.current = storeKey;
    if (storeKey === urlIntent.current) return;
    urlIntent.current = storeKey;
    const next = encodeCompanionSearch(target, searchParams).toString();
    const href = next ? `${pathname}?${next}` : pathname;
    inflight.current.add(storeKey);
    const wasClosed = urlTarget.kind === "none";
    if (wasClosed && target.kind !== "none") {
      router.push(href, { scroll: false });
    } else {
      router.replace(href, { scroll: false });
    }
    // Only react to store changes.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [storeKey]);

  return null;
}
