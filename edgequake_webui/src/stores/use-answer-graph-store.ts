/**
 * Answer-on-graph store (SPEC-155 W6 / LAW-155-11).
 * Holds the last chat answer subgraph so Graph Studio can highlight it.
 */
import { create } from "zustand";
import type { SubgraphBundle } from "@/lib/utils/subgraph-types";

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
  getAnswer: (messageId: string) => AnswerGraphEntry | null;
  clear: () => void;
}

/** Map API subgraph entities → graph node ids + name fallbacks. */
export function mapSubgraphToAnswerFocus(subgraph: SubgraphBundle): {
  nodeIds: string[];
  entityNames: string[];
} {
  const nodeIds: string[] = [];
  const entityNames: string[] = [];
  for (const e of subgraph.entities ?? []) {
    const graphId = e.graph_node_id || e.node_id || "";
    if (graphId) nodeIds.push(graphId);
    if (e.name) entityNames.push(e.name);
    // Also keep API entity id as a weak candidate
    if (e.id && !graphId) entityNames.push(e.id);
  }
  return { nodeIds: [...new Set(nodeIds)], entityNames: [...new Set(entityNames)] };
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
  getAnswer: (messageId) => get().byMessageId[messageId] ?? null,
  clear: () => set({ lastAnswer: null, byMessageId: {} }),
}));
