import type { QueryContext } from "@/types";
import type { CitationResolver } from "@/components/query/markdown/citation-resolver";
import { getDocumentTitle } from "./passage-text";

/** Build a markdown citation resolver from query context chunks. */
export function buildCitationResolver(
  context?: QueryContext | null,
): CitationResolver | undefined {
  const chunks = context?.chunks;
  if (!chunks?.length) return undefined;

  // Raw retrieval scores can exceed 1 (RRF / rerank); normalise like the Docs tab.
  const scoreCeiling = Math.max(1, ...chunks.map((c) => c.score ?? 0));

  return (sourceId: string) => {
    const asIndex = Number.parseInt(sourceId, 10);
    const chunk = Number.isFinite(asIndex)
      ? chunks[asIndex - 1]
      : chunks.find(
          (c) =>
            c.chunk_id === sourceId ||
            String(c.reference_id) === sourceId ||
            c.document_id === sourceId,
        );

    if (!chunk?.document_id) return null;

    const index = Number.isFinite(asIndex)
      ? asIndex
      : chunks.indexOf(chunk) + 1;

    return {
      index,
      chunk: {
        content: chunk.content ?? "",
        document_id: chunk.document_id,
        score: Math.min(1, (chunk.score ?? 0) / scoreCeiling),
        chunk_id: chunk.chunk_id,
        file_path: chunk.file_path,
        title: getDocumentTitle([chunk]),
        page_start: chunk.page_start,
        page_end: chunk.page_end,
        start_line: chunk.start_line,
        end_line: chunk.end_line,
      },
    };
  };
}
