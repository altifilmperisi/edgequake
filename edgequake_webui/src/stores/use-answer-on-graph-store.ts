/**
 * Answer-on-graph focus store — SPEC-155 W6 / EC-155-65.
 */
"use client";

import { create } from "zustand";

export type AnswerSubgraphEntity = {
  id?: string;
  graph_node_id?: string;
  name: string;
  entity_type?: string;
};

export type AnswerSubgraph = {
  entities: AnswerSubgraphEntity[];
  relationships?: Array<{
    source: string;
    target: string;
    relation_type?: string;
  }>;
};

interface AnswerOnGraphState {
  messageId: string | null;
  subgraph: AnswerSubgraph | null;
  setAnswerFocus: (messageId: string, subgraph: AnswerSubgraph) => void;
  clearAnswerFocus: () => void;
  /** Resolve graph node ids (prefer graph_node_id, fallback normalised name). */
  resolveNodeIds: (existingNodeIds: Set<string>) => string[];
}

function normaliseName(name: string): string {
  return name.trim().toUpperCase().replace(/\s+/g, "_");
}

export const useAnswerOnGraphStore = create<AnswerOnGraphState>((set, get) => ({
  messageId: null,
  subgraph: null,
  setAnswerFocus: (messageId, subgraph) => set({ messageId, subgraph }),
  clearAnswerFocus: () => set({ messageId: null, subgraph: null }),
  resolveNodeIds: (existingNodeIds) => {
    const sub = get().subgraph;
    if (!sub) return [];
    const ids: string[] = [];
    for (const e of sub.entities) {
      if (e.graph_node_id && existingNodeIds.has(e.graph_node_id)) {
        ids.push(e.graph_node_id);
        continue;
      }
      // Fallback: match by bare name or suffix after workspace::
      const bare = normaliseName(e.name);
      for (const nid of existingNodeIds) {
        const suffix = nid.includes("::") ? nid.split("::").pop()! : nid;
        if (suffix === bare || nid.endsWith(bare)) {
          ids.push(nid);
          break;
        }
      }
    }
    return [...new Set(ids)];
  },
}));
