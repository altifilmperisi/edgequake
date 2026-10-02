/**
 * SPEC-101 — Pure wizard step navigation + validation (unit-tested).
 */

import type { RelationEdge } from '@/constants/kg-schema-presets';
import { isChunkingDraftValid } from '@/lib/onboarding/draft-chunking';
import { isExtractBudgetDraftValid } from '@/lib/onboarding/draft-extract-budget';

export type WizardKind =
  | 'first-run'
  | 'create-tenant'
  | 'create-workspace'
  | 'reconfigure-workspace';

export type WizardStepId =
  | 'admin'
  | 'tenant-basics'
  | 'models'
  | 'workspace-basics'
  | 'document-parsing'
  | 'chunking'
  | 'extract-budget'
  | 'extraction'
  | 'review';

export type PdfParserBackendDraft =
  | 'none'
  | 'vision'
  | 'edgeparse'
  | 'edgeparse-ocr'
  | 'auto';

export interface WizardDraft {
  adminUsername: string;
  adminEmail: string;
  adminPassword: string;
  adminPasswordConfirm: string;
  tenantName: string;
  tenantDescription: string;
  workspaceName: string;
  workspaceSlug: string;
  workspaceDescription: string;
  useServerDefaults: boolean;
  extractionLanguage: string | null;
  /** SPEC-116 — inherit | adaptive | fixed (null ≡ inherit). */
  chunkingMode: 'inherit' | 'adaptive' | 'fixed' | null;
  chunkTokenSize: number;
  chunkOverlapTokenSize: number;
  /** SPEC-117 — inherit | custom (null ≡ inherit). */
  extractBudgetMode: 'inherit' | 'custom' | null;
  extractMaxEntities: number;
  extractMaxRecords: number;
  entityTypes: string[];
  /** SPEC-101 Wave 8 — PDF parser (reconfigure). */
  pdfParserBackend: PdfParserBackendDraft;
  /** SPEC-015V — Vision extract policy (document parsing). */
  visionExtractImages: boolean;
  visionExtractCharts: boolean;
  visionExtractFigures: boolean;
  visionPageSystemPrompt: string;
  visionImageSystemPrompt: string;
  visionChartSystemPrompt: string;
  visionFigureSystemPrompt: string;
  /** SPEC-101 Wave 8 — strict entity types (reconfigure). */
  entityTypesStrict: boolean;
  /** SPEC-102 — entity type → hex color overrides. */
  entityTypeColors: Record<string, string>;
  /** SPEC-114 — relation type allow-list (empty = free-form). */
  relationTypes: string[];
  /** SPEC-114 — strict relation types. */
  relationTypesStrict: boolean;
  /** SPEC-114 — optional domain preset id. */
  kgSchemaPreset?: string;
  /** SPEC-114b — typed edges (source — relation → target). */
  relationEdges: RelationEdge[];
  /** SPEC-109 — seed default reasoning effort. */
  reasoningEffort?: string;
  /** Model overrides (persisted with the draft — LAW-101-9). */
  llmPick?: WizardModelPick;
  embeddingPick?: WizardModelPick;
  visionPick?: WizardModelPick;
  advancedOpen: boolean;
}

/** Slash-style model pick stored on the wizard draft. */
export interface WizardModelPick {
  provider: string;
  model: string;
  fullId?: string;
  dimension?: number;
}

export const EMPTY_WIZARD_DRAFT: WizardDraft = {
  adminUsername: 'admin',
  adminEmail: '',
  adminPassword: '',
  adminPasswordConfirm: '',
  tenantName: '',
  tenantDescription: '',
  workspaceName: '',
  workspaceSlug: '',
  workspaceDescription: '',
  useServerDefaults: true,
  extractionLanguage: null,
  chunkingMode: null,
  chunkTokenSize: 1200,
  chunkOverlapTokenSize: 100,
  extractBudgetMode: null,
  extractMaxEntities: 40,
  extractMaxRecords: 100,
  entityTypes: [],
  pdfParserBackend: 'none',
  visionExtractImages: true,
  visionExtractCharts: true,
  visionExtractFigures: true,
  visionPageSystemPrompt: '',
  visionImageSystemPrompt: '',
  visionChartSystemPrompt: '',
  visionFigureSystemPrompt: '',
  entityTypesStrict: true,
  entityTypeColors: {},
  relationTypes: [],
  relationTypesStrict: true,
  kgSchemaPreset: undefined,
  relationEdges: [],
  reasoningEffort: undefined,
  llmPick: undefined,
  embeddingPick: undefined,
  visionPick: undefined,
  advancedOpen: false,
};

