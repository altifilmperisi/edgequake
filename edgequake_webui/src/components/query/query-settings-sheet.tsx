/**
 * @module QuerySettingsSheet
 * @description Settings panel for query configuration (extracted from query-interface.tsx for better compilation performance).
 *
 * Provides UI controls for:
 * - Streaming toggle
 * - Top K results
 * - Temperature
 * - Max tokens
 *
 * @implements FEAT0007 - Natural Language Query Processing
 * @implements BR0105 - Streaming must show progressive thinking indicators
 */
'use client';

import { Button } from '@/components/ui/button';
import {
  DrawerBody,
  DrawerField,
  DrawerSection,
  DrawerSliderField,
  DrawerToggleRow,
} from '@/components/ui/drawer-layout';
import { Label } from '@/components/ui/label';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from '@/components/ui/sheet';
import { Textarea } from '@/components/ui/textarea';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import type { DocumentFilter } from '@/types/query';
import {
  BookOpen,
  Brain,
  FileText,
  Filter,
  Gauge,
  Info,
  Settings2,
  Sliders,
  Thermometer,
  Zap,
} from 'lucide-react';
import type { ReactNode } from 'react';
import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { DocumentPickerPopover } from './document-picker-popover';
import { ProviderModelSelector } from './provider-model-selector';
import { QueryDocumentFilter } from './query-document-filter';
import { ReasoningEffortSelect } from '@/components/settings/reasoning-effort-select';
import { useLlmModels } from '@/hooks/use-providers';
import {
  effectiveEffortWhenAuto,
  modelSupportsThinking,
  supportedReasoningEffortsForModel,
} from '@/lib/settings/reasoning-effort-supported';
import { normalizeModelFullId } from '@/lib/onboarding/model-payload';

interface QuerySettings {
  stream: boolean;
  topK: number;
  temperature: number;
  maxTokens: number;
  systemPrompt?: string;
  fullChunkContent?: boolean;
  /** SPEC-109: Auto when undefined */
  reasoningEffort?: string;
}

interface QuerySettingsSheetProps {
  /** Current query settings */
  settings: QuerySettings;
  /** Callback to update query settings */
  onSettingsChange: (updates: Partial<QuerySettings>) => void;
  /** Whether the settings panel is disabled */
  disabled?: boolean;
  /** Optional trigger button */
  trigger?: ReactNode;
  /** Provider+model selector — moved from main toolbar for density reduction */
  providerModel?: string;
  onProviderModelChange?: (value: string) => void;
  /** Document filter — moved from main toolbar for density reduction */
  documentFilter?: DocumentFilter | undefined;
  onDocumentFilterChange?: (value: DocumentFilter | undefined) => void;
  /** SPEC-031: Explicit document scope selection */
  scopedDocumentIds?: string[];
  onScopedDocumentIdsChange?: (ids: string[]) => void;
}

