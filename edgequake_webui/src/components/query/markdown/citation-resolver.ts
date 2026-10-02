/** Chunk data a citation chip needs to preview AND link to its source. */
export type CitationChunk = {
  content: string;
  document_id: string;
  /** Normalised to 0..1 (never > 100%). */
  score: number;
  chunk_id?: string;
  file_path?: string;
  /** Human-readable document title derived from the file name / content. */
  title?: string;
  page_start?: number;
  page_end?: number;
  start_line?: number;
  end_line?: number;
};

export type CitationResolver = (
  sourceId: string,
) => { index: number; chunk: CitationChunk } | null;
