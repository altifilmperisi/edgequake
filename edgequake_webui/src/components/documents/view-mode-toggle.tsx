/**
 * Segmented PDF / split / Markdown view switcher for the document viewer.
 *
 * Presentational only: the parent owns the mode.
 */
'use client';

import { Button } from '@/components/ui/button';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import { cn } from '@/lib/utils';
import { Columns2, PanelLeftClose, PanelRightClose } from 'lucide-react';
import type { ComponentType } from 'react';

export type ViewMode = 'side-by-side' | 'pdf-only' | 'markdown-only';

const OPTIONS: ReadonlyArray<{
  mode: ViewMode;
  label: string;
  Icon: ComponentType<{ className?: string }>;
}> = [
  { mode: 'pdf-only', label: 'PDF Only', Icon: PanelRightClose },
  { mode: 'side-by-side', label: 'Split View', Icon: Columns2 },
  { mode: 'markdown-only', label: 'Markdown Only', Icon: PanelLeftClose },
];

export function ViewModeToggle({
  mode,
  onModeChange,
  className,
}: {
  mode: ViewMode;
  onModeChange: (mode: ViewMode) => void;
  className?: string;
}) {
  return (
    <TooltipProvider>
      <div
        role="group"
        aria-label="Viewer layout"
        data-testid="viewer-mode-toggle"
        className={cn('flex items-center gap-0.5 rounded-md bg-muted/70 p-0.5', className)}
      >
        {OPTIONS.map(({ mode: option, label, Icon }) => (
          <Tooltip key={option}>
            <TooltipTrigger asChild>
              <Button
                variant={mode === option ? 'secondary' : 'ghost'}
                size="icon"
                className={cn('h-6 w-6', mode === option && 'bg-background shadow-sm')}
                aria-label={label}
                aria-pressed={mode === option}
                onClick={() => onModeChange(option)}
              >
                <Icon className="h-3.5 w-3.5" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>{label}</TooltipContent>
          </Tooltip>
        ))}
      </div>
    </TooltipProvider>
  );
}
