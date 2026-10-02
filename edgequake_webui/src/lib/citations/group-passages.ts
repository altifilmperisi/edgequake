import type { QueryContext } from '@/types';

export type Chunks = NonNullable<QueryContext['chunks']>;

export function groupPassagesByPage(chunks: Chunks): Map<number | null, Chunks> | null {
  const hasPages = chunks.some((c) => c.page_start !== undefined);
  if (!hasPages) return null;

  const map = new Map<number | null, Chunks>();
  for (const chunk of chunks) {
    const key = chunk.page_start ?? null;
    const list = map.get(key) ?? [];
    list.push(chunk);
    map.set(key, list);
  }
  return map;
}

export function chunksByDocument(
  chunks: QueryContext['chunks'] | undefined,
): Record<string, Chunks> {
  if (!chunks?.length) return {};

  return chunks.reduce(
    (acc, chunk) => {
      if (!acc[chunk.document_id]) {
        acc[chunk.document_id] = [];
      }
      acc[chunk.document_id].push(chunk);
      return acc;
    },
    {} as Record<string, Chunks>,
  );
}

export function normalizeChunkScores(chunksByDoc: Record<string, Chunks>): (score: number) => number {
  const allRawScores = Object.values(chunksByDoc).flatMap((chunks) =>
    chunks.map((c) => c.score),
  );
  const scoreNormalizer = allRawScores.length > 0 ? Math.max(1.0, ...allRawScores) : 1.0;
  return (score: number): number => Math.min(1.0, score / scoreNormalizer);
}
