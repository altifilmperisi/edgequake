/**
 * Presentational ingestion activity banner (DRY surface for documents page).
 *
 * @implements SPEC-045 — honest ingestion UX (working / queued / stuck)
 * @implements SPEC-048 — stage-specific microcopy + progress label gating
 * @implements SPEC-122 — bulk admit≠ready physics + concurrency lane hint
 */
'use client';

import { Button } from '@/components/ui/button';
import { Progress } from '@/components/ui/progress';
import { getQueueMetrics } from '@/lib/api/edgequake';
import {
  concurrencyLaneHint,
  shouldShowBulkBanner,
} from '@/lib/documents/admit-copy';
import {
  getDocumentDisplayStatus,
  isTerminalStatus,
} from '@/lib/documents/status-domain';
import {
  buildIngestionAlertHeadline,
  ingestionAlertContainerClass,
  ingestionAlertDetailClass,
  ingestionAlertTitleClass,
} from '@/lib/pipeline/ingestion-alert-presenter';
import { resolveBannerProgressMeta } from '@/lib/pipeline/graph-merge-progress';
import { translateIngestionDetail } from '@/lib/pipeline/ingestion-user-messages';
import {
  buildIngestionRunView,
  formatRunHeadline,
  selectPrimaryRun,
  buildIngestionRunViews,
} from '@/lib/pipeline/ingestion-run-view';
import {
  resolvePipelineUiState,
  summarizePipelineDocuments,
} from '@/lib/pipeline/pipeline-document-state';
import type { Document, PipelineStatus, QueueMetrics } from '@/types';
import { cn } from '@/lib/utils';
import { useQuery } from '@tanstack/react-query';
import { AlertTriangle, Clock, Loader2 } from 'lucide-react';
import Link from 'next/link';
import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';

export interface IngestionAlertBannerProps {
  pipelineStatus: PipelineStatus;
  documents: Document[];
  onOpenDetails: () => void;
  onReprocessStuck?: (documents: Document[]) => void;
  isReprocessing?: boolean;
}

