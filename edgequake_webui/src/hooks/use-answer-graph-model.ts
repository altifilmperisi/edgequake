/**
 * SPEC-157 — Local, expandable graph model behind the companion Graph pane.
 * Starts from the answer subgraph; "expand" merges one node's neighbourhood
 * (capped) without ever touching the workspace graph store (LAW-157-9).
 */
import { getEntityNeighborhood } from "@/lib/api/edgequake";
import {
  mergeNeighborhood,
  type AnswerGraphModel,
} from "@/lib/query/answer-graph";
import { useCallback, useRef, useState } from "react";

export type ExpandOutcome = "added" | "none" | "failed";

export function useAnswerGraphModel(base: AnswerGraphModel) {
  const [model, setModel] = useState(base);
  // Latest model for merge decisions (a state updater runs lazily, so it can't
  // report whether the merge grew the graph).
  const modelRef = useRef(base);
  const [expanding, setExpanding] = useState<string | null>(null);
  const [expanded, setExpanded] = useState<ReadonlySet<string>>(new Set());

  const expand = useCallback(
    async (nodeId: string): Promise<ExpandOutcome> => {
      if (expanding) return "none";
      setExpanding(nodeId);
      try {
        const incoming = await getEntityNeighborhood(nodeId, 1);
        const next = mergeNeighborhood(modelRef.current, {
          nodes: incoming.nodes ?? [],
          edges: incoming.edges ?? [],
        });
        const grew = next !== modelRef.current;
        if (grew) {
          modelRef.current = next;
          setModel(next);
        }
        setExpanded((prev) => new Set(prev).add(nodeId));
        return grew ? "added" : "none";
      } catch {
        return "failed";
      } finally {
        setExpanding(null);
      }
    },
    [expanding],
  );

  return { model, expand, expanding, expanded };
}
