/**
 * @module PDFMarkdownSplitView
 * @description Side-by-side view of PDF and extracted Markdown.
 *
 * @implements SPEC-002 - Document Viewer with PDF+Markdown display
 * @implements SPEC-143 - Directional page sync via shared controller
 * @implements FEAT0731 - PDF and Markdown side-by-side view
 * @implements FEAT0732 - View mode toggle
 * @implements FEAT0733 - Panel synchronization controls
 */
'use client';

import { Button } from '@/components/ui/button';
import { PageSyncModeControl } from '@/components/documents/page-sync-mode-control';
import { usePageSyncController } from '@/hooks/use-page-sync-controller';
import { pdfCurrentPageForMode } from '@/lib/documents/page-sync-mode';
import { cn } from '@/lib/utils';
import { hasPageMarkers } from '@/lib/utils/page-markers';
import { Columns, FileText, FileType } from 'lucide-react';
import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { MarkdownViewer } from './markdown-viewer';
import { PDFViewer } from './pdf-viewer';

type ViewMode = 'pdf' | 'markdown' | 'split';

interface PDFMarkdownSplitViewProps {
  pdfUrl: string;
  markdown: string | null;
  className?: string;
  height?: number;
  initialMode?: ViewMode;
  documentId?: string | null;
}

export function PDFMarkdownSplitView({
  pdfUrl,
  markdown,
  className,
  height = 500,
  initialMode = 'split',
  documentId = null,
}: PDFMarkdownSplitViewProps) {
  const { t } = useTranslation();
  const [viewMode, setViewMode] = useState<ViewMode>(initialMode);
  const pageSync = usePageSyncController({ initialPage: 1 });
  const syncAvailable = hasPageMarkers(markdown);
  const pdfCurrentPage = pdfCurrentPageForMode(
    syncAvailable ? pageSync.syncMode : 'none',
    pageSync.activePage,
  );

  const handleModeChange = useCallback((mode: ViewMode) => {
    setViewMode(mode);
  }, []);

  const showPdf = viewMode === 'pdf' || viewMode === 'split';
  const showMarkdown = viewMode === 'markdown' || viewMode === 'split';

  return (
    <div className={cn('flex flex-col', className)} data-testid="pdf-markdown-split-view">
      <div className="flex items-center justify-between gap-2 p-2 border-b bg-muted/30">
        <div className="flex items-center gap-1">
          <span className="text-sm font-medium text-muted-foreground mr-2">
            {t('documents.viewer.viewMode', 'View:')}
          </span>

          <Button
            variant={viewMode === 'pdf' ? 'secondary' : 'ghost'}
            size="sm"
            className="h-8"
            onClick={() => handleModeChange('pdf')}
            title={t('documents.viewer.pdfOnly', 'PDF Only')}
          >
            <FileType className="h-4 w-4 mr-1.5" />
            <span className="hidden sm:inline">PDF</span>
          </Button>

          <Button
            variant={viewMode === 'split' ? 'secondary' : 'ghost'}
            size="sm"
            className="h-8"
            onClick={() => handleModeChange('split')}
            title={t('documents.viewer.splitView', 'Side by Side')}
          >
            <Columns className="h-4 w-4 mr-1.5" />
            <span className="hidden sm:inline">{t('documents.viewer.split', 'Split')}</span>
          </Button>

          <Button
            variant={viewMode === 'markdown' ? 'secondary' : 'ghost'}
            size="sm"
            className="h-8"
            onClick={() => handleModeChange('markdown')}
            title={t('documents.viewer.markdownOnly', 'Markdown Only')}
          >
            <FileText className="h-4 w-4 mr-1.5" />
            <span className="hidden sm:inline">Markdown</span>
          </Button>

          {viewMode === 'split' ? (
            <PageSyncModeControl
              className="ml-1"
              mode={pageSync.syncMode}
              onModeChange={pageSync.setSyncMode}
              available={syncAvailable}
              compact={false}
            />
          ) : null}
        </div>
      </div>

      <div
        className={cn(
          'flex-1',
          viewMode === 'split' ? 'flex flex-col lg:grid lg:grid-cols-2' : 'flex',
        )}
        style={{ height: `${height}px` }}
      >
        {showPdf && (
          <div
            className={cn(
              'flex flex-col overflow-hidden',
              viewMode === 'split'
                ? 'h-1/2 lg:h-full lg:border-r border-border'
                : 'flex-1',
            )}
          >
            <PDFViewer
              file={pdfUrl}
              showToolbar={true}
              height={viewMode === 'split' ? height / 2 : height}
              className="flex-1"
              currentPage={pdfCurrentPage}
              onPageChange={pageSync.setPageFromPdf}
              onGestureStart={() => pageSync.beginGesture('pdf')}
              onGestureEnd={pageSync.endGesture}
              documentId={documentId ?? undefined}
            />
          </div>
        )}

        {showMarkdown && (
          <div
            className={cn(
              'flex flex-col overflow-hidden',
              viewMode === 'split' ? 'h-1/2 lg:h-full' : 'flex-1',
            )}
          >
            <MarkdownViewer
              content={markdown}
              showToolbar={true}
              height={viewMode === 'split' ? height / 2 : height}
              title={t('documents.viewer.extractedMarkdown', 'Extracted Markdown')}
              className="flex-1"
              documentId={documentId}
              activePage={pageSync.activePage}
              syncEnabled={pageSync.syncEnabled && syncAvailable}
              followMarkdown={pageSync.followMarkdown && syncAvailable}
              onPageFromMd={pageSync.setPageFromMd}
              syncDriver={pageSync.driver}
              onMdGestureStart={() => pageSync.beginGesture('md')}
              onMdGestureEnd={pageSync.endGesture}
            />
          </div>
        )}
      </div>
    </div>
  );
}

export default PDFMarkdownSplitView;
