/**
 * SPEC-157 — ONE intent for "show this answer on the graph" (LAW-157-4/10).
 *
 * Records the answer subgraph in the SSOT store (so Graph Studio can focus
 * it), then docks the Graph pane on /query or navigates to Studio.
 */
"use client";

import { canDockCompanion } from "@/hooks/use-open-source";
import { subgraphFromContext } from "@/lib/query/answer-graph";
import { useAnswerGraphStore } from "@/stores/use-answer-graph-store";
import { useCompanionPaneStore } from "@/stores/use-companion-pane-store";
import type { QueryContext } from "@/types/query";
import { usePathname, useRouter } from "next/navigation";
import { useCallback } from "react";

/** Graph Studio deep link focusing a recorded answer (LAW-155-11). */
export function studioAnswerHref(messageId: string): string {
  return `/graph?answerMessage=${encodeURIComponent(messageId)}&focus=answer`;
}

export function useOpenAnswerGraph() {
  const router = useRouter();
  const pathname = usePathname();

  return useCallback(
    (messageId: string, context: QueryContext | undefined) => {
      useAnswerGraphStore
        .getState()
        .recordSubgraph(messageId, subgraphFromContext(context));
      if (canDockCompanion(pathname)) {
        useCompanionPaneStore.getState().openGraph(messageId);
        return;
      }
      router.push(studioAnswerHref(messageId));
    },
    [pathname, router],
  );
}
