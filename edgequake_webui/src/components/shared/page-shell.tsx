/**
 * PageShell — SPEC-155 LAW-155-2 one page chrome rhythm.
 */
'use client';

import { cn } from '@/lib/utils';
import type { ReactNode } from 'react';

export function PageShell({
  children,
  className,
  narrow = false,
}: {
  children: ReactNode;
  className?: string;
  narrow?: boolean;
}) {
  return (
    <div
      className={cn(
        'mx-auto w-full px-4 py-6 sm:px-6',
        narrow ? 'max-w-3xl' : 'max-w-7xl',
        className,
      )}
    >
      {children}
    </div>
  );
}
