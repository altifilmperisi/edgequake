export type OnDocumentClickOptions = {
  documentId: string;
  chunkContent?: string;
  chunkIndex?: number;
  startLine?: number;
  endLine?: number;
  chunkId?: string;
  page?: number;
};

export type OnDocumentClick = (options: OnDocumentClickOptions) => void;

export function invokeDocumentClick(
  handler: OnDocumentClick | undefined,
  options: OnDocumentClickOptions,
): void {
  handler?.(options);
}

export type DocumentTitleLabels = {
  untitled: string;
  untitledDocument: string;
};
