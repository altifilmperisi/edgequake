/**
 * @module DeletionImpactCard
 * @description Shows a pre-delete impact summary (entities, relationships, chunks).
 *
 * WHY: Users should know what will be removed before confirming a delete.
 * This is a pure display component — no data fetching. The parent dialog
 * owns the data via useDeletionImpact.
 *
 * SHARED ENTITY SEMANTICS (SPEC-050 / EC-2):
 *  - entities_to_remove: exclusive to this document → PERMANENTLY REMOVED
 *  - entities_to_update: shared with other documents → SURVIVES (pruned sources)
 *  - relationships_to_remove: exclusive or endpoint deleted → PERMANENTLY REMOVED
 *  - relationships_to_update: shared with other documents → SURVIVES (pruned sources)
 *  - injected entities (empty source_ids): NEVER touched by document deletion
 *
 * @implements SPEC-050: Impact preview before delete (AC-050-01).
 * @implements SPEC-050/EC-2: Shared entity semantics — updated vs. removed.
 */
'use client';

import { Skeleton } from '@/components/ui/skeleton';
import {
    Tooltip,
    TooltipContent,
    TooltipProvider,
    TooltipTrigger,
} from '@/components/ui/tooltip';
import type { DeletionImpact } from '@/lib/api/edgequake';
import { AlertTriangle, CheckCircle2, Database, FileText, HelpCircle, Network } from 'lucide-react';
import { useTranslation } from 'react-i18next';

interface DeletionImpactCardProps {
  /** Impact data loaded from GET /deletion-impact. Null while loading. */
  impact: DeletionImpact | null;
  /** True while the impact is being fetched. */
  isLoading: boolean;
  /** Error if the fetch failed — shows graceful fallback. */
  error?: Error | null;
}

/**
 * Single metric row inside the impact card.
 */
function ImpactRow({
  icon: Icon,
  label,
  value,
  variant = 'neutral',
  tooltip,
}: {
  icon: React.ElementType;
  label: string;
  value: number;
  variant?: 'neutral' | 'danger' | 'warning' | 'safe';
  tooltip?: string;
}) {
  const colorClass =
    variant === 'danger'
      ? 'text-rose-600 dark:text-rose-400'
      : variant === 'warning'
        ? 'text-amber-600 dark:text-amber-400'
        : variant === 'safe'
          ? 'text-sky-600 dark:text-sky-400'
          : 'text-muted-foreground';

  const row = (
    <div className="flex items-center justify-between gap-3 py-1">
      <div className="flex items-center gap-2">
        <Icon className={`h-3.5 w-3.5 shrink-0 ${colorClass}`} />
        <span className="text-sm text-muted-foreground">{label}</span>
        {tooltip && (
          <HelpCircle className="h-3 w-3 text-muted-foreground/50 shrink-0" />
        )}
      </div>
      <span className={`text-sm font-medium tabular-nums ${colorClass}`}>
        {value.toLocaleString()}
      </span>
    </div>
  );

  if (!tooltip) return row;

  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <div className="cursor-help">{row}</div>
        </TooltipTrigger>
        <TooltipContent side="left" className="max-w-64 text-xs">
          {tooltip}
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}

/**
 * Displays the deletion impact for a single document.
 *
 * Shows loading skeletons while fetching, a graceful error banner if the
 * analysis fails, and the full breakdown once loaded.
 *
 * Two distinct sections:
 *  - "Permanently removed" (red) — entities/relationships with no other sources
 *  - "Will survive" (amber) — entities/relationships shared with other documents
 *    that will persist but with their source list pruned
 */
