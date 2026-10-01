'use client';

/**
 * @module KgSchemaPreview
 * @description Honest read-only schema preview — real typed edges only (SPEC-114b LAW-114-12).
 */

import { resolveEntityTypeColor } from '@/lib/graph/entity-type-colors';
import { cn } from '@/lib/utils';
import type { RelationEdge } from '@/constants/kg-schema-presets';
import { useTranslation } from 'react-i18next';

export interface KgSchemaPreviewProps {
  entityTypes: string[];
  relationTypes: string[];
  /** Real typed edges — never invent pairings. */
  relationEdges?: RelationEdge[];
  colors?: Record<string, string>;
  /** `sidebar` = tall column for wide wizard layout; `band` = compact strip. */
  layout?: 'band' | 'sidebar';
}

const MAX_ENTITY_PILLS_BAND = 6;
const MAX_ENTITY_PILLS_SIDEBAR = 10;
const MAX_EDGES_BAND = 5;
const MAX_EDGES_SIDEBAR = 8;

export function KgSchemaPreview({
  entityTypes,
  relationTypes,
  relationEdges = [],
  colors = {},
  layout = 'band',
}: KgSchemaPreviewProps) {
  const { t } = useTranslation();
  const maxEntities =
    layout === 'sidebar' ? MAX_ENTITY_PILLS_SIDEBAR : MAX_ENTITY_PILLS_BAND;
  const maxEdges = layout === 'sidebar' ? MAX_EDGES_SIDEBAR : MAX_EDGES_BAND;
  const entities = entityTypes.slice(0, maxEntities);
  const edges = relationEdges.slice(0, maxEdges);
  const moreEntities = Math.max(0, entityTypes.length - entities.length);
  const moreEdges = Math.max(0, relationEdges.length - edges.length);

  return (
    <section
      className={cn(
        'rounded-lg border bg-muted/20 p-3 space-y-2 h-full',
        layout === 'sidebar' && 'lg:flex lg:flex-col',
      )}
      data-testid="kg-schema-preview"
      aria-label={t('kgSchema.previewLabel', 'Visual schema preview')}
    >
      <div className="flex items-center justify-between gap-2 shrink-0">
        <h5 className="text-xs font-medium">
          {t('kgSchema.previewHeading', 'Visual schema')}
        </h5>
        <span className="text-xs text-muted-foreground tabular-nums">
          {entityTypes.length}/{relationTypes.length}/{relationEdges.length}
        </span>
      </div>

      {entityTypes.length === 0 && relationTypes.length === 0 ? (
        <p className="text-xs text-muted-foreground italic">
          {t(
            'kgSchema.previewEmpty',
            'Select a domain or add types to preview the vocabulary.',
          )}
        </p>
      ) : (
        <div
          className={cn(
            'space-y-2 overflow-y-auto min-h-0',
            layout === 'sidebar'
              ? 'max-h-none lg:flex-1 lg:max-h-[min(22rem,50vh)]'
              : 'max-h-[140px]',
          )}
        >
          <div
            className="flex flex-wrap gap-1.5"
            data-testid="kg-schema-preview-entities"
          >
            {entities.map((type) => (
              <span
                key={type}
                className="inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-xs font-mono"
                style={{
                  borderColor: resolveEntityTypeColor(type, colors),
                  backgroundColor: `${resolveEntityTypeColor(type, colors)}22`,
                }}
              >
                <span
                  className="h-1.5 w-1.5 rounded-full shrink-0"
                  style={{
                    backgroundColor: resolveEntityTypeColor(type, colors),
                  }}
                />
                {type}
              </span>
            ))}
            {moreEntities > 0 && (
              <span className="text-xs text-muted-foreground self-center">
                +{moreEntities}
              </span>
            )}
          </div>

          {edges.length > 0 ? (
            <ul
              className={cn(
                'font-mono text-muted-foreground',
                layout === 'sidebar'
                  ? 'space-y-1.5 text-xs leading-snug'
                  : 'space-y-1 text-xs',
                layout === 'band' &&
                  'sm:flex sm:flex-wrap sm:gap-x-3 sm:gap-y-1 sm:space-y-0',
              )}
              data-testid="kg-schema-preview-relations"
            >
              {edges.map((edge, i) => (
                <li key={`${edge.source}-${edge.relation}-${edge.target}-${i}`} className="truncate">
                  <span className="text-foreground/70">{edge.source}</span>
                  <span className="mx-0.5 text-muted-foreground/80">─</span>
                  <span className="text-foreground">{edge.relation}</span>
                  <span className="mx-0.5 text-muted-foreground/80">→</span>
                  <span className="text-foreground/70">{edge.target}</span>
                </li>
              ))}
              {moreEdges > 0 && (
                <li className="italic">+{moreEdges} more</li>
              )}
            </ul>
          ) : (
            <p
              className="text-xs text-muted-foreground italic"
              data-testid="kg-schema-preview-no-edges"
            >
              {relationTypes.length > 0
                ? t(
                    'kgSchema.previewUnconstrainedEndpoints',
                    'Relations listed; endpoints unconstrained (no typed edges).',
                  )
                : t(
                    'kgSchema.previewFreeFormRelations',
                    'Relations: free-form (no allow-list)',
                  )}
            </p>
          )}
        </div>
      )}
    </section>
  );
}

export default KgSchemaPreview;
