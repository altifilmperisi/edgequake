/**
 * SPEC-157 — ONE intent for "show me this source" (LAW-157-4).
 *
 * On /query (flag on) the document opens in the companion pane beside the
 * chat. Anywhere else, or with the flag off, it navigates to the full-page
 * viewer exactly as before — so every caller stays a one-liner.
 */
"use client";

import { isCompanionEnabled } from "@/lib/query/companion-flag";
import {
  locationToDocumentHref,
  type SourceLocation,
} from "@/lib/query/companion-pane";
import { useCompanionPaneStore } from "@/stores/use-companion-pane-store";
import { usePathname, useRouter } from "next/navigation";
import { useCallback } from "react";

export type OpenOutcome = "docked" | "navigated";

/** Whether a companion pane can be hosted on this route. */
export function canDockCompanion(pathname: string | null): boolean {
  return pathname === "/query" && isCompanionEnabled();
}

export function useOpenSource() {
  const router = useRouter();
  const pathname = usePathname();

  return useCallback(
    (location: SourceLocation): OpenOutcome => {
      if (canDockCompanion(pathname)) {
        useCompanionPaneStore.getState().openSource(location);
        return "docked";
      }
      router.push(locationToDocumentHref(location));
      return "navigated";
    },
    [pathname, router],
  );
}
