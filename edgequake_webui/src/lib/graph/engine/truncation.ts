import type { TruncationInput, TruncationResult } from "./types";

/**
 * Honour server totals / is_truncated. Never invert "got fewer than max"
 * into truncation (F-155-G03).
 */
export function resolveTruncationInfo(input: TruncationInput): TruncationResult {
  const totalNodes = input.totalNodes ?? input.streamedNodes;
  const totalEdges = input.totalEdges ?? input.streamedEdges;

  const isTruncated =
    typeof input.isTruncated === "boolean"
      ? input.isTruncated
      : totalNodes > input.streamedNodes || totalEdges > input.streamedEdges;

  return {
    isTruncated,
    totalNodes,
    totalEdges,
  };
}
