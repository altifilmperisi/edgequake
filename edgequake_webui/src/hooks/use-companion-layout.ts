/**
 * SPEC-157 — resolve how the Query row is laid out (chat | companion | history)
 * from the measured container, the history preference and the stored width.
 */
"use client";

import { useElementWidth } from "@/hooks/use-element-width";
import {
  resolveCompanionLayout,
  type CompanionLayout,
  type HistoryPresentation,
} from "@/lib/query/companion-layout";
import { useCompanionPaneStore } from "@/stores/use-companion-pane-store";
import { useQueryUIStore } from "@/stores/use-query-ui-store";

export function useCompanionLayout(historyDocked: boolean): {
  rootRef: (node: HTMLDivElement | null) => void;
  layout: CompanionLayout;
  /** How history is shown right now (companion budget applies only when open). */
  history: HistoryPresentation;
  companionOpen: boolean;
} {
  const [rootRef, containerWidth] = useElementWidth<HTMLDivElement>();
  const companionOpen = useCompanionPaneStore((s) => s.target.kind !== "none");
  const preferredWidth = useCompanionPaneStore((s) => s.width);
  const historyOpen = useQueryUIStore((s) => s.historyPanelOpen);

  const layout = resolveCompanionLayout({
    containerWidth,
    historyDocked,
    historyOpen,
    preferredWidth,
  });
  const history: HistoryPresentation = companionOpen
    ? layout.history
    : !historyDocked
      ? "overlay"
      : historyOpen
        ? "docked"
        : "rail";
  return { rootRef, layout, history, companionOpen };
}
