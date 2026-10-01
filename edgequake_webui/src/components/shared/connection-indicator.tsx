/**
 * ConnectionIndicator — global online / backend status (LAW-155-7).
 */
'use client';

import { cn } from '@/lib/utils';
import { useTranslation } from 'react-i18next';

export type ConnectionState = 'online' | 'checking' | 'degraded' | 'offline';

const DOT: Record<ConnectionState, string> = {
  online: 'bg-success',
  checking: 'bg-muted-foreground',
  degraded: 'bg-warning',
  offline: 'bg-destructive',
};

export function ConnectionIndicator({
  state,
  label,
  className,
}: {
  state: ConnectionState;
  label?: string;
  className?: string;
}) {
  const { t } = useTranslation();
  const text =
    label ??
    (state === 'online'
      ? t('connection.online', 'Connected')
      : state === 'checking'
        ? t('connection.checking', 'Checking…')
        : state === 'degraded'
          ? t('connection.degraded', 'Degraded')
          : t('connection.offline', 'Offline'));

  return (
    <div
      className={cn('inline-flex items-center gap-2 text-sm text-muted-foreground', className)}
      role="status"
      aria-live="polite"
    >
      <span className={cn('h-2 w-2 rounded-full', DOT[state])} aria-hidden />
      <span className="hidden sm:inline">{text}</span>
      <span className="sr-only">{text}</span>
    </div>
  );
}
