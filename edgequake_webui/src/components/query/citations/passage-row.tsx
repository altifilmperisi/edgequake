'use client';

import { BookOpen, ExternalLink } from 'lucide-react';
import Link from 'next/link';
import { useRouter } from 'next/navigation';
import { useTranslation } from 'react-i18next';

import { getConfidenceLabel } from '@/lib/citations/confidence';
import type { Chunks } from '@/lib/citations/group-passages';
import { formatPassagePreview, stripMarkdownSyntax } from '@/lib/citations/passage-text';
import type { OnDocumentClick } from '@/lib/citations/types';
import { invokeDocumentClick } from '@/lib/citations/types';
import { buildDocumentPageUrl, formatChunkPageBadge } from '@/lib/utils/document-url';

export interface PassageRowProps {
  chunk: Chunks[number];
  chunkIdx: number;
  docId: string;
  normalizeScore: (s: number) => number;
  fullChunkContent: boolean;
  onDocumentClick?: OnDocumentClick;
}

export function PassageRow({
  chunk,
  chunkIdx,
  docId,
  normalizeScore,
  fullChunkContent,
  onDocumentClick,
}: PassageRowProps) {
  const { t } = useTranslation();
  const router = useRouter();
  const score = normalizeScore(chunk.score);
  const { color: scoreColor, labelKey, defaultLabel } = getConfidenceLabel(score);

  const pageUrl =
    chunk.document_id && chunk.page_start !== undefined
      ? buildDocumentPageUrl(chunk.document_id, chunk.chunk_id, chunk.page_start)
      : null;
  const pageBadge = formatChunkPageBadge(chunk.page_start, chunk.page_end);
  const pageAria =
    chunk.page_start !== undefined &&
    chunk.page_end !== undefined &&
    chunk.page_end > chunk.page_start
      ? t('query.citations.pageRangeAria', 'Pages {{start}} to {{end}}', {
          start: chunk.page_start,
          end: chunk.page_end,
        })
      : t('query.citations.pageSingleAria', 'Page {{page}}', { page: chunk.page_start });

  const passageIndex = chunk.reference_id ?? (chunk.chunk_index ?? chunkIdx) + 1;
  const preview = stripMarkdownSyntax(chunk.content).slice(0, 80);

  const openPassage = () => {
    invokeDocumentClick(onDocumentClick, {
      documentId: docId,
      chunkContent: chunk.content,
      chunkIndex: chunk.chunk_index ?? chunkIdx,
      startLine: chunk.start_line,
      endLine: chunk.end_line,
      chunkId: chunk.chunk_id,
      page: chunk.page_start,
    });
  };

  const navigateToPage = (e: React.MouseEvent | React.KeyboardEvent) => {
    e.stopPropagation();
    if (pageUrl) router.push(pageUrl);
  };

  return (
    <div className="relative">
      <button
        type="button"
        className="w-full text-left p-2.5 rounded-lg bg-muted/30 hover:bg-yellow-50 dark:hover:bg-yellow-900/20 border border-transparent hover:border-yellow-200 dark:hover:border-yellow-800 transition-all duration-150 group/chunk focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/50"
        title={t(
          'query.citations.openPassageTitle',
          'Click to open and highlight this passage in the document viewer',
        )}
        aria-label={t('query.citations.openPassageAria', 'Open passage {{index}}: {{preview}}{{ellipsis}}', {
          index: passageIndex,
          preview,
          ellipsis: (chunk.content?.length ?? 0) > 80 ? '...' : '',
        })}
        onClick={openPassage}
      >
        <div className="flex items-start gap-2">
          <span
            className="flex-shrink-0 mt-0.5 w-5 h-5 rounded bg-muted/60 text-muted-foreground text-xs font-mono flex items-center justify-center select-none"
            aria-hidden="true"
          >
            {passageIndex}
          </span>

          <p
            data-testid="source-passage-text"
            data-full-chunk={fullChunkContent ? 'true' : 'false'}
            className={`text-xs text-foreground/85 flex-1 leading-relaxed break-words overflow-hidden ${
              fullChunkContent ? 'whitespace-pre-wrap' : 'line-clamp-3'
            }`}
          >
            {formatPassagePreview(chunk.content, fullChunkContent)}
          </p>

          <span
            className={`text-xs font-semibold flex-shrink-0 mt-0.5 tabular-nums ${scoreColor}`}
            title={t(labelKey, defaultLabel)}
          >
            {Math.round(score * 100)}%
          </span>
        </div>

        {pageUrl && pageBadge && (
          <div className="mt-1.5 pl-7 flex items-center">
            <span
              role="link"
              tabIndex={0}
              className="inline-flex items-center gap-0.5 text-xs font-medium text-primary/75 hover:text-primary transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-primary/50 rounded-sm cursor-pointer"
              title={t('query.citations.openPdfPageTitle', 'Open PDF at page {{page}}', {
                page: chunk.page_start,
              })}
              aria-label={t('query.citations.goToPageAria', 'Go to {{pageAria}} in document viewer', {
                pageAria,
              })}
              data-testid="citation-page-badge"
              onClick={navigateToPage}
              onKeyDown={(e) => {
                if (e.key === 'Enter' || e.key === ' ') {
                  e.preventDefault();
                  navigateToPage(e);
                }
              }}
            >
              <BookOpen className="h-2.5 w-2.5" aria-hidden="true" />
              <span className="ml-0.5">{pageBadge}</span>
              <ExternalLink className="h-2 w-2 ml-0.5 opacity-70" aria-hidden="true" />
            </span>
          </div>
        )}

        {!pageUrl && chunk.start_line !== undefined && chunk.end_line !== undefined && (
          <div className="mt-1 pl-7 text-xs text-muted-foreground">
            {t('query.citations.lineRange', 'L{{start}}–{{end}}', {
              start: chunk.start_line,
              end: chunk.end_line,
            })}
          </div>
        )}
      </button>
    </div>
  );
}

