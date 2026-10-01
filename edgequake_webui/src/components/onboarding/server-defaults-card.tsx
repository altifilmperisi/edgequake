'use client';

import { Button } from '@/components/ui/button';
import { Skeleton } from '@/components/ui/skeleton';
import { useServerModelDefaults } from '@/hooks/use-server-model-defaults';
import type { DefaultsSource, ModelDefaultsSlice } from '@/lib/onboarding/inherited-defaults';
import { cn } from '@/lib/utils';
import { useTranslation } from 'react-i18next';

export interface ResolvedDefaultsDisplay extends ModelDefaultsSlice {
  isLoading?: boolean;
  hasConfiguredDefaults: boolean;
  /** When set, drives title (“Using tenant defaults” / “Using server defaults”). */
  source?: DefaultsSource;
  sourceLabel?: string;
  sourceHint?: string;
}

export interface ServerDefaultsCardProps {
  className?: string;
  onCustomize?: () => void;
  customizeLabel?: string;
  showCustomize?: boolean;
  /** When true, show an "Overridden" badge (Advanced custom models active). */
  overridden?: boolean;
  /**
   * When provided, render these resolved lines instead of calling useServerModelDefaults.
   * Used by Create Workspace (tenant → server ladder).
   */
  defaults?: ResolvedDefaultsDisplay;
}

function formatId(provider?: string, model?: string): string {
  if (provider && model) return `${provider}/${model}`;
  if (model) return model;
  return 'not configured';
}

function ServerDefaultsCardView({
  className,
  onCustomize,
  customizeLabel,
  showCustomize = true,
  overridden = false,
  isLoading,
  hasConfiguredDefaults,
  defaultLlmProvider,
  defaultLlmModel,
  defaultEmbeddingProvider,
  defaultEmbeddingModel,
  defaultVisionProvider,
  defaultVisionModel,
  source = 'server',
  sourceLabel,
  sourceHint,
}: ServerDefaultsCardProps &
  ResolvedDefaultsDisplay & {
    isLoading: boolean;
  }) {
  const { t } = useTranslation();

  const title =
    sourceLabel ??
    (source === 'tenant'
      ? t('onboarding.usingTenantDefaults', 'Using tenant defaults')
      : hasConfiguredDefaults
        ? t('onboarding.usingServerDefaults', 'Using server defaults')
        : t('onboarding.serverDefaultsMissing', 'Server defaults not fully configured'));

  const hint =
    sourceHint ??
    (source === 'tenant'
      ? t(
          'onboarding.tenantDefaultsSource',
          'Inherits from the current tenant (falls back to server). Can be overridden for this workspace.',
        )
      : t(
          'onboarding.serverDefaultsSource',
          'Source: environment / server config. Can be overridden per tenant or workspace.',
        ));

  if (isLoading) {
    return (
      <div
        className={cn('rounded-lg border p-3 space-y-2', className)}
        data-testid="server-defaults-card-loading"
      >
        <Skeleton className="h-4 w-40" />
        <Skeleton className="h-3 w-full" />
        <Skeleton className="h-3 w-full" />
        <Skeleton className="h-3 w-3/4" />
      </div>
    );
  }

  return (
    <div
      className={cn('rounded-lg border p-3 space-y-2', className)}
      data-testid="server-defaults-card"
      data-configured={hasConfiguredDefaults ? 'true' : 'false'}
      data-overridden={overridden ? 'true' : 'false'}
      data-source={source}
    >
      <div className="flex items-center justify-between gap-2">
        <p className="text-sm font-medium">{title}</p>
        {overridden ? (
          <span
            className="rounded-md bg-muted px-1.5 py-0.5 text-xs font-medium uppercase tracking-wide text-muted-foreground"
            data-testid="server-defaults-overridden"
          >
            {t('onboarding.overriddenDefaults', 'Overridden')}
          </span>
        ) : null}
      </div>
      <dl className="space-y-1 text-xs font-mono text-muted-foreground">
        <div className="flex gap-2" data-testid="server-defaults-llm">
          <dt className="w-20 shrink-0 font-sans font-medium text-foreground/80">LLM</dt>
          <dd>{formatId(defaultLlmProvider, defaultLlmModel)}</dd>
        </div>
        <div className="flex gap-2" data-testid="server-defaults-embedding">
          <dt className="w-20 shrink-0 font-sans font-medium text-foreground/80">Embedding</dt>
          <dd>{formatId(defaultEmbeddingProvider, defaultEmbeddingModel)}</dd>
        </div>
        <div className="flex gap-2" data-testid="server-defaults-vision">
          <dt className="w-20 shrink-0 font-sans font-medium text-foreground/80">Vision</dt>
          <dd>{formatId(defaultVisionProvider, defaultVisionModel)}</dd>
        </div>
      </dl>
      <p className="text-xs text-muted-foreground">{hint}</p>
      {showCustomize && onCustomize ? (
        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={onCustomize}
          data-testid="server-defaults-customize"
        >
          {customizeLabel ?? t('onboarding.customizeModels', 'Customize models')}
        </Button>
      ) : null}
    </div>
  );
}

function ServerDefaultsCardFromServer(props: Omit<ServerDefaultsCardProps, 'defaults'>) {
  const server = useServerModelDefaults();
  return (
    <ServerDefaultsCardView
      {...props}
      isLoading={server.isLoading}
      hasConfiguredDefaults={server.hasConfiguredDefaults}
      defaultLlmProvider={server.defaultLlmProvider}
      defaultLlmModel={server.defaultLlmModel}
      defaultEmbeddingProvider={server.defaultEmbeddingProvider}
      defaultEmbeddingModel={server.defaultEmbeddingModel}
      defaultVisionProvider={server.defaultVisionProvider}
      defaultVisionModel={server.defaultVisionModel}
      source="server"
    />
  );
}

/**
 * SPEC-101 — Explicit defaults for LLM · Embedding · Vision (LAW-101-2).
 * Presentational when `defaults` prop is set; otherwise loads server defaults.
 */
export function ServerDefaultsCard({ defaults, ...rest }: ServerDefaultsCardProps) {
  if (defaults) {
    return (
      <ServerDefaultsCardView
        {...rest}
        isLoading={Boolean(defaults.isLoading)}
        hasConfiguredDefaults={defaults.hasConfiguredDefaults}
        defaultLlmProvider={defaults.defaultLlmProvider}
        defaultLlmModel={defaults.defaultLlmModel}
        defaultEmbeddingProvider={defaults.defaultEmbeddingProvider}
        defaultEmbeddingModel={defaults.defaultEmbeddingModel}
        defaultVisionProvider={defaults.defaultVisionProvider}
        defaultVisionModel={defaults.defaultVisionModel}
        source={defaults.source ?? 'server'}
        sourceLabel={defaults.sourceLabel}
        sourceHint={defaults.sourceHint}
      />
    );
  }
  return <ServerDefaultsCardFromServer {...rest} />;
}
