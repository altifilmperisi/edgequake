/**
 * @module PDFViewer
 * @description Reusable PDF viewer using react-pdf.
 * SPEC-143: continuous multi-page scroll stack + onPageChange for Markdown sync.
 *
 * @implements SPEC-002 - Document Viewer with PDF display
 * @implements SPEC-033 - Controlled currentPage / deeplink
 * @implements SPEC-143 - Continuous stack, wheel page nav, onPageChange
 * @implements FEAT0711 - PDF rendering with react-pdf
 * @implements FEAT0712 - Page navigation controls
 * @implements FEAT0713 - Zoom controls
 */
'use client';

import {
  PdfPageOverlay,
  type OverlayChips,
} from '@/components/documents/pdf-page-overlay';
import { Button } from '@/components/ui/button';
import { Skeleton } from '@/components/ui/skeleton';
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import {
  getDocumentPageLayout,
  listDocumentPages,
} from '@/lib/api/edgequake/documents';
import {
  beginScrollSuppress,
  offsetTopWithin,
  pageAtReadingLine,
  scrollPaneTo,
  scrollTopForPage,
} from '@/lib/documents/page-scroll';
import {
  extractPdfSourceUrl,
  fetchAuthenticatedPdfData,
  isApiProtectedPdfUrl,
  type PdfFileSource,
} from '@/lib/documents/resolve-authenticated-pdf-source';
import { cn } from '@/lib/utils';
import { useQuery } from '@tanstack/react-query';
import {
  ChevronLeft,
  ChevronRight,
  Loader2,
  Maximize2,
  Minimize2,
  XCircle,
  ZoomIn,
  ZoomOut,
} from 'lucide-react';
import dynamic from 'next/dynamic';
import type { DocumentProps } from 'react-pdf';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import 'react-pdf/dist/Page/AnnotationLayer.css';
import 'react-pdf/dist/Page/TextLayer.css';

type PDFFileSource = PdfFileSource;

const Document = dynamic(() => import('react-pdf').then((mod) => mod.Document), {
  ssr: false,
  loading: () => <PDFLoadingSkeleton />,
});

const Page = dynamic(() => import('react-pdf').then((mod) => mod.Page), {
  ssr: false,
});

if (typeof window !== 'undefined') {
  import('react-pdf').then(({ pdfjs }) => {
    pdfjs.GlobalWorkerOptions.workerSrc = '/pdf.worker.min.mjs';
  });
}

/** Windowed render kicks in above this page count (SPEC-143). */
const WINDOW_THRESHOLD = 20;
const WINDOW_RADIUS = 2;

interface PDFViewerProps {
  file: PDFFileSource;
  className?: string;
  initialPage?: number;
  /**
   * Controlled current page (1-indexed).
   * External navigation (deeplink, markdown sync, chunk click).
   */
  currentPage?: number;
  /** SPEC-143: emitted when the active page changes via scroll / toolbar / keyboard. */
  onPageChange?: (page: number) => void;
  /** SPEC-143: user gesture ownership for the sync controller. */
  onGestureStart?: () => void;
  onGestureEnd?: () => void;
  initialScale?: number;
  showToolbar?: boolean;
  width?: number;
  height?: number;
  onLoadSuccess?: (numPages: number) => void;
  onLoadError?: (error: Error) => void;
  /** Document id for SPEC-128 layout overlay. */
  documentId?: string;
}

function PDFLoadingSkeleton() {
  return (
    <div className="flex flex-col items-center justify-center p-8 space-y-4">
      <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      <Skeleton className="h-[400px] w-[300px]" />
    </div>
  );
}

