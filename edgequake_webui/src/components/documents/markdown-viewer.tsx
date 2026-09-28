/**
 * @module MarkdownViewer
 * @description Reusable Markdown viewer component with copy and scroll features.
 * Displays extracted markdown content with syntax highlighting and formatting.
 *
 * @implements SPEC-002 - Document Viewer with Markdown display
 * @implements SPEC-143 - Page sync via useMarkdownPageObserver
 * @implements FEAT0721 - Markdown rendering with syntax highlighting
 * @implements FEAT0722 - Copy content to clipboard
 * @implements FEAT0723 - Line numbers display
 *
 * @enforces BR0721 - Smooth scrolling within container
 * @enforces BR0722 - Proper typography and spacing
 *
 * @see {@link docs/features.md} FEAT0721-0723
 */
'use client';

import { StreamingMarkdownRenderer } from '@/components/query/markdown';
import {
  VIRTUALIZATION_CHAR_THRESHOLD,
  VirtualizedMarkdownContent,
} from '@/components/query/markdown/VirtualizedMarkdownContent';
import { Button } from '@/components/ui/button';
import { useMarkdownPageObserver } from '@/hooks/use-markdown-page-observer';
import type { PageSyncDriver } from '@/hooks/use-page-sync-controller';
import { rewriteMarkdownMmAssetUrls } from '@/lib/api/edgequake/documents';
import { downloadFile, sanitizeFilename } from '@/lib/export-conversation';
import { cn } from '@/lib/utils';
import { injectPageAnchors } from '@/lib/utils/page-markers';
import { Check, Copy, Download, FileText } from 'lucide-react';
import { useCallback, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

interface MarkdownViewerProps {
  /** Markdown content to display */
  content: string | null;
  /** Optional class name for container */
  className?: string;
  /** Fixed height for container (enables scrolling) */
  height?: number;
  /** Whether to show the toolbar */
  showToolbar?: boolean;
  /** Whether to show line numbers */
  showLineNumbers?: boolean;
  /** Title displayed in toolbar */
  title?: string;
  /** When set, rewrite `![…](assets/…)` to document mm-asset API URLs (MV-28). */
  documentId?: string | null;
  /** SPEC-143: active page from sync controller. */
  activePage?: number;
  /** SPEC-143: any directional sync selected (badge / reveal chrome). */
  syncEnabled?: boolean;
  /** SPEC-143: markdown follows PDF (pdf-to-md). */
  followMarkdown?: boolean;
  /** SPEC-143: report page when user scrolls markdown (always records). */
  onPageFromMd?: (page: number) => void;
  /** SPEC-143: skip auto-scroll when markdown is the driver. */
  syncDriver?: PageSyncDriver;
  onMdGestureStart?: () => void;
  onMdGestureEnd?: () => void;
}

/**
 * MarkdownViewer component for displaying markdown content.
 *
 * Uses the existing StreamingMarkdownRenderer for high-quality markdown rendering
 * with syntax highlighting, code blocks, tables, and math support.
 */
export function MarkdownViewer({
  content,
  className,
  height,
  showToolbar = true,
  showLineNumbers = false,
  title = 'Extracted Markdown',
  documentId = null,
  activePage,
  syncEnabled = false,
  followMarkdown = false,
  onPageFromMd,
  syncDriver = 'none',
  onMdGestureStart,
  onMdGestureEnd,
}: MarkdownViewerProps) {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);
  const contentRef = useRef<HTMLDivElement>(null);
  const scrollRootRef = useRef<HTMLDivElement>(null);
  const displayContent = useMemo(() => {
    if (!content) return content;
    // SPEC-143: inject page anchors after asset rewrite (same SSOT as ContentRenderer).
    return injectPageAnchors(rewriteMarkdownMmAssetUrls(content, documentId));
  }, [content, documentId]);

  // Own the scrollport only when a fixed height is provided. Otherwise the
  // side-by-side pane (`md-scroll-container`) is the scroll root.
  const ownsScroll = height != null && height > 0;

  useMarkdownPageObserver({
    containerRef: contentRef,
    scrollRootRef: ownsScroll ? scrollRootRef : undefined,
    // Always observe when wired so switching to md-to-pdf has a fresh md page.
    enabled: Boolean(onPageFromMd && displayContent),
    onPage: (page) => onPageFromMd?.(page),
    scrollToPage: followMarkdown ? activePage ?? null : null,
    skipScroll: !followMarkdown || syncDriver === 'md',
    onGestureStart: onMdGestureStart,
    onGestureEnd: onMdGestureEnd,
  });

  const handleCopy = useCallback(async () => {
    if (!content) return;
    try {
      await navigator.clipboard.writeText(content);
      setCopied(true);
      toast.success(t('common.copied', 'Copied to clipboard'));
      setTimeout(() => setCopied(false), 2000);
    } catch {
      toast.error(t('common.copyFailed', 'Failed to copy'));
    }
  }, [content, t]);

  const handleDownload = useCallback(() => {
    if (!content) return;
    try {
      const filename = `${sanitizeFilename(title)}.md`;
      downloadFile(content, filename, 'text/markdown;charset=utf-8');
      toast.success(t('documents.download.markdownStarted', 'Markdown download started'));
    } catch {
      toast.error(t('documents.download.markdownFailed', 'Failed to download markdown'));
    }
  }, [content, title, t]);

  if (!content) {
    return (
      <div className="flex flex-col items-center justify-center p-8 text-center text-muted-foreground">
        <FileText className="h-12 w-12 mb-4 opacity-50" />
        <p>{t('documents.viewer.noMarkdown', 'No markdown content available')}</p>
        <p className="text-sm mt-2">
          {t('documents.viewer.noMarkdownHint', 'The document may not have been processed yet.')}
        </p>
      </div>
    );
  }

  return (
    <div className={cn('flex flex-col', className)}>
      {showToolbar && (
        <div className="flex items-center justify-between gap-2 p-2 border-b bg-muted/30">
          <div className="flex items-center gap-2">
            <FileText className="h-4 w-4 text-muted-foreground" />
            <span className="text-sm font-medium">{title}</span>
          </div>
          <div className="flex items-center gap-1">
            <Button
              variant="ghost"
              size="sm"
              className="h-8"
              onClick={handleDownload}
              title={t('documents.download.markdown', 'Download markdown')}
            >
              <Download className="h-4 w-4" />
              <span className="ml-1.5 hidden sm:inline">
                {t('documents.download.markdown', 'Download markdown')}
              </span>
            </Button>
            <Button
              variant="ghost"
              size="sm"
              className="h-8"
              onClick={handleCopy}
              title={t('common.copy', 'Copy to clipboard')}
            >
              {copied ? (
                <Check className="h-4 w-4 text-green-500" />
              ) : (
                <Copy className="h-4 w-4" />
              )}
              <span className="ml-1.5 hidden sm:inline">
                {copied ? t('common.copied', 'Copied') : t('common.copy', 'Copy')}
              </span>
            </Button>
          </div>
        </div>
      )}

      <div
        ref={scrollRootRef}
        className={cn('flex-1 bg-background', ownsScroll && 'overflow-auto')}
        style={{ height: ownsScroll ? `${height}px` : 'auto' }}
        data-testid={ownsScroll ? 'md-scroll-container' : undefined}
      >
        {syncEnabled && activePage != null && activePage >= 1 ? (
          <div
            className="sticky top-0 z-10 px-4 py-0.5 text-xs text-muted-foreground bg-background/90 backdrop-blur-sm"
            data-testid="md-page-indicator"
            data-page={activePage}
          >
            Page {activePage}
          </div>
        ) : null}
        <div ref={contentRef} className="relative">
          {displayContent!.length >= VIRTUALIZATION_CHAR_THRESHOLD ? (
            <VirtualizedMarkdownContent
              content={displayContent!}
              scrollToPage={followMarkdown ? activePage ?? null : null}
            >
              {(pageContent) => (
                <div
                  className={cn(
                    'p-4 md:p-6',
                    'prose prose-sm md:prose-base dark:prose-invert max-w-none',
                    'prose-headings:scroll-mt-4',
                    'prose-pre:bg-muted/50 prose-pre:border prose-pre:border-border',
                    'prose-code:before:content-none prose-code:after:content-none',
                    'prose-table:text-sm',
                    showLineNumbers && 'markdown-with-line-numbers',
                  )}
                >
                  <StreamingMarkdownRenderer
                    content={pageContent}
                    isStreaming={false}
                    revealPage={followMarkdown ? activePage ?? null : null}
                  />
                </div>
              )}
            </VirtualizedMarkdownContent>
          ) : (
            <div
              className={cn(
                'p-4 md:p-6',
                'prose prose-sm md:prose-base dark:prose-invert max-w-none',
                'prose-headings:scroll-mt-4',
                'prose-pre:bg-muted/50 prose-pre:border prose-pre:border-border',
                'prose-code:before:content-none prose-code:after:content-none',
                'prose-table:text-sm',
                showLineNumbers && 'markdown-with-line-numbers',
              )}
            >
              <StreamingMarkdownRenderer
                content={displayContent!}
                isStreaming={false}
                revealPage={followMarkdown ? activePage ?? null : null}
              />
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

export default MarkdownViewer;
