/**
 * @module BackendStatusBanner
 * @description Dismissible banner that surfaces transport failures so the
 * user understands the dashboard is waiting for the backend rather than
 * broken. Pairs with the QueryProvider retry policy: while React Query
 * retries NetworkError silently in the background, this banner tells the
 * user *why* counts read as 0 and offers a manual retry.
 *
 * SPEC-021 P-G13: surfaces *unreachable* / *misconfigured* only.
 * *degraded* (busy during ingestion) is intentionally silent — laggy counts
 * during processing are expected and must not interrupt the UI.
 *
 * @implements FEAT1030 - System health monitoring (visible degradation)
 */
'use client';

import { Loader2, RefreshCw, WifiOff, X } from 'lucide-react';
import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { getBackendReadinessSnapshot } from '@/lib/api/client';
import { getBackendReadyRefetchIntervalForState } from '@/lib/runtime/health-poll';
import { useTranslation } from 'react-i18next';

/**
 * Banner shown when the backend is unreachable or degraded under load.
 *
 * - One `/live` + `/health` probe on mount; periodic poll only if
 *   `EDGEQUAKE_HEALTH_POLL_MS` is set (paused under Playwright automation).
 * - While readiness is `degraded`, poll until healthy so Header Busy clears (GH-400).
 * - Shares React Query key `['backend-ready']` with Header and SystemStatus (SSOT).
 * - Auto-dismisses once the backend reports ready.
 * - User can dismiss manually; the banner stays dismissed until the next
 *   navigation (sessionStorage) to avoid reappearing on every refetch.
 */
export function BackendStatusBanner() {
  const { t } = useTranslation();
  const [dismissed, setDismissed] = useState(false);

  const { data: readiness, isLoading } = useQuery({
    queryKey: ['backend-ready'],
    queryFn: () => getBackendReadinessSnapshot(),
    refetchInterval: (query) =>
      getBackendReadyRefetchIntervalForState(query.state.data?.state),
    staleTime: 5_000,
  });

  const state = readiness?.state;
  // WHY: Do not surface the busy/degraded "Processing documents…" banner —
  // ingestion lag is normal and the amber strip is noise on detail/list pages.
  // Keep the banner only for true outages / wrong-port misconfiguration.
  if (
    dismissed ||
    isLoading ||
    !state ||
    state === 'ready' ||
    state === 'degraded'
  ) {
    return null;
  }

  const isMisconfigured = state === 'misconfigured';

  return (
    /* ES-01: rendered as fixed overlay so it NEVER causes a layout shift (CLS).
     * WHY: Previously rendered inline between breadcrumb and main — every
     * appearance/disappearance shifted the entire content area down/up.
     * Fixed top anchors it to the visual top without affecting document flow. */
    <div
      role="status"
      aria-live="polite"
      className="fixed top-12 left-0 right-0 z-50 flex items-center gap-2 border-b border-amber-200 bg-amber-50/95 dark:border-amber-900/50 dark:bg-amber-950/90 backdrop-blur-sm px-4 py-2 text-sm text-amber-800 dark:text-amber-200 shadow-sm"
    >
      <WifiOff className="h-4 w-4 shrink-0" aria-hidden="true" />
      {/* ES-02: user-friendly language; developer details in tooltip/title only */}
      <span className="flex-1">
        {isMisconfigured
          ? t('common.backendWrongPort', 'EdgeQuake is starting up on a different port. Please refresh in a moment.')
          : t('common.backendNotReady', 'EdgeQuake is not available right now. Please check that the server is running.')}
      </span>
      <Loader2 className="h-3 w-3 animate-spin opacity-70" aria-hidden="true" />
      <button
        type="button"
        onClick={() => typeof window !== 'undefined' && window.location.reload()}
        className="inline-flex items-center gap-1 rounded px-2 py-0.5 text-xs font-medium hover:bg-amber-100 dark:hover:bg-amber-900/50 transition-colors"
        aria-label={t('common.retry', 'Retry connection')}
      >
        <RefreshCw className="h-3 w-3" aria-hidden="true" />
        {t('common.retry', 'Retry')}
      </button>
      <button
        type="button"
        onClick={() => setDismissed(true)}
        className="rounded p-0.5 hover:bg-amber-100 dark:hover:bg-amber-900/50 transition-colors"
        aria-label={t('common.dismiss', 'Dismiss')}
      >
        <X className="h-3.5 w-3.5" aria-hidden="true" />
      </button>
    </div>
  );
}

export default BackendStatusBanner;