/**
 * Insert ingest-tuning steps before extraction:
 * chunking → extract-budget → extraction (language / KG schema).
 */
function withIngestTuningBeforeExtraction(
  steps: WizardStepId[],
  includeExtraction: boolean,
): WizardStepId[] {
  if (!includeExtraction) return steps;
  const out: WizardStepId[] = [];
  for (const step of steps) {
    if (step === 'extraction') {
      out.push('chunking', 'extract-budget');
    }
    out.push(step);
  }
  return out;
}

/** Document parsing sits immediately after models on every wizard kind. */
function withDocumentParsingAfterModels(steps: WizardStepId[]): WizardStepId[] {
  if (steps.includes('document-parsing')) return steps;
  const out: WizardStepId[] = [];
  for (const step of steps) {
    out.push(step);
    if (step === 'models') out.push('document-parsing');
  }
  return out;
}

export function stepsForWizard(
  kind: WizardKind,
  opts: { includeAdmin: boolean; includeExtraction: boolean } = {
    includeAdmin: false,
    includeExtraction: true,
  },
): WizardStepId[] {
  let steps: WizardStepId[];
  if (kind === 'reconfigure-workspace') {
    steps = withIngestTuningBeforeExtraction(
      ['models', 'document-parsing', 'extraction', 'review'],
      true,
    );
  } else if (kind === 'create-tenant') {
    steps = withIngestTuningBeforeExtraction(
      ['tenant-basics', 'models', 'workspace-basics', 'extraction', 'review'],
      true,
    );
  } else if (kind === 'create-workspace') {
    const create: WizardStepId[] = ['workspace-basics', 'models'];
    if (opts.includeExtraction) create.push('extraction');
    create.push('review');
    steps = withIngestTuningBeforeExtraction(create, opts.includeExtraction);
  } else {
    // first-run
    const first: WizardStepId[] = [];
    if (opts.includeAdmin) first.push('admin');
    first.push('tenant-basics', 'models', 'workspace-basics');
    if (opts.includeExtraction) first.push('extraction');
    first.push('review');
    steps = withIngestTuningBeforeExtraction(first, opts.includeExtraction);
  }
  return withDocumentParsingAfterModels(steps);
}

export function canProceed(
  step: WizardStepId,
  draft: WizardDraft,
  opts: {
    hasConfiguredDefaults: boolean;
    advancedModelsValid: boolean;
    /** When true (reconfigure), Apply requires at least one config diff. */
    hasConfigChanges?: boolean;
  } = {
    hasConfiguredDefaults: true,
    advancedModelsValid: true,
  },
): boolean {
  switch (step) {
    case 'admin': {
      const userOk = draft.adminUsername.trim().length >= 3;
      const passOk = draft.adminPassword.length >= 8;
      const match = draft.adminPassword === draft.adminPasswordConfirm;
      return userOk && passOk && match;
    }
    case 'tenant-basics':
      return draft.tenantName.trim().length > 0;
    case 'workspace-basics':
      return draft.workspaceName.trim().length > 0;
    case 'models':
      if (draft.useServerDefaults) {
        return opts.hasConfiguredDefaults;
      }
      return opts.advancedModelsValid;
    case 'document-parsing':
      return true;
    case 'chunking':
      return isChunkingDraftValid(draft);
    case 'extract-budget':
      return isExtractBudgetDraftValid(draft);
    case 'extraction':
      return true;
    case 'review':
      if (opts.hasConfigChanges === false) return false;
      return true;
    default:
      return false;
  }
}

export function clampStepIndex(index: number, stepCount: number): number {
  if (stepCount <= 0) return 0;
  return Math.max(0, Math.min(index, stepCount - 1));
}

export function progressPercent(index: number, stepCount: number): number {
  if (stepCount <= 0) return 0;
  return Math.round(((index + 1) / stepCount) * 100);
}

/** Persist non-secret draft fields (never password). */
export function draftForStorage(draft: WizardDraft): Omit<
  WizardDraft,
  'adminPassword' | 'adminPasswordConfirm'
> {
  const {
    adminPassword: _p,
    adminPasswordConfirm: _c,
    ...safe
  } = draft;
  return safe;
}
