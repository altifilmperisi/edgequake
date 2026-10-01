/**
 * SPEC-143 — Explicit three-way page sync direction control.
 *
 * Presentational only: parents own mode state via usePageSyncController.
 */

'use client';

import { Button } from '@/components/ui/button';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import {
  PAGE_SYNC_MODES,
  type PageSyncMode,
} from '@/lib/documents/page-sync-mode';
import { cn } from '@/lib/utils';
import { useTranslation } from 'react-i18next';

export interface PageSyncModeControlProps {
  mode: PageSyncMode;
  onModeChange: (mode: PageSyncMode) => void;
  /** When false, all segments disabled (no page markers). */
  available?: boolean;
  className?: string;
  /** Compact height for side-by-side toolbar (24px). Default true. */
  compact?: boolean;
  /** Override root test id (desktop vs mobile chrome). */
  testId?: string;
}

const MODE_LABEL_KEY: Record<PageSyncMode, string> = {
  none: 'documents.viewer.syncMode.none',
  'pdf-to-md': 'documents.viewer.syncMode.pdfToMd',
  'md-to-pdf': 'documents.viewer.syncMode.mdToPdf',
};

const MODE_LABEL_FALLBACK: Record<PageSyncMode, string> = {
  none: 'None',
  'pdf-to-md': 'PDF → MD',
  'md-to-pdf': 'MD → PDF',
};

const MODE_ARIA_KEY: Record<PageSyncMode, string> = {
  none: 'documents.viewer.syncMode.noneAria',
  'pdf-to-md': 'documents.viewer.syncMode.pdfToMdAria',
  'md-to-pdf': 'documents.viewer.syncMode.mdToPdfAria',
};

const MODE_ARIA_FALLBACK: Record<PageSyncMode, string> = {
  none: 'No page sync — panes scroll independently',
  'pdf-to-md': 'PDF drives Markdown — markdown follows the PDF page',
  'md-to-pdf': 'Markdown drives PDF — PDF follows the markdown page',
};

export function PageSyncModeControl({
  mode,
  onModeChange,
  available = true,
  className,
  compact = true,
  testId = 'pdf-md-sync-mode',
}: PageSyncModeControlProps) {
  const { t } = useTranslation();
  const disabled = !available;
  const effectiveMode: PageSyncMode = disabled ? 'none' : mode;

  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <div
            role="group"
            aria-label={t(
              'documents.viewer.syncMode.groupAria',
              'PDF and Markdown page sync direction',
            )}
            data-testid={testId}
            data-sync={disabled ? 'none' : mode}
            className={cn(
              'inline-flex items-center gap-0.5 rounded bg-background p-0.5',
              className,
            )}
          >
            {PAGE_SYNC_MODES.map((option) => {
              const selected = effectiveMode === option;
              return (
                <Button
                  key={option}
                  type="button"
                  variant={selected ? 'secondary' : 'ghost'}
                  size="sm"
                  disabled={disabled}
                  aria-pressed={selected}
                  aria-label={t(MODE_ARIA_KEY[option], MODE_ARIA_FALLBACK[option])}
                  data-testid={`${testId}-${option}`}
                  className={cn(
                    compact ? 'h-6 px-1.5 text-xs leading-none' : 'h-8 px-2 text-xs',
                    'font-medium',
                  )}
                  onClick={() => onModeChange(option)}
                >
                  {t(MODE_LABEL_KEY[option], MODE_LABEL_FALLBACK[option])}
                </Button>
              );
            })}
          </div>
        </TooltipTrigger>
        <TooltipContent>
          {disabled
            ? t(
                'documents.viewer.syncMode.unavailable',
                'No page markers in this document',
              )
            : t(MODE_ARIA_KEY[mode], MODE_ARIA_FALLBACK[mode])}
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}

export default PageSyncModeControl;
