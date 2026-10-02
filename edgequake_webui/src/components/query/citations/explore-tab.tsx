'use client';

import { Button } from '@/components/ui/button';
import type { QueryContext } from '@/types';
import { Network } from 'lucide-react';
import { useTranslation } from 'react-i18next';

export function ExploreTab({
  entityCount,
  relationshipCount,
  entities,
  onExploreGraph,
}: {
  entityCount: number;
  relationshipCount: number;
  entities?: QueryContext['entities'];
  onExploreGraph?: (entityLabels: string[]) => void;
}) {
  const { t } = useTranslation();

  const handleExploreClick = () => {
    const labels = entities?.map((e) => e.label) || [];
    onExploreGraph?.(labels);
  };

  return (
    <div className="flex flex-col items-center justify-center h-70 sm:h-90 space-y-4">
      <div className="relative">
        <div className="w-20 h-20 rounded-full bg-gradient-to-br from-primary/20 to-primary/5 flex items-center justify-center">
          <Network className="h-8 w-8 text-primary" />
        </div>
        <div className="absolute -top-1 -right-1 w-6 h-6 rounded-full bg-primary text-primary-foreground text-xs font-semibold flex items-center justify-center">
          {entityCount}
        </div>
      </div>
      <div className="text-center space-y-1">
        <p className="text-sm font-semibold">
          {t('query.citations.exploreGraphTitle', 'Explore Knowledge Graph')}
        </p>
        <p className="text-xs text-muted-foreground">
          {t('query.citations.exploreGraphSubtitle', '{{topics}} topics · {{connections}} connections', {
            topics: entityCount,
            connections: relationshipCount,
          })}
        </p>
      </div>
      <Button
        onClick={handleExploreClick}
        className="gap-2"
        size="sm"
        aria-label={t('query.citations.exploreGraphAria', 'Explore graph with {{topics}} topics and {{connections}} connections', {
          topics: entityCount,
          connections: relationshipCount,
        })}
      >
        <Network className="h-4 w-4" aria-hidden="true" />
        {t('query.citations.openGraphExplorer', 'Open Graph Explorer')}
      </Button>
    </div>
  );
}
