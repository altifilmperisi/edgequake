/**
 * @module WorkspaceRelationTypesCard
 * @description Read-only relation type vocabulary for workspace settings (SPEC-114).
 */
'use client';

import { Badge } from '@/components/ui/badge';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import type { Workspace } from '@/types';
import { GitBranch } from 'lucide-react';
import { useTranslation } from 'react-i18next';

export interface WorkspaceRelationTypesCardProps {
  workspace: Workspace;
}

export function WorkspaceRelationTypesCard({
  workspace,
}: WorkspaceRelationTypesCardProps) {
  const { t } = useTranslation();
  const types = workspace.relation_types ?? [];
  const edges = workspace.relation_edges ?? [];
  const preset = workspace.kg_schema_preset;

  return (
    <Card className="gap-2 py-4 h-full" data-testid="workspace-relation-types-card">
      <CardHeader className="px-4 pb-0 gap-1">
        <CardTitle className="flex items-center gap-2 text-base">
          <GitBranch className="h-4 w-4 text-teal-700" />
          {t('relationTypes.title', 'Relation Types')}
        </CardTitle>
        <CardDescription className="text-xs leading-snug">
          {t(
            'relationTypes.futureOnlyHint',
            'Applies to future document ingestions. Use Rebuild Knowledge Graph to refresh existing edges.',
          )}
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-2 px-4">
        {preset ? (
          <p className="text-xs text-muted-foreground" data-testid="kg-schema-preset-badge">
            {t('kgSchema.domainLabel', 'Domain')}:{' '}
            <span className="font-medium capitalize">{preset}</span>
          </p>
        ) : null}
        {types.length > 0 ? (
          <>
            <div className="flex flex-wrap gap-1.5">
              {types.map((type) => (
                <Badge
                  key={type}
                  variant="secondary"
                  className="text-xs font-mono"
                  data-testid={`ws-relation-type-${type}`}
                >
                  {type}
                </Badge>
              ))}
            </div>
            <p
              className="text-xs text-muted-foreground"
              data-testid="relation-types-strict-status"
            >
              {workspace.relation_types_strict !== false
                ? t(
                    'relationTypes.strictOn',
                    'Strict limit: on (unknown relations remapped)',
                  )
                : t(
                    'relationTypes.strictOff',
                    'Strict limit: off (free-form relations allowed)',
                  )}
            </p>
          </>
        ) : (
          <p
            className="rounded-md border border-dashed bg-muted/20 px-2.5 py-2 text-xs text-muted-foreground"
            data-testid="relation-types-free-form"
          >
            {t(
              'relationTypes.freeFormHint',
              'No relation allow-list — the model may use free-form relationship labels.',
            )}
          </p>
        )}
        {edges.length > 0 ? (
          <div className="space-y-1" data-testid="workspace-relation-edges">
            <p className="text-xs font-medium">
              {t('kgSchema.typedEdgesHeading', 'Typed edges')}{' '}
              <span className="text-muted-foreground font-normal">
                ({edges.length})
              </span>
            </p>
            <ul className="space-y-0.5 font-mono text-xs text-muted-foreground">
              {edges.slice(0, 6).map((e) => (
                <li
                  key={`${e.source}-${e.relation}-${e.target}`}
                  data-testid={`ws-relation-edge-${e.source}-${e.relation}-${e.target}`}
                >
                  <span className="text-foreground/80">{e.source}</span>
                  {' — '}
                  <span className="text-foreground">{e.relation}</span>
                  {' → '}
                  <span className="text-foreground/80">{e.target}</span>
                </li>
              ))}
              {edges.length > 6 ? (
                <li className="italic">+{edges.length - 6} more</li>
              ) : null}
            </ul>
          </div>
        ) : (
          <p
            className="rounded-md border border-dashed bg-muted/20 px-2.5 py-2 text-xs text-muted-foreground"
            data-testid="relation-edges-unconstrained"
          >
            {t(
              'kgSchema.edgesUnconstrainedHint',
              'No typed edges — endpoints are unconstrained for listed relations.',
            )}
          </p>
        )}
      </CardContent>
    </Card>
  );
}