export interface PagePassageGroupProps {
  page: number | null;
  passages: Chunks;
  docId: string;
  normalizeScore: (s: number) => number;
  fullChunkContent: boolean;
  onDocumentClick?: OnDocumentClick;
}

export function PagePassageGroup({
  page,
  passages,
  docId,
  normalizeScore,
  fullChunkContent,
  onDocumentClick,
}: PagePassageGroupProps) {
  const { t } = useTranslation();
  const firstChunkDocId = passages[0]?.document_id;
  const pageDeeplink =
    page !== null && firstChunkDocId
      ? buildDocumentPageUrl(firstChunkDocId, undefined, page)
      : null;

  return (
    <div className="space-y-1.5">
      {page !== null && (
        <div className="flex items-center gap-2 pt-1.5 pb-0.5">
          <div className="flex items-center gap-1 bg-primary/8 dark:bg-primary/12 border border-primary/15 rounded-full px-2 py-0.5">
            <BookOpen className="h-2.5 w-2.5 text-primary/70 flex-shrink-0" aria-hidden="true" />
            <span className="text-xs font-semibold text-primary/80 leading-none">
              {t('query.citations.pageHeader', 'Page {{page}}', { page })}
            </span>
          </div>

          <div className="flex-1 h-px bg-border/40" aria-hidden="true" />

          {pageDeeplink && (
            <Link
              href={pageDeeplink}
              className="inline-flex items-center gap-0.5 text-xs font-medium text-primary/70 hover:text-primary transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-primary/40 rounded-sm"
              aria-label={t('query.citations.openPdfAtPageAria', 'Open PDF at page {{page}}', { page })}
              title={t('query.citations.jumpToPageTitle', 'Jump to page {{page}} in the document viewer', {
                page,
              })}
            >
              <ExternalLink className="h-2.5 w-2.5" />
            </Link>
          )}
        </div>
      )}

      <div className={page !== null ? 'pl-1 space-y-1.5' : 'space-y-1.5'}>
        {passages.map((chunk, idx) => (
          <PassageRow
            key={chunk.chunk_id ?? idx}
            chunk={chunk}
            chunkIdx={idx}
            docId={docId}
            normalizeScore={normalizeScore}
            fullChunkContent={fullChunkContent}
            onDocumentClick={onDocumentClick}
          />
        ))}
      </div>
    </div>
  );
}
