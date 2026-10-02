/**
 * Status pill for the document detail header.
 *
 * One component for every lifecycle state (replaces four near-identical Badge
 * blocks). It is deliberately NOT button-shaped: tinted, non-interactive, so it
 * cannot be mistaken for the Cancel / Reprocess actions beside it.
 */
'use client';

import { cn } from '@/lib/utils';
import type { DetailLifecycle } from '@/lib/documents/detail-lifecycle';
import { AlertCircle, Loader2, StopCircle } from 'lucide-react';

type PillTone = 'working' | 'partial' | 'failed' | 'cancelled';

const TONE_CLASSES: Record<PillTone, string> = {
  working:
    'bg-sky-50 text-sky-800 ring-sky-200 dark:bg-sky-950/40 dark:text-sky-200 dark:ring-sky-900',
  partial:
    'bg-amber-50 text-amber-800 ring-amber-200 dark:bg-amber-950/40 dark:text-amber-200 dark:ring-amber-900',
  failed:
    'bg-rose-50 text-rose-800 ring-rose-200 dark:bg-rose-950/40 dark:text-rose-200 dark:ring-rose-900',
  cancelled:
    'bg-muted text-muted-foreground ring-border',
};

/** Pure: which tone (if any) a lifecycle should render with. */
export function lifecyclePillTone(
  lifecycle: Pick<DetailLifecycle, 'kind' | 'showSpinner'>,
): PillTone | null {
  if (lifecycle.showSpinner) return 'working';
  switch (lifecycle.kind) {
    case 'partial':
      return 'partial';
    case 'failed':
      return 'failed';
    case 'cancelled':
      return 'cancelled';
    default:
      return null;
  }
}

export function DetailLifecycleBadge({
  lifecycle,
}: {
  lifecycle: Pick<DetailLifecycle, 'kind' | 'showSpinner' | 'label'>;
}) {
  const tone = lifecyclePillTone(lifecycle);
  if (!tone) return null;
  const Icon =
    tone === 'working' ? Loader2 : tone === 'cancelled' ? StopCircle : AlertCircle;
  return (
    <span
      role="status"
      data-testid="detail-lifecycle-badge"
      data-tone={tone}
      className={cn(
        'inline-flex h-6 max-w-[16rem] items-center gap-1.5 rounded-full px-2.5 text-xs font-medium ring-1 ring-inset',
        TONE_CLASSES[tone],
      )}
    >
      <Icon
        className={cn('h-3 w-3 shrink-0', tone === 'working' && 'animate-spin')}
        aria-hidden="true"
      />
      <span className="truncate">{lifecycle.label}</span>
    </span>
  );
}
