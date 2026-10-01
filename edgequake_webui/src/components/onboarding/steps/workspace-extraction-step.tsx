'use client';

import { EntityTypeSelector } from '@/components/shared/entity-type-selector';
import {
  KgDomainApplyDefaultsButton,
  KgDomainPresetPicker,
  type DomainPresetId,
} from '@/components/shared/kg-domain-preset-picker';
import { RelationTypeSelector } from '@/components/shared/relation-type-selector';
import { TypedEdgeEditor } from '@/components/shared/typed-edge-editor';
import { CreateWorkspaceExtractionLanguageField } from '@/components/workspace/create-workspace-extraction-language-field';
import { detectPreset } from '@/constants/entity-presets';
import { getPresetTypes } from '@/constants/entity-type-catalog';
import {
  EDGE_PRESETS,
  RELATION_PRESETS,
  detectKgSchemaPreset,
  filterEdgesForVocabulary,
} from '@/constants/kg-schema-presets';
import type { WizardDraft } from '@/lib/onboarding/wizard-state';
import { applyExtractionLanguageToEntityTypes } from '@/lib/workspace/remap-entity-types-on-language-change';
import { useCallback, useMemo } from 'react';
import { useTranslation } from 'react-i18next';

export type { DomainPresetId };

export interface WorkspaceExtractionStepProps {
  draft: WizardDraft;
  onChange: (patch: Partial<WizardDraft>) => void;
  /** When true (reconfigure), show strict checkboxes. */
  showStrict?: boolean;
}