export function IngestionAlertBanner({
  pipelineStatus,
  documents,
  onOpenDetails,
  onReprocessStuck,
  isReprocessing = false,
}: IngestionAlertBannerProps) {
  const { t } = useTranslation();

  const summary = useMemo(
    () => summarizePipelineDocuments(documents),
    [documents],
  );

  const pipelineUi = useMemo(
    () => resolvePipelineUiState(documents, pipelineStatus),
    [documents, pipelineStatus],
  );

  const bulkCounts = useMemo(() => {
    const pending = summary.waitingCount;
    const processing = summary.activeCount;
    const completed = documents.filter((doc) => {
      const status = getDocumentDisplayStatus(doc);
      return (
        status === 'completed' ||
        status === 'indexed' ||
        (isTerminalStatus(status) &&
          status !== 'failed' &&
          status !== 'cancelled' &&
          status !== 'partial_failure' &&
          status !== 'delete_failed' &&
          status !== 'dead_letter')
      );
    }).length;
    return { pending, processing, completed };
  }, [documents, summary.activeCount, summary.waitingCount]);

  const { data: queueMetrics } = useQuery<QueueMetrics>({
    queryKey: ['queue-metrics', 'documents-banner'],
    queryFn: () => getQueueMetrics(),
    refetchInterval: pipelineUi.showPipelineIndicator ? 5000 : false,
    enabled: pipelineUi.showPipelineIndicator,
    staleTime: 4000,
  });

  const showBulkPhysics = shouldShowBulkBanner(
    bulkCounts.pending,
    bulkCounts.processing,
  );

  const laneHint = useMemo(() => {
    const k = queueMetrics?.max_tasks_per_tenant;
    const english = concurrencyLaneHint(k);
    if (!english) return null;
    if (k === 1) {
      return t(
        'documents.upload.bulk.serialHint',
        'Processing one document at a time',
      );
    }
    return (
      t('documents.upload.bulk.parallelHint', { k }) ||
      `Processing up to ${k} documents in parallel`
    );
  }, [queueMetrics?.max_tasks_per_tenant, t]);

  const detailDocs =
    pipelineUi.alertMode === 'stuck'
      ? pipelineUi.stuckDocs
      : pipelineUi.alertMode === 'queued'
        ? summary.waitingDocs
        : [...summary.activeDocs, ...summary.waitingDocs];

  const primaryRun = useMemo(() => {
    const map = buildIngestionRunViews(detailDocs);
    return selectPrimaryRun(map);
  }, [detailDocs]);

  const progressMeta = useMemo(
    () =>
      pipelineUi.isActivelyProcessing
        ? resolveBannerProgressMeta(summary.activeDocs)
        : undefined,
    [pipelineUi.isActivelyProcessing, summary.activeDocs],
  );

  if (!pipelineUi.showPipelineIndicator) {
    return null;
  }

  const baseHeadline = buildIngestionAlertHeadline(pipelineUi, t);
  // SPEC-048: stage-specific title when working (not generic "Processing N")
  const headlineText =
    pipelineUi.alertMode === 'working' && primaryRun
      ? formatRunHeadline(primaryRun)
      : baseHeadline.text;

  const messageContext =
    pipelineUi.alertMode === 'stuck'
      ? 'stuck'
      : pipelineUi.alertMode === 'queued'
        ? 'queued'
        : 'active';

  const hintText =
    pipelineUi.alertMode === 'stuck'
      ? t(
          'pipeline.stuckHint',
          'No worker is processing these documents. Reprocess to create a new task.',
        )
      : pipelineUi.alertMode === 'queued'
        ? t(
            'pipeline.waitingHint',
            'Workers are idle but documents are queued. Processing will resume automatically.',
          )
        : null;

  const progressLabel = progressMeta
    ? t(
        progressMeta.labelKey,
        progressMeta.labelKey === 'pipeline.graphMergeProgress'
          ? 'Graph save progress'
          : progressMeta.labelKey === 'pipeline.extractionProgress'
            ? 'Extraction progress'
            : progressMeta.labelKey === 'pipeline.embeddingProgress'
              ? 'Embedding progress'
              : progressMeta.labelKey === 'pipeline.conversionProgress'
                ? 'Conversion progress'
                : 'Stage progress',
      )
    : null;

  return (
    <div
      data-testid="ingestion-status-banner"
      data-ingestion-mode={baseHeadline.dataTestId}
      className={cn(
        'flex flex-col gap-1.5 px-3 py-2 border rounded-lg cursor-pointer transition-colors overflow-hidden',
        ingestionAlertContainerClass(baseHeadline.variant),
      )}
      onClick={onOpenDetails}
      role="button"
      tabIndex={0}
      onKeyDown={(e) => e.key === 'Enter' && onOpenDetails()}
    >
      {showBulkPhysics ? (
        <p
          data-testid="spec122-bulk-ingest-banner"
          aria-live="polite"
          className={cn(
            'text-xs font-medium min-w-0 truncate',
            ingestionAlertDetailClass(baseHeadline.variant),
          )}
        >
          {t('documents.upload.bulk.banner', {
            processing: bulkCounts.processing,
            pending: bulkCounts.pending,
            completed: bulkCounts.completed,
          }) ||
            `Processing ${bulkCounts.processing} · ${bulkCounts.pending} queued · ${bulkCounts.completed} completed`}
          {laneHint ? ` — ${laneHint}` : ''}
          <Link
            href="/pipeline"
            className="ml-2 underline underline-offset-2 hover:opacity-80"
            data-testid="spec122-queue-metrics-link"
            onClick={(event) => event.stopPropagation()}
          >
            {t('documents.upload.bulk.queueMetricsLink', 'Queue metrics')}
          </Link>
        </p>
      ) : null}
      <div className="flex items-start gap-3 min-w-0">
        {baseHeadline.showAlert ? (
          <AlertTriangle className="h-4 w-4 text-rose-600 dark:text-rose-400 shrink-0 mt-0.5" />
        ) : baseHeadline.showClock ? (
          <Clock className="h-4 w-4 text-amber-600 dark:text-amber-400 shrink-0 mt-0.5" />
        ) : (
          <Loader2 className="h-4 w-4 text-sky-600 dark:text-sky-400 animate-spin shrink-0 mt-0.5" />
        )}
        <div className="flex-1 min-w-0 space-y-1">
          <div className="flex items-start justify-between gap-2 min-w-0">
            <div className="min-w-0 space-y-0.5">
              <p
                data-testid={baseHeadline.dataTestId}
                className={cn(
                  'text-sm font-medium min-w-0 truncate',
                  ingestionAlertTitleClass(baseHeadline.variant),
                )}
              >
                {headlineText}
              </p>
              {pipelineUi.alertMode === 'working' && primaryRun ? (
                <p
                  data-testid="spec048-banner-run-detail"
                  data-stage={primaryRun.stage}
                  className={cn(
                    'text-xs truncate',
                    ingestionAlertDetailClass(baseHeadline.variant),
                  )}
                >
                  {primaryRun.filename}
                  {pipelineUi.activeDocCount > 1
                    ? ` · +${pipelineUi.activeDocCount - 1} more`
                    : ''}
                </p>
              ) : null}
            </div>
            <div className="flex items-center gap-2 shrink-0">
              {pipelineUi.isStuck && onReprocessStuck && (
                <Button
                  type="button"
                  size="sm"
                  variant="outline"
                  data-testid="ingestion-banner-reprocess"
                  className="h-7 text-xs border-rose-300 text-rose-700 hover:bg-rose-100 dark:border-rose-700 dark:text-rose-300"
                  disabled={isReprocessing}
                  onClick={(event) => {
                    event.stopPropagation();
                    onReprocessStuck(pipelineUi.stuckDocs);
                  }}
                >
                  {isReprocessing ? (
                    <Loader2 className="h-3 w-3 animate-spin" />
                  ) : (
                    t('pipeline.reprocessStuck', 'Reprocess')
                  )}
                </Button>
              )}
              <span
                className={cn(
                  'text-xs whitespace-nowrap',
                  ingestionAlertDetailClass(baseHeadline.variant),
                )}
              >
                {t('pipeline.clickForDetails', 'Details →')}
              </span>
            </div>
          </div>

          {hintText && (
            <p className={cn('text-xs', ingestionAlertDetailClass(baseHeadline.variant))}>
              {hintText}
            </p>
          )}

          {pipelineUi.alertMode !== 'working' && detailDocs.length > 0 && (
            <div className="space-y-0.5 min-w-0">
              {detailDocs.slice(0, 2).map((doc) => {
                const run = buildIngestionRunView(doc);
                const detail = run
                  ? formatRunHeadline(run)
                  : translateIngestionDetail(doc, messageContext);
                return (
                  <p
                    key={doc.id}
                    data-testid="spec048-banner-run-detail"
                    data-stage={run?.stage}
                    className={cn(
                      'text-xs line-clamp-2 break-words',
                      ingestionAlertDetailClass(baseHeadline.variant),
                    )}
                  >
                    {detail}
                  </p>
                );
              })}
            </div>
          )}

          {progressMeta && progressLabel && (
            <div
              className="space-y-1 pt-0.5"
              data-testid="ingestion-banner-progress"
              data-progress-label={progressMeta.labelKey}
            >
              <div className="flex items-center justify-between gap-2 text-xs">
                <span className={ingestionAlertDetailClass(baseHeadline.variant)}>
                  {progressLabel}
                </span>
                <span
                  className={cn(
                    'tabular-nums font-medium',
                    ingestionAlertDetailClass(baseHeadline.variant),
                  )}
                >
                  {Math.round(progressMeta.progress01 * 100)}%
                </span>
              </div>
              <Progress
                value={progressMeta.progress01 * 100}
                className="h-1.5 bg-black/5 dark:bg-white/10 [&_[data-slot=progress-indicator]]:bg-sky-500"
              />
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

export default IngestionAlertBanner;
