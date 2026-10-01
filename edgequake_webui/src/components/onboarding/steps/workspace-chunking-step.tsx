'use client';

/**
 * SPEC-116 — Dedicated wizard step for workspace chunking (SRP).
 * Reuses {@link WorkspaceChunkingCard}; draft mapping via draft-chunking SSOT.
 * Step title lives on WizardShell — card uses variant="wizard".
 */

import {
  WorkspaceChunkingCard,
  type WorkspaceChunkingValue,
} from '@/components/workspace/workspace-chunking-card';
import {
  chunkingValueFromDraft,
  draftPatchFromChunkingValue,
} from '@/lib/onboarding/draft-chunking';
import type { WizardDraft } from '@/lib/onboarding/wizard-state';
import { useTranslation } from 'react-i18next';

export interface WorkspaceChunkingStepProps {
  draft: WizardDraft;
  onChange: (patch: Partial<WizardDraft>) => void;
}

export function WorkspaceChunkingStep({
  draft,
  onChange,
}: WorkspaceChunkingStepProps) {
  const { t } = useTranslation();
  const value = chunkingValueFromDraft(draft);

  return (
    <div
      className="mx-auto flex w-full max-w-2xl flex-col gap-4"
      data-testid="wizard-step-chunking"
    >
      <WorkspaceChunkingCard
        variant="wizard"
        isEditing
        workspace={{
          chunking_mode: draft.chunkingMode,
          chunk_token_size: draft.chunkTokenSize,
          chunk_overlap_token_size: draft.chunkOverlapTokenSize,
        }}
        value={value}
        onChange={(next: WorkspaceChunkingValue) =>
          onChange(draftPatchFromChunkingValue(next))
        }
      />

      <aside
        className="rounded-lg border border-dashed bg-muted/20 px-3 py-2.5 text-xs leading-relaxed text-muted-foreground"
        data-testid="wizard-chunking-extract-hint"
      >
        {t(
          'onboarding.chunkingExtractBudgetHint',
          'Next: extract budget caps how many entities each LLM response may emit. Adaptive chunking + a high budget can inflate mentions — tune both deliberately.',
        )}
      </aside>
    </div>
  );
}
