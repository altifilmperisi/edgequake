import type { QueryContext } from '@/types';

import type { DocumentTitleLabels } from './types';

type Chunks = NonNullable<QueryContext['chunks']>;

const DEFAULT_TITLE_LABELS: DocumentTitleLabels = {
  untitled: 'Untitled',
  untitledDocument: 'Untitled Document',
};

export function stripMarkdownSyntax(text: string | null | undefined): string {
  return (text ?? '')
    .replace(/^#{1,6}\s+/gm, '')
    .replace(/(\*{1,3}|_{1,3})(.+?)\1/g, '$2')
    .replace(/`([^`]+)`/g, '$1')
    .replace(/\[([^\]]+)\]\([^)]*\)/g, '$1')
    .replace(/!\[([^\]]*)\]\([^)]*\)/g, '$1')
    .replace(/\[\^[^\]]+\]/g, '')
    .replace(/^[-*_]{3,}\s*$/gm, '')
    .replace(/(\*{1,3}|_{1,3}|~~)(?=\S)/g, '')
    .replace(/^\s+/gm, '')
    .trim();
}

export function formatPassagePreview(
  rawContent: string | null | undefined,
  fullChunkContent: boolean,
): string {
  // Persisted/legacy sources may omit `content`; never crash the sources panel.
  const content = rawContent ?? '';
  const clean = stripMarkdownSyntax(content);
  if (fullChunkContent) {
    return clean || content;
  }
  if (clean.length > 220) {
    return clean.slice(0, 220).replace(/[*_`~]+$/, '') + '…';
  }
  return clean || content.slice(0, 220);
}

export function getDocumentTitle(
  chunks: Chunks,
  labels: DocumentTitleLabels = DEFAULT_TITLE_LABELS,
): string {
  const chunk = chunks[0];
  if (!chunk) return labels.untitled;

  if (chunk.file_path) {
    const filename = chunk.file_path.split('/').pop() || '';
    const cleanName = filename.replace(/\.(md|txt|pdf|docx?|html?|rst|json|xml)$/i, '');
    if (cleanName.length > 0) {
      return cleanName.length > 50 ? cleanName.slice(0, 50) + '...' : cleanName;
    }
  }

  const body = chunk.content ?? '';
  const titleMatch = body.match(/^#+\s+(.+)$/m);
  if (titleMatch && titleMatch[1]) {
    const title = titleMatch[1].trim();
    return title.length > 50 ? title.slice(0, 50) + '...' : title;
  }

  const lines = body.split('\n').filter((line) => line.trim().length > 0);
  if (lines.length > 0) {
    const firstLine = lines[0].trim();
    if (firstLine.length > 3 && !firstLine.match(/^[-*#=]+$/)) {
      return firstLine.length > 50 ? firstLine.slice(0, 50) + '...' : firstLine;
    }
  }

  return labels.untitledDocument;
}