export function QuerySettingsSheet({
  settings,
  onSettingsChange,
  disabled = false,
  trigger,
  providerModel,
  onProviderModelChange,
  documentFilter,
  onDocumentFilterChange,
  scopedDocumentIds,
  onScopedDocumentIdsChange,
}: QuerySettingsSheetProps) {
  const { t } = useTranslation();
  const { data: llmData } = useLlmModels();
  const reasoningSupported = useMemo(() => {
    if (!providerModel?.trim()) return undefined;
    const { provider, model } = normalizeModelFullId(providerModel);
    return supportedReasoningEffortsForModel(llmData?.models, provider, model);
  }, [llmData?.models, providerModel]);
  const thinkingSupported = useMemo(() => {
    if (!providerModel?.trim()) return undefined;
    const { provider, model } = normalizeModelFullId(providerModel);
    return modelSupportsThinking(llmData?.models, provider, model);
  }, [llmData?.models, providerModel]);
  const reasoningEffectiveAuto = useMemo(() => {
    if (!providerModel?.trim()) {
      return effectiveEffortWhenAuto(undefined, undefined, undefined, "query");
    }
    const { provider, model } = normalizeModelFullId(providerModel);
    return effectiveEffortWhenAuto(llmData?.models, provider, model, "query");
  }, [llmData?.models, providerModel]);

  return (
    <Sheet>
      <SheetTrigger asChild>
        {trigger || (
          <Button
            variant="ghost"
            size="icon"
            disabled={disabled}
            aria-label={t("query.settings.title", "Query settings")}
            data-testid="query-settings-trigger"
          >
            <Settings2 className="h-4 w-4" />
          </Button>
        )}
      </SheetTrigger>
      <SheetContent
        data-testid="query-settings-sheet"
        size="lg"
        className="w-full sm:w-[480px] flex flex-col p-0 overflow-hidden"
      >
        <SheetHeader className="border-b shrink-0 bg-background">
          <SheetTitle className="flex items-center gap-2 text-base">
            <Sliders className="h-4 w-4 text-primary" />
            {t('query.settings.title', 'Query Settings')}
          </SheetTitle>
          <SheetDescription className="text-sm leading-snug">
            {t('query.settings.description', 'Configure how the AI processes and responds to your queries.')}
          </SheetDescription>
        </SheetHeader>

        <ScrollArea className="flex-1 min-h-0" showShadows>
          <DrawerBody data-testid="query-settings-scroll-body">
            {/* Context Section — Provider & Document Filter (moved from main toolbar) */}
            {(onProviderModelChange || onDocumentFilterChange) && (
              <DrawerSection
                icon={<Filter className="h-3.5 w-3.5 text-blue-500" />}
                title={t('query.settings.context', 'Context')}
              >
                {onProviderModelChange && (
                  <DrawerField
                    label={t('query.settings.provider', 'AI Provider & Model')}
                  >
                    <ProviderModelSelector
                      value={providerModel ?? ''}
                      onChange={onProviderModelChange}
                      disabled={disabled}
                    />
                  </DrawerField>
                )}
                {onDocumentFilterChange && (
                  <DrawerField
                    label={t('query.settings.documentFilter', 'Document Filter')}
                    hint={t(
                      'query.filter.description',
                      'Restrict RAG context to documents matching these criteria.',
                    )}
                  >
                    <QueryDocumentFilter
                      value={documentFilter}
                      onChange={onDocumentFilterChange}
                      disabled={disabled}
                      variant="block"
                    />
                  </DrawerField>
                )}
                {/* SPEC-031: Explicit document scope picker */}
                {onScopedDocumentIdsChange && (
                  <DrawerField
                    label={t('query.scope.sectionTitle', 'Document Scope')}
                    hint={t(
                      'query.scope.description',
                      'Restrict queries to specific documents. Default is all workspace docs.',
                    )}
                    trailing={
                      scopedDocumentIds && scopedDocumentIds.length > 0 ? (
                        <span className="text-xs text-muted-foreground tabular-nums">
                          {t('query.scope.selectedCount', '{{count}} selected', {
                            count: scopedDocumentIds.length,
                          })}
                        </span>
                      ) : null
                    }
                  >
                    <DocumentPickerPopover
                      selectedIds={scopedDocumentIds ?? []}
                      onSelectionChange={onScopedDocumentIdsChange}
                      disabled={disabled}
                      trigger={
                        <button
                          type="button"
                          disabled={disabled}
                          className="w-full flex items-center gap-2 rounded-md border bg-background px-3 py-2 text-xs text-muted-foreground hover:bg-accent hover:text-accent-foreground transition-colors disabled:opacity-50"
                        >
                          <FileText className="h-3.5 w-3.5 shrink-0" />
                          {scopedDocumentIds && scopedDocumentIds.length > 0
                            ? t('query.scope.editSelection', 'Edit scope ({{count}} docs)', {
                                count: scopedDocumentIds.length,
                              })
                            : t('query.scope.addDocuments', 'Add documents to scope')
                          }
                        </button>
                      }
                    />
                  </DrawerField>
                )}
              </DrawerSection>
            )}

            <DrawerSection
              icon={<Zap className="h-3.5 w-3.5 text-amber-500" />}
              title={t('query.settings.responseMode', 'Response Mode')}
            >
              <DrawerToggleRow
                id="stream-toggle"
                data-testid="query-settings-stream-toggle"
                label={t('query.settings.streaming', 'Streaming')}
                description={t('query.settings.streamingDescription', 'Show response as it generates')}
                checked={settings.stream}
                onCheckedChange={(stream) => onSettingsChange({ stream })}
              />
              <DrawerToggleRow
                id="full-chunk-toggle"
                data-testid="query-settings-full-chunk-toggle"
                label={t('query.settings.fullPassageText', 'Full passage text')}
                description={t(
                  'query.settings.fullPassageTextDescription',
                  'Show complete retrieved chunks in citations (uses more bandwidth)',
                )}
                checked={settings.fullChunkContent ?? false}
                onCheckedChange={(fullChunkContent) => onSettingsChange({ fullChunkContent })}
                disabled={disabled}
              />
            </DrawerSection>

            <DrawerSection
              icon={<BookOpen className="h-3.5 w-3.5 text-blue-500" />}
              title={t('query.settings.retrieval', 'Retrieval')}
            >
              <DrawerSliderField
                label={t('query.settings.topK', 'Top K Results')}
                value={settings.topK}
                onValueChange={(topK) => onSettingsChange({ topK })}
                min={1}
                max={50}
                step={1}
                hint={t(
                  'query.settings.topKHint',
                  'Number of relevant chunks to retrieve from the knowledge graph',
                )}
                startLabel="1 · Precise"
                endLabel="50 · Comprehensive"
              />
            </DrawerSection>

            <DrawerSection
              icon={<Brain className="h-3.5 w-3.5 text-purple-500" />}
              title={t('query.settings.generation', 'Generation')}
            >
              <ReasoningEffortSelect
                value={settings.reasoningEffort}
                onChange={(reasoningEffort) =>
                  onSettingsChange({ reasoningEffort })
                }
                supported={reasoningSupported}
                thinkingSupported={thinkingSupported}
                effectiveWhenAuto={reasoningEffectiveAuto}
                disabled={disabled}
                label={t('query.settings.reasoningEffort', 'Reasoning effort')}
              />
              <DrawerSliderField
                icon={<Thermometer className="h-3.5 w-3.5 text-muted-foreground" />}
                label={t('query.settings.temperature', 'Temperature')}
                value={settings.temperature}
                displayValue={settings.temperature.toFixed(1)}
                onValueChange={(temperature) => onSettingsChange({ temperature })}
                min={0}
                max={2}
                step={0.1}
                hint={t(
                  'query.settings.temperatureHint',
                  'Controls randomness. Lower = more focused, higher = more creative',
                )}
                startLabel="0 · Precise"
                endLabel="2 · Creative"
              />
              <DrawerSliderField
                icon={<Gauge className="h-3.5 w-3.5 text-muted-foreground" />}
                label={t('query.settings.maxTokens', 'Max Tokens')}
                value={settings.maxTokens}
                onValueChange={(maxTokens) => onSettingsChange({ maxTokens })}
                min={256}
                max={4096}
                step={256}
                hint={t(
                  'query.settings.maxTokensHint',
                  'Maximum length of the generated response',
                )}
                startLabel="256"
                endLabel="4096"
              />
            </DrawerSection>

            <DrawerSection
              icon={<FileText className="h-3.5 w-3.5 text-emerald-500" />}
              title={t('query.settings.systemPrompt', 'System Prompt')}
            >
              <div className="space-y-2">
                <div className="flex items-center gap-1.5">
                  <Label htmlFor="system-prompt" className="text-sm font-medium">
                    {t('query.settings.systemPromptLabel', 'Custom Instructions')}
                  </Label>
                  <TooltipProvider>
                    <Tooltip>
                      <TooltipTrigger aria-label="System prompt help">
                        <Info className="h-3.5 w-3.5 text-muted-foreground" />
                      </TooltipTrigger>
                      <TooltipContent side="top" className="max-w-[240px]">
                        <p className="text-xs leading-snug">
                          {t(
                            'query.settings.systemPromptHint',
                            'Additional instructions injected into the RAG prompt. Use this to steer tone, format, or domain focus without replacing the core prompt.',
                          )}
                        </p>
                      </TooltipContent>
                    </Tooltip>
                  </TooltipProvider>
                </div>
                <Textarea
                  id="system-prompt"
                  data-testid="query-settings-system-prompt"
                  placeholder={t(
                    'query.settings.systemPromptPlaceholder',
                    'e.g. "Always respond in bullet points" or "Focus on security implications"',
                  )}
                  value={settings.systemPrompt ?? ''}
                  onChange={(e) =>
                    onSettingsChange({ systemPrompt: e.target.value || undefined })
                  }
                  className="min-h-[96px] resize-y text-sm"
                  rows={4}
                />
                {settings.systemPrompt ? (
                  <p className="text-xs text-muted-foreground leading-snug">
                    {t(
                      'query.settings.systemPromptActive',
                      'System prompt active — will be injected into every query.',
                    )}
                  </p>
                ) : null}
              </div>
            </DrawerSection>
          </DrawerBody>
        </ScrollArea>
      </SheetContent>
    </Sheet>
  );
}
