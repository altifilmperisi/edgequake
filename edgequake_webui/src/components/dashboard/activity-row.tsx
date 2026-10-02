/**
 * @fileoverview One Recent-activity row: title, relative time, shared status
 * badge, and a single honest detail line (live progress or why it needs you).
 */
'use client';

import { StatusBadge } from '@/components/documents/status-badge';
import { cn } from '@/lib/utils';
import type { ActivityDetail, ActivityItem, ActivityTone } from '@/lib/dashboard/activity-model';
import { formatDistanceToNow } from 'date-fns';
import { FileText } from 'lucide-react';
import Link from 'next/link';

const TONE_CLASS: Record<ActivityTone, string> = {
  error: 'text-rose-600 dark:text-rose-400',
  warn: 'text-amber-700 dark:text-amber-400',
  muted: 'text-muted-foreground',
};

function relativeTime(iso: string | null): string {
  if (!iso) return '';
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? '' : formatDistanceToNow(d, { addSuffix: true });
}

/** Thin bar: determinate when the server sent a fraction, else a quiet pulse. */
function LiveProgress({ pct, label }: { pct: number | null; label: string }) {
  return (
    <div className="mt-1.5 flex items-center gap-2" data-testid="dashboard-activity-progress">
      <div
        className="h-1 min-w-0 flex-1 overflow-hidden rounded-full bg-muted"
        role={pct === null ? undefined : 'progressbar'}
        aria-valuenow={pct ?? undefined}
        aria-valuemin={pct === null ? undefined : 0}
        aria-valuemax={pct === null ? undefined : 100}
        aria-label={label}
      >
        <div
          className={cn(
            'h-full rounded-full bg-sky-500 transition-[width] duration-500',
            pct === null && 'w-1/3 motion-safe:animate-pulse',
          )}
          style={pct === null ? undefined : { width: `${Math.max(pct, 4)}%` }}
        />
      </div>
      <span className="shrink-0 truncate text-[11px] text-muted-foreground tabular-nums">
        {label}
        {pct === null ? '' : ` · ${pct}%`}
      </span>
    </div>
  );
}

function DetailLine({ detail }: { detail: ActivityDetail }) {
  if (detail.kind === 'progress') {
    return <LiveProgress pct={detail.pct} label={detail.label} />;
  }
  return (
    <p
      className={cn('mt-0.5 truncate text-xs', TONE_CLASS[detail.tone])}
      data-testid="dashboard-activity-note"
      title={detail.text}
    >
      {detail.text}
    </p>
  );
}

export function ActivityRow({ item }: { item: ActivityItem }) {
  const when = relativeTime(item.createdAt);
  return (
    <Link
      href={`/documents?id=${item.id}`}
      data-testid="dashboard-activity-row"
      data-status={item.status}
      className={cn(
        'flex items-start gap-3 rounded-lg px-3 py-2.5 transition-colors',
        'hover:bg-muted/50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring',
      )}
    >
      <div className="mt-0.5 flex h-8 w-8 shrink-0 items-center justify-center rounded-md bg-muted">
        <FileText className="h-4 w-4 text-muted-foreground" aria-hidden="true" />
      </div>
      <div className="min-w-0 flex-1">
        <div className="flex items-center justify-between gap-3">
          <p className="truncate text-sm font-medium" title={item.title}>
            {item.title}
          </p>
          <div className="shrink-0">
            <StatusBadge status={item.status} />
          </div>
        </div>
        {when ? <p className="text-xs text-muted-foreground">{when}</p> : null}
        {item.detail ? <DetailLine detail={item.detail} /> : null}
      </div>
    </Link>
  );
}