export function WorkspaceExtractionStep({
  draft,
  onChange,
  showStrict = false,
}: WorkspaceExtractionStepProps) {
  const { t } = useTranslation();
  const relationEdges = draft.relationEdges ?? [];

  const activePreset = useMemo(
    () =>
      detectKgSchemaPreset(
        draft.entityTypes,
        draft.relationTypes,
        relationEdges,
      ),
    [draft.entityTypes, draft.relationTypes, relationEdges],
  );

  /** Entity list matches a domain but relations/edges diverge — offer one-click restore. */
  const suggestedDomain = useMemo((): DomainPresetId | null => {
    const entityKey = detectPreset(draft.entityTypes);
    // Blank is already empty — do not nudge "Apply Blank defaults".
    if (entityKey === 'custom' || entityKey === 'blank') return null;
    if (
      activePreset === entityKey &&
      draft.relationTypes.length > 0 &&
      relationEdges.length > 0
    ) {
      return null;
    }
    return entityKey;
  }, [
    activePreset,
    draft.entityTypes,
    draft.relationTypes.length,
    relationEdges.length,
  ]);

  const handleLanguageChange = useCallback(
    (next: string | null) => {
      const { types, remapped } = applyExtractionLanguageToEntityTypes(
        draft.entityTypes,
        draft.extractionLanguage,
        next,
      );
      if (!remapped) {
        onChange({ extractionLanguage: next });
        return;
      }
      const edges = filterEdgesForVocabulary(
        relationEdges,
        types,
        draft.relationTypes,
      );
      onChange({
        extractionLanguage: next,
        entityTypes: types,
        relationEdges: edges,
        kgSchemaPreset: detectKgSchemaPreset(types, draft.relationTypes, edges),
      });
    },
    [
      draft.entityTypes,
      draft.extractionLanguage,
      draft.relationTypes,
      onChange,
      relationEdges,
    ],
  );

  const handleDomainSelect = useCallback(
    (key: DomainPresetId) => {
      onChange({
        entityTypes: getPresetTypes(key, draft.extractionLanguage),
        relationTypes: [...RELATION_PRESETS[key]],
        relationEdges: EDGE_PRESETS[key].map((e) => ({ ...e })),
        kgSchemaPreset: key,
      });
    },
    [draft.extractionLanguage, onChange],
  );

  return (
    <div className="space-y-3" data-testid="wizard-step-extraction">
      <div className="min-w-0" data-testid="wizard-extraction-language">
        <CreateWorkspaceExtractionLanguageField
          value={draft.extractionLanguage}
          onChange={handleLanguageChange}
          compact
        />
      </div>

      <section
        className="rounded-lg border bg-muted/15 px-3 py-2"
        data-testid="wizard-extraction-domain"
      >
        <KgDomainPresetPicker
          activePreset={activePreset}
          onSelect={handleDomainSelect}
          compact
          showRelationSamples={false}
        />
        {suggestedDomain ? (
          <div className="mt-2 flex flex-wrap items-center gap-2">
            <KgDomainApplyDefaultsButton
              presetKey={suggestedDomain}
              onApply={() => handleDomainSelect(suggestedDomain)}
            />
            <span className="text-xs text-muted-foreground">
              {t(
                'kgSchema.applyDefaultsHint',
                'Loads default entities, relations, and typed edges for this domain.',
              )}
            </span>
          </div>
        ) : null}
      </section>

      {/*
        Layout: entity | relation on one row from md; typed edges spans full width
        until xl where all three share equal columns. Avoids crushing the edge
        editor into a ~15rem gutter that clips add controls.
      */}
      <div
        className="grid grid-cols-1 gap-3 md:grid-cols-2 xl:grid-cols-3 xl:items-start"
        data-testid="wizard-extraction-kg-schema"
      >
        <div
          className="min-w-0 rounded-lg border p-3 space-y-2 flex flex-col"
          data-testid="wizard-extraction-entity-types"
        >
          <div className="space-y-0.5 shrink-0">
            <h4 className="text-sm font-medium">
              {t('onboarding.entityTypesHeading', 'Entity types')}
            </h4>
            <p className="text-xs text-muted-foreground">
              {t(
                'onboarding.entityTypesHintShort',
                'Kinds of nodes to extract into the graph.',
              )}
            </p>
          </div>
          <EntityTypeSelector
            value={draft.entityTypes}
            onChange={(types) => {
              const edges = filterEdgesForVocabulary(
                relationEdges,
                types,
                draft.relationTypes,
              );
              onChange({
                entityTypes: types,
                relationEdges: edges,
                kgSchemaPreset: detectKgSchemaPreset(
                  types,
                  draft.relationTypes,
                  edges,
                ),
              });
            }}
            extractionLanguage={draft.extractionLanguage}
            hidePresets
            density="compact"
            chipAreaClassName="min-h-14 max-h-32"
            strictLimit={draft.entityTypesStrict}
            onStrictLimitChange={
              showStrict
                ? (strict) => onChange({ entityTypesStrict: strict })
                : undefined
            }
            colors={draft.entityTypeColors}
            onColorsChange={(entityTypeColors) =>
              onChange({ entityTypeColors })
            }
          />
        </div>

        <div
          className="min-w-0 rounded-lg border p-3 space-y-2 flex flex-col"
          data-testid="wizard-extraction-relation-types"
        >
          <div className="space-y-0.5 shrink-0">
            <h4 className="text-sm font-medium">
              {t('onboarding.relationTypesHeading', 'Relation types')}
            </h4>
            <p className="text-xs text-muted-foreground">
              {t(
                'onboarding.relationTypesHintShort',
                'Allowed edge labels between entities. Loaded from the domain preset — edit freely.',
              )}
            </p>
          </div>
          <RelationTypeSelector
            value={draft.relationTypes}
            onChange={(types) => {
              const edges = filterEdgesForVocabulary(
                relationEdges,
                draft.entityTypes,
                types,
              );
              onChange({
                relationTypes: types,
                relationEdges: edges,
                kgSchemaPreset: detectKgSchemaPreset(
                  draft.entityTypes,
                  types,
                  edges,
                ),
              });
            }}
            density="compact"
            chipAreaClassName="min-h-14 max-h-32"
            strictLimit={draft.relationTypesStrict}
            onStrictLimitChange={
              showStrict
                ? (strict) => onChange({ relationTypesStrict: strict })
                : undefined
            }
          />
        </div>

        <div className="min-w-0 md:col-span-2 xl:col-span-1">
          <TypedEdgeEditor
            entityTypes={draft.entityTypes}
            relationTypes={draft.relationTypes}
            edges={relationEdges}
            onChange={(edges) =>
              onChange({
                relationEdges: edges,
                kgSchemaPreset: detectKgSchemaPreset(
                  draft.entityTypes,
                  draft.relationTypes,
                  edges,
                ),
              })
            }
            onRelationTypesChange={(types) =>
              onChange({
                relationTypes: types,
                kgSchemaPreset: detectKgSchemaPreset(
                  draft.entityTypes,
                  types,
                  relationEdges,
                ),
              })
            }
          />
        </div>
      </div>

      <p className="text-xs text-muted-foreground">
        {t(
          'onboarding.kgSchemaFutureOnly',
          'Applies to future extractions. Rebuild the knowledge graph to refresh existing nodes and edges.',
        )}
      </p>
    </div>
  );
}
