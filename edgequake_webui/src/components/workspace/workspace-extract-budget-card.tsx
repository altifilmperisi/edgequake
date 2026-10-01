/**
 * @module WorkspaceExtractBudgetCard
 * @description Workspace-scoped per-response extract caps (SPEC-117).
 *
 * @implements SPEC-117 — Workspace Extract Budget
 */
'use client';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  formatExtractBudgetBadge,
  LIGHTRAG_EXTRACT_MAX_ENTITIES,
  LIGHTRAG_EXTRACT_MAX_RECORDS,
  parseExtractBudgetMode,
  validateExtractBudgetPair,
  type ExtractBudgetMode,
} from '@/constants/extract-budget';
import { Gauge } from 'lucide-react';
import { useTranslation } from 'react-i18next';

export interface WorkspaceExtractBudgetValue {
  mode: ExtractBudgetMode;
  entities: number;
  records: number;
}

export interface WorkspaceExtractBudgetCardProps {
  isEditing: boolean;
  workspace: {
    extract_budget_mode?: string | null;
    extract_max_entities?: number | null;
    extract_max_records?: number | null;
  };
  value?: WorkspaceExtractBudgetValue;
  onChange?: (next: WorkspaceExtractBudgetValue) => void;
  disabled?: boolean;
  /** Wizard shell already shows the step title — hide the card heading. */
  variant?: 'settings' | 'wizard';
}

export function workspaceExtractBudgetFromWorkspace(workspace: {
  extract_budget_mode?: string | null;
  extract_max_entities?: number | null;
  extract_max_records?: number | null;
}): WorkspaceExtractBudgetValue {
  const hasEntities =
    typeof workspace.extract_max_entities === 'number' &&
    workspace.extract_max_entities > 0;
  return {
    mode: parseExtractBudgetMode(workspace.extract_budget_mode, hasEntities),
    entities: workspace.extract_max_entities ?? LIGHTRAG_EXTRACT_MAX_ENTITIES,
    records: workspace.extract_max_records ?? LIGHTRAG_EXTRACT_MAX_RECORDS,
  };
}

export function WorkspaceExtractBudgetCard({
  isEditing,
  workspace,
  value,
  onChange,
  disabled = false,
  variant = 'settings',
}: WorkspaceExtractBudgetCardProps) {
  const { t } = useTranslation();
  const configured = workspaceExtractBudgetFromWorkspace(workspace);
  const current = value ?? configured;
  const validationError =
    current.mode === 'custom'
      ? validateExtractBudgetPair(current.entities, current.records)
      : null;

  return (
    <Card className="gap-2 py-4" data-testid="workspace-extract-budget-card">
      <CardHeader className="flex flex-col gap-2 px-4 sm:flex-row sm:items-start sm:justify-between">
        <div className="min-w-0 space-y-1">
          {variant === 'settings' ? (
            <>
              <CardTitle className="flex items-center gap-2 text-base">
                <Gauge className="h-4 w-4 text-indigo-600" />
                {t('workspace.extractBudget.title', 'Extract budget')}
              </CardTitle>
              <CardDescription className="text-xs leading-snug">
                {t(
                  'workspace.extractBudget.description',
                  'Per-response entity and record caps for LLM extraction (not a global graph quota).',
                )}
              </CardDescription>
            </>
          ) : null}
          <p
            className="text-xs text-muted-foreground"
            data-testid="extract-budget-future-only-hint"
          >
            {t(
              'workspace.extractBudget.futureOnlyHint',
              'Applies to future ingestions. Rebuild knowledge graph to reprocess existing documents. Adaptive chunking + high budget can inflate entity mentions — see Chunking.',
            )}
          </p>
        </div>
        {!isEditing ? (
          <Badge
            variant="secondary"
            className="w-fit shrink-0 text-sm"
            data-testid="ws-extract-budget-value"
          >
            {formatExtractBudgetBadge(configured)}
          </Badge>
        ) : null}
      </CardHeader>
      {isEditing ? (
        <CardContent className="space-y-3 px-4">
          <div className="flex flex-col gap-2" role="radiogroup" aria-label="Extract budget mode">
            <label className="flex items-center gap-2 text-sm">
              <input
                type="radio"
                name="extract-budget-mode"
                value="inherit"
                checked={current.mode === 'inherit'}
                disabled={disabled}
                data-testid="extract-budget-mode-inherit"
                onChange={() =>
                  onChange?.({
                    mode: 'inherit',
                    entities: current.entities,
                    records: current.records,
                  })
                }
              />
              {t(
                'workspace.extractBudget.inherit',
                'Inherit fleet (usually 40 entities / 100 records)',
              )}
            </label>
            <label className="flex items-center gap-2 text-sm">
              <input
                type="radio"
                name="extract-budget-mode"
                value="custom"
                checked={current.mode === 'custom'}
                disabled={disabled}
                data-testid="extract-budget-mode-custom"
                onChange={() =>
                  onChange?.({
                    mode: 'custom',
                    entities: current.entities || LIGHTRAG_EXTRACT_MAX_ENTITIES,
                    records: current.records || LIGHTRAG_EXTRACT_MAX_RECORDS,
                  })
                }
              />
              {t('workspace.extractBudget.custom', 'Custom')}
            </label>
          </div>
          <Button
            type="button"
            variant="outline"
            size="sm"
            disabled={disabled}
            data-testid="extract-budget-preset-lightrag"
            onClick={() =>
              onChange?.({
                mode: 'custom',
                entities: LIGHTRAG_EXTRACT_MAX_ENTITIES,
                records: LIGHTRAG_EXTRACT_MAX_RECORDS,
              })
            }
          >
            {t(
              'workspace.extractBudget.lightragPreset',
              'Match LightRAG (40/100)',
            )}
          </Button>
          {current.mode === 'custom' ? (
            <div className="flex flex-wrap gap-3">
              <div className="space-y-1.5">
                <Label htmlFor="extract-budget-entities">
                  {t('workspace.extractBudget.entities', 'Max entities')}
                </Label>
                <Input
                  id="extract-budget-entities"
                  type="number"
                  min={1}
                  value={current.entities}
                  disabled={disabled}
                  data-testid="extract-budget-entities"
                  onChange={(e) =>
                    onChange?.({
                      ...current,
                      mode: 'custom',
                      entities: Number(e.target.value) || 0,
                    })
                  }
                />
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="extract-budget-records">
                  {t('workspace.extractBudget.records', 'Max records')}
                </Label>
                <Input
                  id="extract-budget-records"
                  type="number"
                  min={1}
                  value={current.records}
                  disabled={disabled}
                  data-testid="extract-budget-records"
                  onChange={(e) =>
                    onChange?.({
                      ...current,
                      mode: 'custom',
                      records: Number(e.target.value) || 0,
                    })
                  }
                />
              </div>
            </div>
          ) : null}
          {validationError ? (
            <p className="text-xs text-destructive" role="alert">
              {validationError}
            </p>
          ) : null}
        </CardContent>
      ) : null}
    </Card>
  );
}
