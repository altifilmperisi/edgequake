'use client';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  HoverCard,
  HoverCardContent,
  HoverCardTrigger,
} from '@/components/ui/hover-card';
import { ScrollArea } from '@/components/ui/scroll-area';
import { displayEntityLabel } from '@/lib/graph/label-utils';
import type { OnDocumentClick } from '@/lib/citations/types';
import { invokeDocumentClick } from '@/lib/citations/types';
import type { QueryContext } from '@/types';
import {
  Brain,
  ChevronDown,
  ChevronUp,
  ExternalLink,
  FileText,
  Network,
  Sparkles,
} from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';

export function KnowledgeTab({
  entities,
  relationships,
  onEntityClick,
  onDocumentClick,
}: {
  entities: QueryContext['entities'];
  relationships: QueryContext['relationships'];
  onEntityClick?: (entityId: string) => void;
  onDocumentClick?: OnDocumentClick;
}) {
  const { t } = useTranslation();
  const [showAllEntities, setShowAllEntities] = useState(false);
  const [showAllRelationships, setShowAllRelationships] = useState(false);
  const visibleEntities = showAllEntities ? entities : entities?.slice(0, 12);

  const hasContent =
    (entities && entities.length > 0) || (relationships && relationships.length > 0);

  if (!hasContent) {
    return (
      <div className="flex flex-col items-center justify-center h-70 sm:h-90 text-muted-foreground">
        <Brain className="h-8 w-8 mb-2 opacity-50" aria-hidden="true" />
        <p className="text-sm">{t('query.citations.noKnowledge', 'No knowledge extracted')}</p>
      </div>
    );
  }

  return (
    <ScrollArea className="h-70 sm:h-90">
      <div className="space-y-5 pr-4">
        {entities && entities.length > 0 && (
          <div className="space-y-2.5">
            <div className="flex items-center gap-2">
              <Sparkles className="h-3.5 w-3.5 text-primary" aria-hidden="true" />
              <h4 className="text-xs font-semibold text-foreground">
                {t('query.citations.keyTopics', 'Key Topics')}
              </h4>
              <Badge variant="secondary" className="text-xs h-4 px-1.5">
                {entities.length}
              </Badge>
            </div>
            <div className="flex flex-wrap gap-1.5">
              {visibleEntities?.map((entity) => (
                <HoverCard key={entity.id} openDelay={300}>
                  <HoverCardTrigger asChild>
                    <button
                      type="button"
                      className="inline-flex items-center rounded-md border border-transparent bg-secondary text-secondary-foreground cursor-pointer hover:bg-primary/15 hover:text-primary hover:border-primary/30 transition-all duration-200 text-xs py-1 px-2.5 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/50"
                      onClick={() => onEntityClick?.(entity.id)}
                    >
                      {entity.label}
                    </button>
                  </HoverCardTrigger>
                  <HoverCardContent className="w-72" align="start">
                    <div className="space-y-2">
                      <div className="flex items-center justify-between gap-2">
                        <p className="font-medium">{entity.label}</p>
                        <div className="flex items-center gap-1 shrink-0">
                          {entity.entity_type && entity.entity_type !== 'UNKNOWN' && (
                            <Badge variant="secondary" className="text-xs">
                              {entity.entity_type.toLowerCase()}
                            </Badge>
                          )}
                          <Badge variant="outline" className="text-xs">
                            {t('query.citations.matchPercent', '{{pct}}% match', {
                              pct: Math.round(entity.relevance * 100),
                            })}
                          </Badge>
                        </div>
                      </div>
                      {(entity.source_file_path || entity.source_document_id) && (
                        <button
                          type="button"
                          onClick={() =>
                            entity.source_document_id &&
                            invokeDocumentClick(onDocumentClick, {
                              documentId: entity.source_document_id,
                            })
                          }
                          className="text-xs text-primary hover:underline flex items-center gap-1"
                        >
                          <FileText className="h-3 w-3" />
                          {t('query.citations.viewSourceDocument', 'View source document')}
                          <ExternalLink className="h-2.5 w-2.5" />
                        </button>
                      )}
                      <Button
                        variant="outline"
                        size="sm"
                        className="w-full text-xs h-7"
                        onClick={() => onEntityClick?.(entity.id)}
                      >
                        <Network className="h-3 w-3 mr-1.5" />
                        {t('query.citations.exploreInGraph', 'Explore in graph')}
                      </Button>
                    </div>
                  </HoverCardContent>
                </HoverCard>
              ))}
              {entities.length > 12 && !showAllEntities && (
                <button
                  type="button"
                  className="inline-flex items-center rounded-md border border-input bg-background cursor-pointer hover:bg-muted text-xs py-1 px-2.5 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/50"
                  onClick={() => setShowAllEntities(true)}
                >
                  {t('query.citations.moreTopics', '+{{count}} more', {
                    count: entities.length - 12,
                  })}
                </button>
              )}
            </div>
          </div>
        )}

        {relationships && relationships.length > 0 && (
          <div className="space-y-2.5">
            <div className="flex items-center gap-2">
              <Network className="h-3.5 w-3.5 text-primary" />
              <h4 className="text-xs font-semibold text-foreground">
                {t('query.citations.connections', 'Connections')}
              </h4>
              <Badge variant="secondary" className="text-xs h-4 px-1.5">
                {relationships.length}
              </Badge>
            </div>
            <div className="space-y-1">
              {(showAllRelationships ? relationships : relationships?.slice(0, 6))?.map(
                (rel, idx) => (
                  <HoverCard key={idx} openDelay={300}>
                    <HoverCardTrigger asChild>
                      <div className="flex items-center gap-1.5 text-xs p-2 rounded-md hover:bg-muted/60 transition-colors cursor-pointer group">
                        <button
                          type="button"
                          className="font-medium hover:text-primary truncate max-w-[100px] text-left focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-primary/50 rounded-sm"
                          title={displayEntityLabel({
                            label: rel.source_label,
                            id: rel.source,
                            maxLen: 80,
                          })}
                          onClick={(e) => {
                            e.stopPropagation();
                            onEntityClick?.(rel.source);
                          }}
                        >
                          {displayEntityLabel({
                            label: rel.source_label,
                            id: rel.source,
                          })}
                        </button>
                        <span className="text-primary/60 group-hover:text-primary transition-colors">
                          →
                        </span>
                        <Badge variant="outline" className="text-xs px-1.5 h-4 font-normal">
                          {rel.type.toLowerCase().replace(/_/g, ' ')}
                        </Badge>
                        <span className="text-primary/60 group-hover:text-primary transition-colors">
                          →
                        </span>
                        <button
                          type="button"
                          className="font-medium hover:text-primary truncate max-w-[100px] text-left focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-primary/50 rounded-sm"
                          title={displayEntityLabel({
                            label: rel.target_label,
                            id: rel.target,
                            maxLen: 80,
                          })}
                          onClick={(e) => {
                            e.stopPropagation();
                            onEntityClick?.(rel.target);
                          }}
                        >
                          {displayEntityLabel({
                            label: rel.target_label,
                            id: rel.target,
                          })}
                        </button>
                        {rel.relevance > 0.01 && (
                          <span className="ml-auto text-xs text-muted-foreground">
                            {Math.round(rel.relevance * 100)}%
                          </span>
                        )}
                      </div>
                    </HoverCardTrigger>
                    <HoverCardContent className="w-64" align="start">
                      <div className="space-y-2">
                        <p className="text-sm font-medium">
                          {displayEntityLabel({
                            label: rel.source_label,
                            id: rel.source,
                            maxLen: 80,
                          })}{' '}
                          →{' '}
                          {displayEntityLabel({
                            label: rel.target_label,
                            id: rel.target,
                            maxLen: 80,
                          })}
                        </p>
                        <Badge variant="secondary" className="text-xs">
                          {rel.type}
                        </Badge>
                        {(rel.source_file_path || rel.source_document_id) && (
                          <button
                            type="button"
                            onClick={() =>
                              rel.source_document_id &&
                              invokeDocumentClick(onDocumentClick, {
                                documentId: rel.source_document_id,
                              })
                            }
                            className="text-xs text-primary hover:underline flex items-center gap-1"
                          >
                            <FileText className="h-3 w-3" />
                            {t('query.citations.viewSource', 'View source')}
                            <ExternalLink className="h-2.5 w-2.5" />
                          </button>
                        )}
                      </div>
                    </HoverCardContent>
                  </HoverCard>
                ),
              )}
              {relationships && relationships.length > 6 && (
                <button
                  type="button"
                  className="w-full text-xs text-muted-foreground hover:text-foreground flex items-center justify-center gap-1 py-1 rounded hover:bg-muted/40 transition-colors mt-1"
                  onClick={() => setShowAllRelationships((v) => !v)}
                >
                  {showAllRelationships ? (
                    <>
                      <ChevronUp className="h-3 w-3" />
                      {t('query.citations.showLess', 'Show less')}
                    </>
                  ) : (
                    <>
                      <ChevronDown className="h-3 w-3" />
                      {t('query.citations.moreConnections', '+{{count}} more connections', {
                        count: relationships.length - 6,
                      })}
                    </>
                  )}
                </button>
              )}
            </div>
          </div>
        )}
      </div>
    </ScrollArea>
  );
}
