/**
 * @fileoverview Dashboard subtitle: what needs a glance right now.
 *
 * WHY: the old subtitle ("5 documents · 100 entities · 144 relationships")
 * repeated the four stat cards directly below it. This line says something the
 * cards cannot: is anything running, queued or broken?
 */
'use client';

import { hasStatusCounts, isAllSettled, summarizeAttention } from '@/lib/dashboard/activity-model';
import type { DocumentStatusCounts } from '@/types';
import Link from 'next/link';
import { useTranslation } from 'react-i18next';

interface DashboardSubtitleProps {
  documentCount: number;
  counts?: Partial<DocumentStatusCounts> | null;
}

export function DashboardSubtitle({ documentCount, counts }: DashboardSubtitleProps) {
  const { t } = useTranslation();

  if (documentCount === 0) {
    return <>{t('dashboard.emptySubtitle', 'Upload your first document to get started')}</>;
  }

  // Counts not loaded yet: say nothing we cannot know (never claim "all processed").
  if (!hasStatusCounts(counts)) {
    return (
      <>
        {t('dashboard.documentsOnly', {
          count: documentCount,
          defaultValue_one: '{{count}} document',
          defaultValue_other: '{{count}} documents',
        })}
      </>
    );
  }

  const attention = summarizeAttention(counts);
  if (isAllSettled(attention)) {
    return <>{t('dashboard.allSettled', 'All documents are processed')}</>;
  }

  const parts: Array<{ key: string; text: string; tone?: string }> = [];
  if (attention.processing > 0) {
    parts.push({
      key: 'processing',
      text: t('dashboard.attention.processing', '{{count}} processing', { count: attention.processing }),
    });
  }
  if (attention.pending > 0) {
    parts.push({
      key: 'pending',
      text: t('dashboard.attention.queued', '{{count}} queued', { count: attention.pending }),
    });
  }
  if (attention.failed > 0) {
    parts.push({
      key: 'failed',
      text: t('dashboard.attention.failed', '{{count}} failed', { count: attention.failed }),
      tone: 'text-rose-600 dark:text-rose-400 font-medium',
    });
  }

  return (
    <Link
      href="/documents"
      data-testid="dashboard-attention"
      className="rounded hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
    >
      {parts.map((p, i) => (
        <span key={p.key}>
          {i > 0 ? ' · ' : ''}
          <span className={p.tone}>{p.text}</span>
        </span>
      ))}
    </Link>
  );
}
