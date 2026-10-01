/**
 * PageHeader — SPEC-155 LAW-155-2.
 */
'use client';

import { cn } from '@/lib/utils';
import type { ReactNode } from 'react';

export function PageHeader({
  title,
  description,
  actions,
  tabs,
  className,
}: {
  title: ReactNode;
  description?: ReactNode;
  actions?: ReactNode;
  tabs?: ReactNode;
  className?: string;
}) {
  return (
    <header className={cn('mb-6 space-y-3', className)}>
      <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
        <div className="min-w-0 space-y-1">
          <h1
            className="break-words text-xl font-semibold tracking-tight text-foreground [overflow-wrap:anywhere]"
            title={typeof title === 'string' ? title : undefined}
          >
            {title}
          </h1>
          {description ? (
            <p className="text-sm text-muted-foreground">{description}</p>
          ) : null}
        </div>
        {actions ? (
          <div className="flex shrink-0 flex-wrap items-center gap-2">{actions}</div>
        ) : null}
      </div>
      {tabs}
    </header>
  );
}
