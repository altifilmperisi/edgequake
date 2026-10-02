import type { QueryContext } from '@/types';

export type ConfidenceStyle = {
  labelKey: string;
  defaultLabel: string;
  color: string;
  bgColor: string;
};

export const calculateConfidence = (context: QueryContext): number => {
  const chunkScores = context.chunks?.map((c) => c.score).filter((s) => s > 0) || [];

  if (chunkScores.length === 0) {
    const entityScores =
      context.entities?.map((e) => e.relevance).filter((r) => r > 0) || [];
    const relScores =
      context.relationships?.map((r) => r.relevance).filter((r) => r > 0) || [];
    const allScores = [...entityScores, ...relScores];
    if (allScores.length === 0) return 0.5;
    return allScores.reduce((a, b) => a + b, 0) / allScores.length;
  }

  const maxScore = Math.max(...chunkScores);
  const avgScore = chunkScores.reduce((a, b) => a + b, 0) / chunkScores.length;
  const entityBonus = Math.min(0.1, (context.entities?.length || 0) * 0.005);

  return Math.min(1.0, maxScore * 0.6 + avgScore * 0.3 + entityBonus);
};

export const getConfidenceLabel = (score: number): ConfidenceStyle => {
  if (score >= 0.5) {
    return {
      labelKey: 'query.citations.confidence.strong',
      defaultLabel: 'Strong',
      color: 'text-blue-600 dark:text-blue-400',
      bgColor: 'bg-blue-500',
    };
  }
  if (score >= 0.3) {
    return {
      labelKey: 'query.citations.confidence.good',
      defaultLabel: 'Good',
      color: 'text-sky-600 dark:text-sky-400',
      bgColor: 'bg-sky-500',
    };
  }
  if (score >= 0.2) {
    return {
      labelKey: 'query.citations.confidence.related',
      defaultLabel: 'Related',
      color: 'text-slate-600 dark:text-slate-400',
      bgColor: 'bg-slate-500',
    };
  }
  return {
    labelKey: 'query.citations.confidence.mentioned',
    defaultLabel: 'Mentioned',
    color: 'text-slate-500 dark:text-slate-400',
    bgColor: 'bg-slate-400',
  };
};