export function DeletionImpactCard({
  impact,
  isLoading,
  error,
}: DeletionImpactCardProps) {
  const { t } = useTranslation();

  if (isLoading) {
    return (
      <div
        className="rounded-md border bg-muted/30 p-3 space-y-2"
        data-testid="deletion-impact-loading"
      >
        <div className="text-xs font-medium text-muted-foreground uppercase tracking-wide mb-2">
          {t('documents.deleteDialog.impactTitle', 'Impact Analysis')}
        </div>
        <Skeleton className="h-5 w-full" />
        <Skeleton className="h-5 w-4/5" />
        <Skeleton className="h-5 w-3/5" />
      </div>
    );
  }

  if (error || !impact) {
    return (
      <div
        className="rounded-md border border-amber-200 bg-amber-50/50 dark:border-amber-900 dark:bg-amber-950/20 p-3"
        data-testid="deletion-impact-error"
      >
        <div className="flex items-center gap-2 text-amber-700 dark:text-amber-400 text-xs">
          <AlertTriangle className="h-3.5 w-3.5 shrink-0" />
          <span>
            {t(
              'documents.deleteDialog.impactUnavailable',
              'Impact analysis unavailable — you can still proceed with deletion.',
            )}
          </span>
        </div>
      </div>
    );
  }

  const hasRemovals =
    impact.entities_to_remove > 0 || impact.relationships_to_remove > 0;
  const hasSurvivors =
    impact.entities_to_update > 0 || impact.relationships_to_update > 0;
  const hasNoGraphImpact = !hasRemovals && !hasSurvivors;

  return (
    <div
      className="rounded-md border bg-muted/30 p-3 space-y-3"
      data-testid="deletion-impact-card"
    >
      {/* Header */}
      <div className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
        {t('documents.deleteDialog.impactTitle', 'Knowledge Graph Impact')}
      </div>

      {/* Chunks row (always shown) */}
      <ImpactRow
        icon={Database}
        label={t('documents.deleteDialog.impactChunks', '{{n}} chunks & embeddings', {
          n: impact.chunks_to_delete,
        })}
        value={impact.chunks_to_delete}
        variant="neutral"
        tooltip={t(
          'documents.deleteDialog.impactChunksTooltip',
          'Document chunks and their vector embeddings stored in the knowledge base.',
        )}
      />

      {/* Permanently removed section */}
      {hasRemovals && (
        <div className="space-y-0.5">
          <div className="text-xs font-semibold uppercase tracking-wider text-rose-500 dark:text-rose-400 mt-1 mb-0.5 flex items-center gap-1">
            <span className="h-1 w-1 rounded-full bg-rose-500 inline-block" />
            {t('documents.deleteDialog.sectionRemoved', 'Permanently removed')}
          </div>

          {impact.entities_to_remove > 0 && (
            <ImpactRow
              icon={Network}
              label={t('documents.deleteDialog.impactEntitiesRemoved', '{{n}} entities', {
                n: impact.entities_to_remove,
              })}
              value={impact.entities_to_remove}
              variant="danger"
              tooltip={t(
                'documents.deleteDialog.impactEntitiesRemovedTooltip',
                'These entities exist ONLY in this document. They will be permanently deleted from the knowledge graph.',
              )}
            />
          )}

          {impact.relationships_to_remove > 0 && (
            <ImpactRow
              icon={FileText}
              label={t('documents.deleteDialog.impactRelationshipsRemoved', '{{n}} relationships', {
                n: impact.relationships_to_remove,
              })}
              value={impact.relationships_to_remove}
              variant="danger"
              tooltip={t(
                'documents.deleteDialog.impactRelationshipsRemovedTooltip',
                'These relationships exist ONLY in this document, or connect to entities that will be removed. They will be permanently deleted.',
              )}
            />
          )}
        </div>
      )}

      {/* Shared / survives section */}
      {hasSurvivors && (
        <div className="space-y-0.5">
          <div className="text-xs font-semibold uppercase tracking-wider text-amber-500 dark:text-amber-400 mt-1 mb-0.5 flex items-center gap-1">
            <span className="h-1 w-1 rounded-full bg-amber-500 inline-block" />
            {t('documents.deleteDialog.sectionSurvives', 'Survive (shared with other documents)')}
          </div>

          {impact.entities_to_update > 0 && (
            <ImpactRow
              icon={Network}
              label={t('documents.deleteDialog.impactEntitiesUpdated', '{{n}} entities will survive', {
                n: impact.entities_to_update,
              })}
              value={impact.entities_to_update}
              variant="warning"
              tooltip={t(
                'documents.deleteDialog.impactEntitiesUpdatedTooltip',
                'These entities are shared with other documents. They will REMAIN in the graph but with fewer supporting sources. Their descriptions will be updated to reflect only the remaining documents.',
              )}
            />
          )}

          {impact.relationships_to_update > 0 && (
            <ImpactRow
              icon={FileText}
              label={t('documents.deleteDialog.impactRelationshipsUpdated', '{{n}} relationships will survive', {
                n: impact.relationships_to_update,
              })}
              value={impact.relationships_to_update}
              variant="warning"
              tooltip={t(
                'documents.deleteDialog.impactRelationshipsUpdatedTooltip',
                'These relationships connect entities that also appear in other documents. They will REMAIN in the graph with updated source references.',
              )}
            />
          )}

          {/* Clarifying banner for shared entities */}
          <div className="flex items-start gap-1.5 mt-1.5 rounded px-2 py-1.5 bg-sky-50/60 dark:bg-sky-950/20 border border-sky-200/60 dark:border-sky-900/40">
            <CheckCircle2 className="h-3 w-3 text-sky-500 shrink-0 mt-0.5" />
            <p className="text-xs text-sky-700 dark:text-sky-400">
              {t(
                'documents.deleteDialog.sharedEntityNote',
                'Shared entities and relationships will NOT be deleted — they survive with evidence from other documents.',
              )}
            </p>
          </div>
        </div>
      )}

      {/* No graph impact */}
      {hasNoGraphImpact && (
        <div className="py-1 text-sm text-muted-foreground">
          {t(
            'documents.deleteDialog.impactNoGraph',
            'No graph entities or relationships will be affected.',
          )}
        </div>
      )}
    </div>
  );
}
