'use client';

import { getConfidenceLabel } from '@/lib/citations/confidence';
import { useTranslation } from 'react-i18next';

export function ConfidenceDots({
  score,
  className = '',
}: {
  score: number;
  className?: string;
}) {
  const { t } = useTranslation();
  const filled = Math.round(score * 5);
  const { bgColor, labelKey, defaultLabel } = getConfidenceLabel(score);
  const pct = Math.round(score * 100);

  return (
    <span
      className={`inline-flex gap-0.5 items-center ${className}`}
      title={t('query.citations.confidencePercentTitle', '{{pct}}% confidence', { pct })}
      aria-label={t('query.citations.confidenceAria', 'Confidence: {{pct}}%', { pct })}
    >
      {[...Array(5)].map((_, i) => (
        <span
          key={i}
          className={`w-1.5 h-1.5 rounded-full transition-colors ${
            i < filled ? bgColor : 'bg-muted-foreground/20'
          }`}
        />
      ))}
      <span className="sr-only">
        {t(labelKey, defaultLabel)}
      </span>
    </span>
  );
}
