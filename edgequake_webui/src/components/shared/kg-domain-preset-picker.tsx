'use client';

/**
 * @module KgDomainPresetPicker
 * @description Always-visible domain cards that apply entity + relation defaults (SPEC-114).
 */

import { Button } from '@/components/ui/button';
import { ENTITY_PRESETS, type PresetKey } from '@/constants/entity-presets';
import { RELATION_PRESETS } from '@/constants/kg-schema-presets';
import { cn } from '@/lib/utils';
import {
  Factory,
  FlaskConical,
  Globe,
  HeartPulse,
  Scale,
  SquareDashed,
  TrendingUp,
} from 'lucide-react';
import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';

const PRESET_ICONS: Record<string, ReactNode> = {
  SquareDashed: <SquareDashed className="h-3.5 w-3.5" />,
  Globe: <Globe className="h-3.5 w-3.5" />,
  Factory: <Factory className="h-3.5 w-3.5" />,
  HeartPulse: <HeartPulse className="h-3.5 w-3.5" />,
  Scale: <Scale className="h-3.5 w-3.5" />,
  FlaskConical: <FlaskConical className="h-3.5 w-3.5" />,
  TrendingUp: <TrendingUp className="h-3.5 w-3.5" />,
};

export type DomainPresetId = Exclude<PresetKey, 'custom'>;

export interface KgDomainPresetPickerProps {
  activePreset: PresetKey;
  onSelect: (key: DomainPresetId) => void;
  /** Optional short sample of relation tokens shown under the card. */
  showRelationSamples?: boolean;
  /** Compact single-line chips for dense wizard layouts (default true). */
  compact?: boolean;
}

export function KgDomainPresetPicker({
  activePreset,
  onSelect,
  showRelationSamples = false,
  compact = true,
}: KgDomainPresetPickerProps) {
  const { t } = useTranslation();

  return (
    <div className="space-y-1.5" data-testid="kg-schema-domain-grid">
      <div className="flex items-baseline justify-between gap-2">
        <div className="min-w-0">
          <h4 className="text-sm font-medium">
            {t('kgSchema.domainHeading', 'Domain preset')}
          </h4>
          {!compact ? (
            <p className="text-xs text-muted-foreground mt-0.5">
              {t(
                'kgSchema.domainHint',
                'One click loads entity types and default relations for that domain. Blank clears the lists so you can build your own.',
              )}
            </p>
          ) : (
            <p className="text-xs text-muted-foreground mt-0.5 truncate">
              {t(
                'kgSchema.domainHintShort',
                'Loads entities, relations, and typed edges. Blank starts empty.',
              )}
            </p>
          )}
        </div>
        {activePreset === 'custom' ? (
          <span
            className="text-xs text-muted-foreground shrink-0"
            data-testid="kg-schema-custom-badge"
          >
            {t('kgSchema.customActive', 'Custom schema')}
          </span>
        ) : null}
      </div>

      <div
        className={cn(
          compact
            ? 'flex flex-wrap gap-1.5'
            : 'grid grid-cols-2 sm:grid-cols-4 lg:grid-cols-7 gap-2',
        )}
      >
        {(
          Object.entries(ENTITY_PRESETS) as [
            DomainPresetId,
            (typeof ENTITY_PRESETS)[DomainPresetId],
          ][]
        ).map(([key, preset]) => {
          const isActive = activePreset === key;
          const entityCount = ENTITY_PRESETS[key].types.length;
          const relationCount = RELATION_PRESETS[key].length;
          const samples = RELATION_PRESETS[key].slice(0, 2).join(', ');
          const isBlank = key === 'blank';

          return (
            <button
              key={key}
              type="button"
              onClick={() => onSelect(key)}
              aria-pressed={isActive}
              data-testid={`kg-schema-preset-${key}`}
              className={cn(
                'text-left transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring',
                compact
                  ? 'inline-flex items-center gap-1.5 rounded-md border px-2 py-1.5'
                  : 'group flex flex-col items-start gap-1 rounded-lg border px-2.5 py-2',
                isActive
                  ? 'border-foreground/40 bg-accent ring-1 ring-foreground/15'
                  : 'border-border bg-background hover:bg-accent/40',
              )}
            >
              <span
                className={cn(
                  'flex items-center gap-1.5 text-xs font-medium',
                  compact && 'whitespace-nowrap',
                )}
              >
                <span
                  className={cn(
                    'text-muted-foreground',
                    isActive && 'text-foreground',
                  )}
                >
                  {PRESET_ICONS[preset.icon]}
                </span>
                {t(preset.labelKey, preset.labelFallback)}
              </span>
              {!compact ? (
                <>
                  <span className="text-xs text-muted-foreground tabular-nums leading-tight">
                    {isBlank
                      ? t('kgSchema.blankCounts', 'Empty slate')
                      : `${entityCount} ${t('kgSchema.entitiesShort', 'entities')} · ${relationCount} ${t('kgSchema.relationsShort', 'relations')}`}
                  </span>
                  {showRelationSamples ? (
                    <span className="text-xs font-mono text-muted-foreground truncate w-full">
                      {isBlank
                        ? t('kgSchema.blankSample', 'Add your own types…')
                        : `${samples}…`}
                    </span>
                  ) : null}
                </>
              ) : (
                <span className="text-xs text-muted-foreground tabular-nums">
                  {isBlank
                    ? t('kgSchema.blankCountsShort', 'empty')
                    : `${entityCount}·${relationCount}`}
                </span>
              )}
            </button>
          );
        })}
      </div>

      {activePreset === 'custom' ? (
        <p className="text-xs text-muted-foreground">
          {t(
            'kgSchema.customHint',
            'Lists were edited manually. Pick a domain above to reset both entity and relation defaults.',
          )}
        </p>
      ) : null}
    </div>
  );
}

/** Compact “Apply domain defaults” control when a near-match is detected. */
export function KgDomainApplyDefaultsButton({
  presetKey,
  onApply,
}: {
  presetKey: DomainPresetId;
  onApply: () => void;
}) {
  const { t } = useTranslation();
  const preset = ENTITY_PRESETS[presetKey];
  return (
    <Button
      type="button"
      size="sm"
      variant="secondary"
      className="h-7 text-xs"
      onClick={onApply}
      data-testid="kg-schema-apply-domain-defaults"
    >
      {t('kgSchema.applyDomainDefaults', 'Apply {{domain}} defaults', {
        domain: t(preset.labelKey, preset.labelFallback),
      })}
    </Button>
  );
}
