'use client';

/**
 * SPEC-117 — Dedicated wizard step for workspace extract budget (SRP).
 * Reuses {@link WorkspaceExtractBudgetCard}; draft mapping via draft-extract-budget SSOT.
 * Step title lives on WizardShell — card uses variant="wizard".
 */

import {
  WorkspaceExtractBudgetCard,
  type WorkspaceExtractBudgetValue,
} from '@/components/workspace/workspace-extract-budget-card';
import {
  draftPatchFromExtractBudgetValue,
  extractBudgetValueFromDraft,
} from '@/lib/onboarding/draft-extract-budget';
import type { WizardDraft } from '@/lib/onboarding/wizard-state';
import { useTranslation } from 'react-i18next';

export interface WorkspaceExtractBudgetStepProps {
  draft: WizardDraft;
  onChange: (patch: Partial<WizardDraft>) => void;
}

export function WorkspaceExtractBudgetStep({
  draft,
  onChange,
}: WorkspaceExtractBudgetStepProps) {
  const { t } = useTranslation();
  const value = extractBudgetValueFromDraft(draft);

  return (
    <div
      className="mx-auto flex w-full max-w-2xl flex-col gap-4"
      data-testid="wizard-step-extract-budget"
    >
      <WorkspaceExtractBudgetCard
        variant="wizard"
        isEditing
        workspace={{
          extract_budget_mode: draft.extractBudgetMode,
          extract_max_entities: draft.extractMaxEntities,
          extract_max_records: draft.extractMaxRecords,
        }}
        value={value}
        onChange={(next: WorkspaceExtractBudgetValue) =>
          onChange(draftPatchFromExtractBudgetValue(next))
        }
      />

      <aside
        className="rounded-lg border border-dashed bg-muted/20 px-3 py-2.5 text-xs leading-relaxed text-muted-foreground"
        data-testid="wizard-extract-budget-chunking-hint"
      >
        {t(
          'onboarding.extractBudgetChunkingHint',
          'Chunking (previous step) controls how text is split; this step caps how many entities each response may emit. Tune them together — adaptive chunks + a high budget can inflate mentions.',
        )}
      </aside>
    </div>
  );
}
