/**
 * StatusBadge — SPEC-155 semantic status SSOT (LAW-155-1).
 */
'use client';

import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';

export type StatusTone = 'neutral' | 'success' | 'warning' | 'info' | 'danger';

const TONE_CLASS: Record<StatusTone, string> = {
  neutral: 'bg-muted text-muted-foreground border-border',
  success: 'bg-success/15 text-success border-success/30',
  warning: 'bg-warning/20 text-warning-foreground border-warning/40',
  info: 'bg-info/15 text-info border-info/30',
  danger: 'bg-destructive/15 text-destructive border-destructive/30',
};

/** Map common domain statuses → tone */
export function statusToneFromLabel(status: string): StatusTone {
  const s = status.toLowerCase();
  if (['completed', 'ready', 'healthy', 'success', 'ok', 'active'].includes(s)) return 'success';
  if (['processing', 'pending', 'queued', 'running', 'warning', 'degraded'].includes(s))
    return 'warning';
  if (['failed', 'error', 'deleted', 'critical', 'offline'].includes(s)) return 'danger';
  if (['info', 'draft', 'idle'].includes(s)) return 'info';
  return 'neutral';
}

export function StatusBadge({
  label,
  tone,
  className,
}: {
  label: string;
  tone?: StatusTone;
  className?: string;
}) {
  const resolved = tone ?? statusToneFromLabel(label);
  return (
    <Badge
      variant="outline"
      className={cn('text-xs font-medium capitalize', TONE_CLASS[resolved], className)}
    >
      {label}
    </Badge>
  );
}
