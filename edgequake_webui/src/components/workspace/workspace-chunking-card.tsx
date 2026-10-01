/**
 * @module WorkspaceChunkingCard
 * @description Workspace-scoped adaptive/fixed chunking (SPEC-116).
 *
 * @implements SPEC-116 — Workspace Adaptive Chunking
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
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  ACC_FAIR_CHUNK_OVERLAP,
  ACC_FAIR_CHUNK_TOKEN_SIZE,
  formatChunkingBadge,
  parseChunkingMode,
  validateFixedChunkPair,
  type ChunkingMode,
} from '@/constants/chunking-policy';
import { Scissors } from 'lucide-react';
import { useTranslation } from 'react-i18next';

export interface WorkspaceChunkingValue {
  mode: ChunkingMode;
  size: number;
  overlap: number;
}

export interface WorkspaceChunkingCardProps {
  isEditing: boolean;
  /** Used for read-only badge; may be a partial workspace-like object. */
  workspace: {
    chunking_mode?: string | null;
    chunk_token_size?: number | null;
    chunk_overlap_token_size?: number | null;
  };
  /** Controlled edit value; when omitted, derives from workspace. */
  value?: WorkspaceChunkingValue;
  onChange?: (next: WorkspaceChunkingValue) => void;
  disabled?: boolean;
  /** Wizard shell already shows the step title — hide the card heading. */
  variant?: 'settings' | 'wizard';
}

export function workspaceChunkingFromWorkspace(workspace: {
  chunking_mode?: string | null;
  chunk_token_size?: number | null;
  chunk_overlap_token_size?: number | null;
}): WorkspaceChunkingValue {
  return {
    mode: parseChunkingMode(workspace.chunking_mode),
    size: workspace.chunk_token_size ?? ACC_FAIR_CHUNK_TOKEN_SIZE,
    overlap: workspace.chunk_overlap_token_size ?? ACC_FAIR_CHUNK_OVERLAP,
  };
}

export function WorkspaceChunkingCard({
  isEditing,
  workspace,
  value,
  onChange,
  disabled = false,
  variant = 'settings',
}: WorkspaceChunkingCardProps) {
  const { t } = useTranslation();
  const configured = workspaceChunkingFromWorkspace(workspace);
  const current = value ?? configured;
  const validationError =
    current.mode === 'fixed'
      ? validateFixedChunkPair(current.size, current.overlap)
      : null;

  const setMode = (mode: ChunkingMode) => {
    onChange?.({
      mode,
      size: current.size || ACC_FAIR_CHUNK_TOKEN_SIZE,
      overlap: current.overlap || ACC_FAIR_CHUNK_OVERLAP,
    });
  };

  return (
    <Card className="gap-2 py-4" data-testid="workspace-chunking-card">
      <CardHeader className="flex flex-col gap-2 px-4 sm:flex-row sm:items-start sm:justify-between">
        <div className="min-w-0 space-y-1">
          {variant === 'settings' ? (
            <>
              <CardTitle className="flex items-center gap-2 text-base">
                <Scissors className="h-4 w-4 text-indigo-600" />
                {t('workspace.chunking.title', 'Chunking')}
              </CardTitle>
              <CardDescription className="text-xs leading-snug">
                {t(
                  'workspace.chunking.description',
                  'How documents are split into chunks before entity extraction.',
                )}
              </CardDescription>
            </>
          ) : null}
          <p
            className="text-xs text-muted-foreground"
            data-testid="chunking-markdown-pack-hint"
          >
            {t(
              'workspace.chunking.markdownPackHint',
              'Markdown files pack small headings into the token budget so a heading is not its own chunk.',
            )}
          </p>
          <p
            className="text-xs text-muted-foreground"
            data-testid="chunking-pdf-pack-hint"
          >
            {t(
              'workspace.chunking.pdfPackHint',
              'PDF conversions pack headings, figures, and short pages into the token budget so extract is not one job per page.',
            )}
          </p>
          <p
            className="text-xs text-muted-foreground"
            data-testid="chunking-future-only-hint"
          >
            {t(
              'workspace.chunking.futureOnlyHint',
              'Applies to future document ingestions. Use Rebuild Knowledge Graph to re-chunk existing documents.',
            )}
          </p>
        </div>
        {!isEditing ? (
          <Badge
            variant="secondary"
            className="w-fit shrink-0 text-sm"
            data-testid="ws-chunking-value"
          >
            {formatChunkingBadge(configured)}
          </Badge>
        ) : null}
      </CardHeader>
      {isEditing ? (
        <CardContent className="space-y-3 px-4">
          <div className="flex flex-col gap-2 sm:flex-row sm:items-end sm:gap-3">
            <div className="w-full max-w-sm space-y-1.5">
              <Label htmlFor="chunking-mode-select">
                {t('workspace.chunking.mode', 'Mode')}
              </Label>
              <Select
                value={current.mode}
                onValueChange={(v) => setMode(parseChunkingMode(v))}
                disabled={disabled}
              >
                <SelectTrigger
                  id="chunking-mode-select"
                  className="w-full"
                  data-testid="chunking-mode-select"
                >
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="inherit">
                    {t('workspace.chunking.inherit', 'Inherit (fleet env)')}
                  </SelectItem>
                  <SelectItem value="adaptive">
                    {t('workspace.chunking.adaptive', 'Adaptive')}
                  </SelectItem>
                  <SelectItem value="fixed">
                    {t('workspace.chunking.fixed', 'Fixed')}
                  </SelectItem>
                </SelectContent>
              </Select>
            </div>
            <Button
              type="button"
              variant="outline"
              size="sm"
              className="shrink-0"
              disabled={disabled}
              data-testid="chunking-acc-fair-chip"
              onClick={() =>
                onChange?.({
                  mode: 'fixed',
                  size: ACC_FAIR_CHUNK_TOKEN_SIZE,
                  overlap: ACC_FAIR_CHUNK_OVERLAP,
                })
              }
            >
              {t(
                'workspace.chunking.accFairChip',
                'Match LightRAG (Acc fair)',
              )}
            </Button>
          </div>

          {current.mode === 'adaptive' ? (
            <p className="text-xs text-muted-foreground">
              {t(
                'workspace.chunking.adaptiveHelp',
                'Adaptive sizing uses 1200, 800, or 600 tokens by document size (same thresholds as fleet adaptive).',
              )}
            </p>
          ) : null}

          {current.mode === 'fixed' ? (
            <div className="flex flex-wrap gap-3">
              <div className="space-y-1.5">
                <Label htmlFor="chunking-size-input">
                  {t('workspace.chunking.size', 'Chunk size (tokens)')}
                </Label>
                <Input
                  id="chunking-size-input"
                  type="number"
                  min={1}
                  className="w-32"
                  data-testid="chunking-size-input"
                  disabled={disabled}
                  value={current.size}
                  onChange={(e) =>
                    onChange?.({
                      ...current,
                      size: Number(e.target.value) || 0,
                    })
                  }
                />
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="chunking-overlap-input">
                  {t('workspace.chunking.overlap', 'Overlap (tokens)')}
                </Label>
                <Input
                  id="chunking-overlap-input"
                  type="number"
                  min={0}
                  className="w-32"
                  data-testid="chunking-overlap-input"
                  disabled={disabled}
                  value={current.overlap}
                  onChange={(e) =>
                    onChange?.({
                      ...current,
                      overlap: Number(e.target.value) || 0,
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