function PDFErrorState({ error, onRetry }: { error: string; onRetry?: () => void }) {
  const { t } = useTranslation();
  const is404 = error.includes('404') || error.includes('not found');
  const isNetworkError =
    error.includes('NetworkError') ||
    error.includes('Failed to fetch') ||
    error.includes('network');

  const displayMessage = is404
    ? t(
        'documents.viewer.pdfNotFound',
        'PDF file is not available. The file may have been removed or processing may not be complete.',
      )
    : isNetworkError
      ? t(
          'documents.viewer.pdfNetworkError',
          'Unable to connect to the server. Please check your connection and try again.',
        )
      : error;

  return (
    <div className="flex flex-col items-center justify-center p-8 space-y-4 text-center">
      <div className="rounded-full bg-muted p-3">
        <XCircle className="h-6 w-6 text-muted-foreground" />
      </div>
      <div className="space-y-1">
        <p className="text-sm font-medium text-muted-foreground">
          {is404
            ? t('documents.viewer.pdfUnavailable', 'PDF Unavailable')
            : t('documents.viewer.loadError', 'Failed to Load PDF')}
        </p>
        <p className="text-xs text-muted-foreground max-w-sm">{displayMessage}</p>
      </div>
      {onRetry && !is404 && (
        <Button variant="outline" size="sm" onClick={onRetry}>
          {t('common.retry', 'Retry')}
        </Button>
      )}
    </div>
  );
}

function layoutToggleDisabled(
  summary: { layout_status: string; region_count?: number | null } | undefined,
  pagesLoaded: boolean,
  pagesError: boolean,
): boolean {
  if (pagesError) return false;
  if (!pagesLoaded) return true;
  if (!summary) return true;
  const status = summary.layout_status;
  const count = summary.region_count ?? 0;
  if (status === 'failed' || status === 'pending') return true;
  if (status === 'skipped' && count === 0) return true;
  return false;
}

function layoutToggleHint(
  summary: { layout_status: string; region_count?: number | null } | undefined,
  disabled: boolean,
  t: (key: string, fallback: string) => string,
): string {
  if (!disabled) {
    return t('documents.viewer.layout.toggleHint', 'Toggle layout regions (O)');
  }
  if (!summary) {
    return t('documents.viewer.layout.unavailable', 'Layout data unavailable');
  }
  return t('documents.viewer.layout.disabled', 'Layout overlay unavailable for this page');
}

