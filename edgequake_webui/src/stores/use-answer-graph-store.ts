/**
 * Answer-on-graph store (SPEC-155 W6 / LAW-155-11, SSOT per SPEC-157 LAW-157-10).
 * Holds the chat answer subgraphs so Graph Studio can highlight them. The
 * Query companion pane does NOT depend on this store (it reads the message).
 */
import { create } from "zustand";
import type { SubgraphBundle } from "@/lib/utils/subgraph-types";
import { mapSubgraphToAnswerFocus } from "@/lib/query/answer-graph";

export interface AnswerGraphEntry {
  messageId: string;
  /** Resolved graph node ids (prefer graph_node_id / node_id). */
  nodeIds: string[];
  /** Entity names for fallback label match. */
  entityNames: string[];
  subgraph?: SubgraphBundle;
  createdAt: number;
}

interface AnswerGraphState {
  lastAnswer: AnswerGraphEntry | null;
  byMessageId: Record<string, AnswerGraphEntry>;
  setAnswerSubgraph: (entry: Omit<AnswerGraphEntry, "createdAt">) => void;
  /** Map a subgraph to focus ids and store it (single write path). */
  recordSubgraph: (messageId: string, subgraph: SubgraphBundle) => void;
  getAnswer: (messageId: string) => AnswerGraphEntry | null;
  clear: () => void;
}

export const useAnswerGraphStore = create<AnswerGraphState>((set, get) => ({
  lastAnswer: null,
  byMessageId: {},
  setAnswerSubgraph: (entry) => {
    const full: AnswerGraphEntry = { ...entry, createdAt: Date.now() };
    set((state) => ({
      lastAnswer: full,
      byMessageId: { ...state.byMessageId, [entry.messageId]: full },
    }));
  },
  recordSubgraph: (messageId, subgraph) => {
    const { nodeIds, entityNames } = mapSubgraphToAnswerFocus(subgraph);
    get().setAnswerSubgraph({ messageId, nodeIds, entityNames, subgraph });
  },
  getAnswer: (messageId) => get().byMessageId[messageId] ?? null,
  clear: () => set({ lastAnswer: null, byMessageId: {} }),
}));
