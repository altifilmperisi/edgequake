/**
 * Query companion pane store (SPEC-157).
 *
 * `target` is the single source of truth for what the pane shows; it is
 * mirrored to the URL by `CompanionUrlSync`. Only the preferred width is
 * persisted — a reload re-derives the target from the URL (LAW-157-3).
 */
import { create } from "zustand";
import { persist } from "zustand/middleware";
import {
  CLOSED_TARGET,
  type CompanionKind,
  type CompanionTarget,
  type SourceLocation,
} from "@/lib/query/companion-pane";
import { COMPANION_DEFAULT_PX } from "@/lib/query/companion-layout";

interface CompanionPaneState {
  target: CompanionTarget;
  /** Remembered so the tab switch can return to the other pane. */
  lastSource: SourceLocation | null;
  lastGraphMessageId: string | null;
  /** Bumped on every explicit open so re-clicking a citation re-navigates. */
  seq: number;
  /** Preferred side-pane width (px); clamped by the layout resolver. */
  width: number;

  openSource: (location: SourceLocation) => void;
  openGraph: (messageId: string) => void;
  close: () => void;
  /** Switch between the two panes using what was last shown. */
  switchTo: (kind: Exclude<CompanionKind, "none">) => void;
  /** URL → store (no side effects beyond remembering the targets). */
  applyTarget: (target: CompanionTarget) => void;
  setWidth: (px: number) => void;
}

function remember(
  state: Pick<CompanionPaneState, "lastSource" | "lastGraphMessageId">,
  target: CompanionTarget,
) {
  return {
    lastSource: target.kind === "pdf" ? target.source : state.lastSource,
    lastGraphMessageId:
      target.kind === "graph" ? target.messageId : state.lastGraphMessageId,
  };
}

export const useCompanionPaneStore = create<CompanionPaneState>()(
  persist(
    (set, get) => ({
      target: CLOSED_TARGET,
      lastSource: null,
      lastGraphMessageId: null,
      seq: 0,
      width: COMPANION_DEFAULT_PX,

      openSource: (location) => {
        const target: CompanionTarget = {
          kind: "pdf",
          source: location,
          messageId: null,
        };
        set((s) => ({ target, seq: s.seq + 1, ...remember(s, target) }));
      },
      openGraph: (messageId) => {
        const target: CompanionTarget = {
          kind: "graph",
          source: null,
          messageId,
        };
        set((s) => ({ target, seq: s.seq + 1, ...remember(s, target) }));
      },
      close: () => set({ target: CLOSED_TARGET }),
      switchTo: (kind) => {
        const { lastSource, lastGraphMessageId } = get();
        if (kind === "pdf" && lastSource) get().openSource(lastSource);
        if (kind === "graph" && lastGraphMessageId) {
          get().openGraph(lastGraphMessageId);
        }
      },
      applyTarget: (target) => set((s) => ({ target, ...remember(s, target) })),
      setWidth: (px) => set({ width: Math.round(px) }),
    }),
    {
      name: "edgequake.query.companion.v1",
      partialize: (s) => ({ width: s.width }),
    },
  ),
);