export function PDFViewer({
  file,
  className,
  initialPage = 1,
  currentPage,
  onPageChange,
  onGestureStart,
  onGestureEnd,
  initialScale = 1.0,
  showToolbar = true,
  width,
  height,
  onLoadSuccess,
  onLoadError,
  documentId,
}: PDFViewerProps) {
  const { t } = useTranslation();
  const [numPages, setNumPages] = useState(0);
  const [pageNumber, setPageNumber] = useState(initialPage);
  const [scale, setScale] = useState(initialScale);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [isFullWidth, setIsFullWidth] = useState(false);
  const [overlayOn, setOverlayOn] = useState(false);
  const [pageBox, setPageBox] = useState<{ width: number; height: number } | null>(null);
  const [basePageHeight, setBasePageHeight] = useState(800);
  const [chips, setChips] = useState<OverlayChips>({
    figures: true,
    charts: true,
    tables: true,
    paragraphs: false,
    columns: false,
    noise: false,
  });

  const scrollRef = useRef<HTMLDivElement>(null);
  const stackRef = useRef<HTMLDivElement>(null);
  const sheetRefs = useRef<Map<number, HTMLDivElement>>(new Map());
  const lastEmittedRef = useRef<number | null>(null);
  const suppressScrollEmitRef = useRef(false);
  const followRafRef = useRef<number | null>(null);
  /** True while the user is gesturing on the PDF pane (wheel / pointer / keys). */
  const pdfGestureRef = useRef(false);
  const onGestureStartRef = useRef(onGestureStart);
  onGestureStartRef.current = onGestureStart;
  const onGestureEndRef = useRef(onGestureEnd);
  onGestureEndRef.current = onGestureEnd;
  const [bottomSpacer, setBottomSpacer] = useState(0);
  /** Bumped when the stack resizes or a user gesture ends, so follow can correct. */
  const [layoutTick, setLayoutTick] = useState(0);
  const alignFrameRef = useRef<number | null>(null);
  /**
   * Last programmatic alignment. Layout shifts compensate from here.
   * A user scroll clears stickToTop so a resize does not yank mid-page.
   */
  const holdRef = useRef<{
    page: number;
    start: number;
    scrollTop: number;
    stickToTop: boolean;
  } | null>(null);
  const isControlled = currentPage !== undefined;

  const scheduleAlign = useCallback(() => {
    if (pdfGestureRef.current) return;
    if (alignFrameRef.current != null) return;
    alignFrameRef.current = requestAnimationFrame(() => {
      alignFrameRef.current = null;
      if (pdfGestureRef.current) return;
      setLayoutTick((n) => n + 1);
    });
  }, []);

  /** Controlled page is SSOT for toolbar when provided. */
  const displayPage =
    currentPage !== undefined && currentPage >= 1 ? currentPage : pageNumber;

  const pagesQuery = useQuery({
    queryKey: ['document-pages', documentId],
    queryFn: () => listDocumentPages(documentId!),
    enabled: Boolean(documentId),
  });
  const pageSummary = pagesQuery.data?.pages?.find((p) => p.page_number === displayPage);
  const overlayDisabled = layoutToggleDisabled(
    pageSummary,
    pagesQuery.isSuccess,
    pagesQuery.isError,
  );
  const overlayHint = layoutToggleHint(pageSummary, overlayDisabled, t);

  const layoutQuery = useQuery({
    queryKey: ['document-page-layout', documentId, displayPage],
    queryFn: () => getDocumentPageLayout(documentId!, displayPage),
    enabled: Boolean(documentId) && overlayOn && displayPage > 0 && !overlayDisabled,
  });

  const collectSheetStarts = useCallback((): Map<number, number> => {
    const root = scrollRef.current;
    const map = new Map<number, number>();
    if (!root) return map;
    sheetRefs.current.forEach((el, p) => {
      map.set(p, offsetTopWithin(root, el));
    });
    return map;
  }, []);

  const emitPage = useCallback(
    (page: number) => {
      const clamped =
        numPages > 0 ? Math.max(1, Math.min(numPages, page)) : Math.max(1, page);
      setPageNumber(clamped);
      if (lastEmittedRef.current === clamped) return;
      lastEmittedRef.current = clamped;
      onPageChange?.(clamped);
    },
    [numPages, onPageChange],
  );

  const scrollToPage = useCallback(
    (page: number) => {
      const root = scrollRef.current;
      const clamped =
        numPages > 0 ? Math.max(1, Math.min(numPages, page)) : Math.max(1, page);
      setPageNumber(clamped);
      if (!root) return;
      const el = sheetRefs.current.get(clamped);
      if (!el) return;
      const starts = collectSheetStarts();
      const maxScroll = Math.max(0, root.scrollHeight - root.clientHeight);
      const top = scrollTopForPage(starts, clamped, maxScroll);
      const result = scrollPaneTo(root, top);
      if (result.pending) {
        beginScrollSuppress(root, suppressScrollEmitRef);
      }
    },
    [numPages, collectSheetStarts],
  );

  // Bottom spacer so every page (including the last) can sit at the reading line.
  useEffect(() => {
    const root = scrollRef.current;
    if (!root || numPages < 1) return;
    const measure = () => {
      const last = sheetRefs.current.get(numPages);
      if (!last) {
        setBottomSpacer(0);
        return;
      }
      const lastH = last.getBoundingClientRect().height;
      setBottomSpacer(Math.max(0, Math.round(root.clientHeight - lastH)));
    };
    measure();
    const ro = new ResizeObserver(measure);
    ro.observe(root);
    return () => ro.disconnect();
  }, [numPages, scale, pageBox]);

  // Follow controlled page (deeplink / markdown sync).
  // Do not snap back while the user is scrolling this pane — that race
  // rewound a scroll-to-page-N back to the previous controlled page.
  useEffect(() => {
    if (currentPage === undefined) return;
    if (pdfGestureRef.current) return;
    const clamped =
      numPages > 0
        ? Math.max(1, Math.min(numPages, currentPage))
        : Math.max(1, currentPage);
    setPageNumber(clamped);
    lastEmittedRef.current = clamped;

    const root = scrollRef.current;
    if (!root || numPages < 1) return;
    const el = sheetRefs.current.get(clamped);
    if (!el) return;

    const starts = collectSheetStarts();
    const start = starts.get(clamped);
    if (start == null) return;
    const maxScroll = Math.max(0, root.scrollHeight - root.clientHeight);
    const desired = scrollTopForPage(starts, clamped, maxScroll);
    const prev = holdRef.current;
    const pageChanged = !prev || prev.page !== clamped;
    // Gesture flag is the only "user moved" signal. Scroll anchoring also
    // changes scrollTop when sheets above resize, and treating that as a
    // user move double-applies the drift.
    let top = desired;
    let stickToTop = true;
    if (!pageChanged && prev && !prev.stickToTop) {
      stickToTop = false;
      const drift = start - prev.start;
      top = Math.abs(drift) > 1 ? root.scrollTop + drift : root.scrollTop;
    }
    const result = scrollPaneTo(root, top);
    if (result.pending) {
      beginScrollSuppress(root, suppressScrollEmitRef);
    }
    holdRef.current = {
      page: clamped,
      start,
      scrollTop: root.scrollTop,
      stickToTop,
    };
  }, [currentPage, numPages, scale, pageBox, bottomSpacer, layoutTick, collectSheetStarts]);

  // Page canvases change stack height after the first scroll. Re-align the
  // controlled page once the user is not mid-gesture.
  useEffect(() => {
    const stack = stackRef.current;
    if (!stack || currentPage === undefined) return;
    const ro = new ResizeObserver(() => {
      scheduleAlign();
    });
    ro.observe(stack);
    return () => {
      ro.disconnect();
      if (alignFrameRef.current != null) {
        cancelAnimationFrame(alignFrameRef.current);
        alignFrameRef.current = null;
      }
    };
  }, [currentPage, numPages, scheduleAlign]);

  // Reading-line spy — in controlled mode only emit during a user gesture so
  // load-time / follower scroll cannot overwrite a deeplink target.
  useEffect(() => {
    const root = scrollRef.current;
    if (!root || numPages < 1) return;

    const pickActivePage = () => {
      if (suppressScrollEmitRef.current) return;
      if (isControlled && !pdfGestureRef.current) return;
      const starts = collectSheetStarts();
      if (starts.size === 0) return;
      const maxScroll = Math.max(0, root.scrollHeight - root.clientHeight);
      const best = pageAtReadingLine(starts, root.scrollTop, undefined, maxScroll);
      if (best >= 1) emitPage(best);
    };

    const onScroll = () => {
      if (followRafRef.current != null) cancelAnimationFrame(followRafRef.current);
      followRafRef.current = requestAnimationFrame(() => {
        followRafRef.current = null;
        pickActivePage();
      });
    };

    let endTimer: number | null = null;
    const markGesture = () => {
      if (endTimer != null) {
        window.clearTimeout(endTimer);
        endTimer = null;
      }
      pdfGestureRef.current = true;
      if (holdRef.current) holdRef.current.stickToTop = false;
      onGestureStartRef.current?.();
    };
    const endGesture = () => {
      // Delay clear so rAF scroll pick can run after scrollend.
      if (endTimer != null) window.clearTimeout(endTimer);
      endTimer = window.setTimeout(() => {
        const wasUser = pdfGestureRef.current;
        if (wasUser) pickActivePage();
        pdfGestureRef.current = false;
        onGestureEndRef.current?.();
        endTimer = null;
        if (wasUser) scheduleAlign();
      }, 120);
    };

    root.addEventListener('scroll', onScroll, { passive: true });
    root.addEventListener('wheel', markGesture, { passive: true });
    root.addEventListener('pointerdown', markGesture, { passive: true });
    root.addEventListener('scrollend', endGesture);
    root.addEventListener('pointerup', endGesture, { passive: true });

    return () => {
      root.removeEventListener('scroll', onScroll);
      root.removeEventListener('wheel', markGesture);
      root.removeEventListener('pointerdown', markGesture);
      root.removeEventListener('scrollend', endGesture);
      root.removeEventListener('pointerup', endGesture);
      if (followRafRef.current != null) cancelAnimationFrame(followRafRef.current);
    };
  }, [numPages, emitPage, scale, collectSheetStarts, isControlled, scheduleAlign]);

  // Keyboard: PageUp/Down, Arrows — only when PDF pane focused or hovered
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.target as HTMLElement | null)?.closest?.('input,textarea,[contenteditable]')) {
        return;
      }
      if (e.key === 'o' || e.key === 'O') {
        if (overlayDisabled) return;
        const root = scrollRef.current;
        const pdfActive =
          root?.contains(document.activeElement) || root?.matches(':hover');
        if (!pdfActive) return;
        setOverlayOn((v) => !v);
        return;
      }
      const root = scrollRef.current;
      if (!root) return;
      const pdfFocused =
        root.contains(document.activeElement) || root.matches(':hover');
      if (!pdfFocused) return;

      const active = displayPage;
      if (e.key === 'PageDown' || e.key === 'ArrowDown') {
        e.preventDefault();
        onGestureStartRef.current?.();
        const next = Math.min(numPages, active + 1);
        scrollToPage(next);
        emitPage(next);
      } else if (e.key === 'PageUp' || e.key === 'ArrowUp') {
        e.preventDefault();
        onGestureStartRef.current?.();
        const prev = Math.max(1, active - 1);
        scrollToPage(prev);
        emitPage(prev);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [overlayDisabled, numPages, displayPage, scrollToPage, emitPage]);

  const [urlOk, setUrlOk] = useState<boolean | null>(null);
  const [probeError, setProbeError] = useState<string | null>(null);
  const [resolvedFile, setResolvedFile] = useState<DocumentProps['file'] | null>(
    null,
  );
  const [authRetryKey, setAuthRetryKey] = useState(0);

  // Auth-enabled demo: bare download URLs 401 without Bearer. Fetch with
  // buildHeaders → in-memory bytes (same pattern as AuthenticatedMarkdownImage).
  useEffect(() => {
    let cancelled = false;
    const ac = new AbortController();

    const sourceUrl = extractPdfSourceUrl(file);
    Promise.resolve().then(async () => {
      if (!file) {
        if (!cancelled) {
          setResolvedFile(null);
          setUrlOk(true);
          setProbeError(null);
        }
        return;
      }

      if (!cancelled) {
        setUrlOk(null);
        setProbeError(null);
        setResolvedFile(null);
        setIsLoading(true);
      }

      // Already-local sources (bytes / non-API URLs) pass through.
      if (
        !sourceUrl ||
        !isApiProtectedPdfUrl(sourceUrl) ||
        sourceUrl.startsWith('blob:')
      ) {
        if (!cancelled) {
          setResolvedFile(file as DocumentProps['file']);
          setUrlOk(true);
        }
        return;
      }

      try {
        const dataFile = await fetchAuthenticatedPdfData(sourceUrl, ac.signal);
        if (cancelled) return;
        setResolvedFile(dataFile);
        setUrlOk(true);
        setProbeError(null);
      } catch (err) {
        if (cancelled || ac.signal.aborted) return;
        const message =
          err instanceof Error
            ? err.message
            : 'ResponseException: Unexpected server response';
        setProbeError(message);
        setUrlOk(false);
        setIsLoading(false);
      }
    });

    return () => {
      cancelled = true;
      ac.abort();
    };
  }, [file, authRetryKey]);

  const handleLoadSuccess = useCallback(
    ({ numPages: n }: { numPages: number }) => {
      setNumPages(n);
      setIsLoading(false);
      setError(null);
      onLoadSuccess?.(n);
    },
    [onLoadSuccess],
  );

  const handleLoadError = useCallback(
    (err: Error) => {
      setError(err.message || 'Failed to load PDF');
      setIsLoading(false);
      onLoadError?.(err);
    },
    [onLoadError],
  );

  const goToPreviousPage = useCallback(() => {
    onGestureStartRef.current?.();
    const prev = Math.max(1, displayPage - 1);
    scrollToPage(prev);
    emitPage(prev);
    window.setTimeout(() => {
      onGestureEndRef.current?.();
    }, 300);
  }, [displayPage, scrollToPage, emitPage]);

  const goToNextPage = useCallback(() => {
    onGestureStartRef.current?.();
    const next = Math.min(numPages, displayPage + 1);
    scrollToPage(next);
    emitPage(next);
    window.setTimeout(() => {
      onGestureEndRef.current?.();
    }, 300);
  }, [numPages, displayPage, scrollToPage, emitPage]);

  const zoomIn = useCallback(() => setScale((prev) => Math.min(3.0, prev + 0.25)), []);
  const zoomOut = useCallback(() => setScale((prev) => Math.max(0.5, prev - 0.25)), []);
  const toggleFullWidth = useCallback(() => setIsFullWidth((prev) => !prev), []);

  const useWindowing = numPages > WINDOW_THRESHOLD;
  const pageList = useMemo(() => {
    if (numPages < 1) return [];
    return Array.from({ length: numPages }, (_, i) => i + 1);
  }, [numPages]);

  const placeholderHeight = Math.max(200, Math.round(basePageHeight * scale));

  if (!file) {
    return (
      <div className="flex items-center justify-center p-8 text-muted-foreground">
        {t('documents.viewer.noFile', 'No PDF file selected')}
      </div>
    );
  }

  const displayError = error ?? probeError;
  if (displayError) {
    return (
      <PDFErrorState
        error={displayError}
        onRetry={() => {
          setError(null);
          setProbeError(null);
          setUrlOk(null);
          setResolvedFile(null);
          setAuthRetryKey((k) => k + 1);
        }}
      />
    );
  }

  if (urlOk === null || !resolvedFile) {
    return (
      <div className={cn('flex flex-col h-full min-h-0', className)}>
        <PDFLoadingSkeleton />
      </div>
    );
  }

  const documentFile: DocumentProps['file'] = resolvedFile;

  return (
    <div
      data-testid="pdf-viewer"
      className={cn('flex flex-col h-full min-h-0', className)}
      tabIndex={0}
    >
      {showToolbar && (
        <div className="flex items-center justify-between gap-2 p-2 border-b bg-muted/30">
          <div className="flex items-center gap-1">
            <Button
              variant="ghost"
              size="icon"
              className="h-8 w-8"
              onClick={goToPreviousPage}
              disabled={displayPage <= 1 || isLoading}
              title={t('documents.viewer.previousPage', 'Previous page')}
              data-testid="pdf-prev-page"
            >
              <ChevronLeft className="h-4 w-4" />
            </Button>
            <span
              className="text-sm text-muted-foreground min-w-[80px] text-center"
              data-testid="pdf-page-indicator"
              data-page={displayPage}
            >
              {isLoading ? '...' : `${displayPage} / ${numPages}`}
            </span>
            <Button
              variant="ghost"
              size="icon"
              className="h-8 w-8"
              onClick={goToNextPage}
              disabled={displayPage >= numPages || isLoading}
              title={t('documents.viewer.nextPage', 'Next page')}
              data-testid="pdf-next-page"
            >
              <ChevronRight className="h-4 w-4" />
            </Button>
          </div>

          <div className="flex items-center gap-1">
            <Button
              variant="ghost"
              size="icon"
              className="h-8 w-8"
              onClick={zoomOut}
              disabled={scale <= 0.5 || isLoading}
              title={t('documents.viewer.zoomOut', 'Zoom out')}
            >
              <ZoomOut className="h-4 w-4" />
            </Button>
            <span className="text-sm text-muted-foreground min-w-[50px] text-center">
              {Math.round(scale * 100)}%
            </span>
            <Button
              variant="ghost"
              size="icon"
              className="h-8 w-8"
              onClick={zoomIn}
              disabled={scale >= 3.0 || isLoading}
              title={t('documents.viewer.zoomIn', 'Zoom in')}
            >
              <ZoomIn className="h-4 w-4" />
            </Button>
            <Button
              variant="ghost"
              size="icon"
              className="h-8 w-8"
              onClick={toggleFullWidth}
              title={
                isFullWidth
                  ? t('documents.viewer.fitWidth', 'Fit width')
                  : t('documents.viewer.fullWidth', 'Full width')
              }
            >
              {isFullWidth ? (
                <Minimize2 className="h-4 w-4" />
              ) : (
                <Maximize2 className="h-4 w-4" />
              )}
            </Button>
            {documentId ? (
              <Tooltip>
                <TooltipTrigger asChild>
                  <span className="inline-flex">
                    <Button
                      variant={overlayOn ? 'secondary' : 'ghost'}
                      size="sm"
                      className="h-8 px-2 text-xs"
                      data-testid="pdf-layout-toggle"
                      aria-pressed={overlayOn}
                      disabled={overlayDisabled}
                      onClick={() => {
                        if (overlayDisabled) return;
                        setOverlayOn((v) => !v);
                      }}
                      title={overlayHint}
                    >
                      {t('documents.viewer.layout.toggle', 'Layout')}
                    </Button>
                  </span>
                </TooltipTrigger>
                <TooltipContent>{overlayHint}</TooltipContent>
              </Tooltip>
            ) : null}
          </div>
        </div>
      )}
      {overlayOn && documentId && !overlayDisabled ? (
        <div className="flex flex-wrap items-center gap-1 px-2 py-1 border-b bg-muted/20 text-xs">
          {(
            [
              ['figures', t('documents.viewer.layout.chipFigures', 'Figures')],
              ['charts', t('documents.viewer.layout.chipCharts', 'Charts')],
              ['tables', t('documents.viewer.layout.chipTables', 'Tables')],
              ['paragraphs', t('documents.viewer.layout.chipParagraphs', 'Paragraphs')],
              ['columns', t('documents.viewer.layout.chipColumns', 'Columns')],
              ['noise', t('documents.viewer.layout.chipNoise', 'Noise')],
            ] as const
          ).map(([key, label]) => (
            <Button
              key={key}
              variant={chips[key] ? 'secondary' : 'ghost'}
              size="sm"
              className="h-6 px-2 text-xs"
              data-testid={`pdf-layout-chip-${key}`}
              onClick={() => setChips((c) => ({ ...c, [key]: !c[key] }))}
            >
              {label}
            </Button>
          ))}
        </div>
      ) : null}

      <div
        ref={scrollRef}
        className={cn(
          'flex-1 min-h-0 overflow-y-auto overflow-x-hidden',
          'bg-muted/10',
        )}
        style={{
          ...(height ? { height: `${height}px` } : {}),
          WebkitOverflowScrolling: 'touch',
        }}
        data-testid="pdf-scroll-container"
      >
        <div ref={stackRef} className="flex flex-col items-center gap-4 py-4">
          <Document
            file={documentFile}
            onLoadSuccess={handleLoadSuccess}
            onLoadError={handleLoadError}
            loading={<PDFLoadingSkeleton />}
            className="pdf-document w-full flex flex-col items-center gap-4"
          >
            {pageList.map((n) => {
              const inWindow =
                !useWindowing || Math.abs(n - displayPage) <= WINDOW_RADIUS;
              return (
                <div
                  key={n}
                  ref={(el) => {
                    if (el) sheetRefs.current.set(n, el);
                    else sheetRefs.current.delete(n);
                  }}
                  data-testid="pdf-page-sheet"
                  data-page={n}
                  className="relative shadow-md bg-background"
                  style={
                    !inWindow
                      ? { width: pageBox?.width ?? width ?? 600, height: placeholderHeight }
                      : pageBox && n === displayPage
                        ? { width: pageBox.width, minHeight: pageBox.height }
                        : undefined
                  }
                >
                  {inWindow ? (
                    <Page
                      pageNumber={n}
                      scale={scale}
                      width={isFullWidth ? undefined : width}
                      className="shadow-md"
                      renderTextLayer={n === displayPage}
                      renderAnnotationLayer={n === displayPage}
                      onRenderSuccess={(page) => {
                        if (n === displayPage || !pageBox) {
                          setPageBox({ width: page.width, height: page.height });
                        }
                        if (scale > 0) {
                          setBasePageHeight(page.height / scale);
                        }
                        scheduleAlign();
                      }}
                      loading={
                        <div className="flex items-center justify-center p-8">
                          <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
                        </div>
                      }
                    />
                  ) : (
                    <div
                      className="flex items-center justify-center text-xs text-muted-foreground"
                      style={{ height: placeholderHeight }}
                    >
                      {t('documents.viewer.pagePlaceholder', 'Page {{n}}', { n })}
                    </div>
                  )}
                  {inWindow &&
                  n === displayPage &&
                  overlayOn &&
                  pageBox &&
                  !overlayDisabled ? (
                    <PdfPageOverlay
                      regions={layoutQuery.data?.regions ?? []}
                      chips={chips}
                      empty={
                        layoutQuery.data?.layout_status === 'extracted' &&
                        (layoutQuery.data.regions?.length ?? 0) === 0
                      }
                    />
                  ) : null}
                </div>
              );
            })}
          </Document>
          {bottomSpacer > 0 ? (
            <div
              aria-hidden="true"
              data-testid="pdf-bottom-spacer"
              style={{ height: bottomSpacer, width: 1, flexShrink: 0 }}
            />
          ) : null}
        </div>
      </div>
    </div>
  );
}

export default PDFViewer;
