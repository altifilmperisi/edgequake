'use client';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { ScrollArea } from '@/components/ui/scroll-area';
import { getConfidenceLabel } from '@/lib/citations/confidence';
import {
  groupPassagesByPage,
  normalizeChunkScores,
  type Chunks,
} from '@/lib/citations/group-passages';
import { getDocumentTitle } from '@/lib/citations/passage-text';
import type { OnDocumentClick } from '@/lib/citations/types';
import { invokeDocumentClick } from '@/lib/citations/types';
import { ChevronDown, ChevronUp, ExternalLink, FileText } from 'lucide-react';
import { useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';

import { PagePassageGroup, PassageRow } from './passage-row';

export function DocumentsTab({
  chunksByDocument,
  fullChunkContent,
  onDocumentClick,
}: {
  chunksByDocument: Record<string, Chunks>;
  fullChunkContent: boolean;
  onDocumentClick?: OnDocumentClick;
}) {
  const { t } = useTranslation();
  const [expandedDocs, setExpandedDocs] = useState<Set<string>>(new Set());
  const entries = Object.entries(chunksByDocument);

  const normalizeScore = useMemo(
    () => normalizeChunkScores(chunksByDocument),
    [chunksByDocument],
  );

  const titleLabels = useMemo(
    () => ({
      untitled: t('query.citations.untitled', 'Untitled'),
      untitledDocument: t('query.citations.untitledDocument', 'Untitled Document'),
    }),
    [t],
  );

  const toggleDocExpand = (docId: string) => {
    setExpandedDocs((prev) => {
      const next = new Set(prev);
      if (next.has(docId)) next.delete(docId);
      else next.add(docId);
      return next;
    });
  };

  const openDocument = (docId: string, chunks: Chunks) => {
    invokeDocumentClick(onDocumentClick, {
      documentId: docId,
      chunkContent: chunks[0]?.content,
      chunkIndex: 0,
      chunkId: chunks[0]?.chunk_id,
      page: chunks[0]?.page_start,
    });
  };

  if (entries.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center h-70 sm:h-90 text-muted-foreground">
        <FileText className="h-8 w-8 mb-2 opacity-50" aria-hidden="true" />
        <p className="text-sm">{t('query.citations.noSourceDocuments', 'No source documents')}</p>
      </div>
    );
  }

  const totalChunks = entries.reduce((acc, [, chunks]) => acc + chunks.length, 0);

  return (
    <div className="space-y-1.5">
      <p className="text-xs text-muted-foreground px-0.5">
        {t('query.citations.documentCount', '{{count}} document', { count: entries.length })}
        {' · '}
        {t('query.citations.passageCount', '{{count}} passage', { count: totalChunks })}
      </p>

      <ScrollArea className="h-70 sm:h-83">
        <div
          className="space-y-2 pr-2"
          role="list"
          aria-label={t('query.citations.sourceDocumentsList', 'Source documents')}
        >
          {entries.map(([docId, chunks], index) => {
            const avgScore =
              chunks.reduce((acc, c) => acc + normalizeScore(c.score), 0) / chunks.length;
            const { color: scoreColor } = getConfidenceLabel(avgScore);
            const isExpanded = expandedDocs.has(docId);
            const visibleChunks = isExpanded ? chunks : chunks.slice(0, 3);
            const hiddenCount = chunks.length - 3;
            const docTitle = getDocumentTitle(chunks, titleLabels);

            return (
              <Card
                key={docId}
                className="group bg-card border border-border/50 hover:border-border hover:shadow-sm transition-all duration-200"
                role="listitem"
              >
                <CardContent className="p-3">
                  <div className="flex items-start gap-3">
                    <span
                      className="flex-shrink-0 w-6 h-6 rounded-full bg-primary/10 text-primary text-xs flex items-center justify-center font-semibold group-hover:bg-primary group-hover:text-primary-foreground transition-colors"
                      aria-hidden="true"
                    >
                      {index + 1}
                    </span>

                    <div className="flex-1 min-w-0 space-y-1.5">
                      <div className="flex items-center justify-between gap-2">
                        <button
                          type="button"
                          className="text-sm font-semibold flex items-center gap-1.5 hover:text-primary transition-colors text-left max-w-full overflow-hidden text-foreground/90 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/50 rounded-sm"
                          onClick={() => openDocument(docId, chunks)}
                          title={t('query.citations.openDocumentTitle', 'Open: {{title}}', {
                            title: docTitle,
                          })}
                          aria-label={t('query.citations.openDocumentAria', 'Open document: {{title}}', {
                            title: docTitle,
                          })}
                        >
                          <FileText
                            className="h-3.5 w-3.5 text-muted-foreground flex-shrink-0"
                            aria-hidden="true"
                          />
                          <span className="truncate">{docTitle}</span>
                        </button>
                        <div className="flex items-center gap-1.5 flex-shrink-0">
                          <span className={`text-xs font-semibold ${scoreColor}`}>
                            {Math.round(avgScore * 100)}%
                          </span>
                          {chunks.length > 1 && (
                            <Badge variant="outline" className="text-xs h-4 px-1">
                              {chunks.length}×
                            </Badge>
                          )}
                          <Button
                            variant="ghost"
                            size="sm"
                            className="h-6 w-6 p-0 opacity-0 group-hover:opacity-100 transition-opacity"
                            onClick={() => openDocument(docId, chunks)}
                            aria-label={t('query.citations.openDocumentAria', 'Open document: {{title}}', {
                              title: docTitle,
                            })}
                          >
                            <ExternalLink className="h-3.5 w-3.5" aria-hidden="true" />
                          </Button>
                        </div>
                      </div>

                      <div className="space-y-1.5 mt-2">
                        {(() => {
                          const grouped = groupPassagesByPage(visibleChunks);
                          if (grouped) {
                            return [...grouped.entries()]
                              .sort(([a], [b]) => {
                                if (a === null) return 1;
                                if (b === null) return -1;
                                return a - b;
                              })
                              .map(([page, passages]) => (
                                <PagePassageGroup
                                  key={page ?? 'nopage'}
                                  page={page}
                                  passages={passages}
                                  docId={docId}
                                  normalizeScore={normalizeScore}
                                  fullChunkContent={fullChunkContent}
                                  onDocumentClick={onDocumentClick}
                                />
                              ));
                          }
                          return visibleChunks.map((chunk, chunkIdx) => (
                            <PassageRow
                              key={chunk.chunk_id ?? chunkIdx}
                              chunk={chunk}
                              chunkIdx={chunkIdx}
                              docId={docId}
                              normalizeScore={normalizeScore}
                              fullChunkContent={fullChunkContent}
                              onDocumentClick={onDocumentClick}
                            />
                          ));
                        })()}

                        {hiddenCount > 0 && (
                          <button
                            type="button"
                            className="w-full text-xs text-muted-foreground hover:text-foreground flex items-center justify-center gap-1 py-1 rounded hover:bg-muted/40 transition-colors"
                            onClick={() => toggleDocExpand(docId)}
                          >
                            {isExpanded ? (
                              <>
                                <ChevronUp className="h-3 w-3" />
                                {t('query.citations.showLess', 'Show less')}
                              </>
                            ) : (
                              <>
                                <ChevronDown className="h-3 w-3" />
                                {t('query.citations.morePassages', '+{{count}} more passage', {
                                  count: hiddenCount,
                                })}
                              </>
                            )}
                          </button>
                        )}
                      </div>
                    </div>
                  </div>
                </CardContent>
              </Card>
            );
          })}
        </div>
      </ScrollArea>
    </div>
  );
}
