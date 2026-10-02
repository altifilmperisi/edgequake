'use client';

import { ExternalLink, FileText } from 'lucide-react';
import Link from 'next/link';
import { useRouter } from 'next/navigation';
import { useCallback, useId, useRef } from 'react';
import { useTranslation } from 'react-i18next';

import type { CitationChunk } from '@/components/query/markdown/citation-resolver';
import {
  Popover,
  PopoverAnchor,
  PopoverContent,
} from '@/components/ui/popover';
import {
  buildCitationHref,
  formatMatchPercent,
  resolveClickIntent,
} from '@/lib/citations/citation-href';
import { getConfidenceLabel } from '@/lib/citations/confidence';
import { formatPassagePreview } from '@/lib/citations/passage-text';
import { formatChunkPageBadge } from '@/lib/utils/document-url';
import { cn } from '@/lib/utils';

import { useHoverIntent } from './use-hover-intent';

export interface InlineCitationProps {
  index: number;
  chunk: CitationChunk;
  className?: string;
}

/**
 * Citation chip. Click → open the source (page + passage highlighted);
 * hover / focus → preview card with an explicit "Open source" action.
 * Touch devices (no hover) tap to preview, then use the card's link.
 */
export function InlineCitation({ index, chunk, className }: InlineCitationProps) {
  const { t } = useTranslation();
  const router = useRouter();
  const { open, show, hide, setOpen } = useHoverIntent();
  const triggerRef = useRef<HTMLButtonElement>(null);
  const contentId = useId();

  const href = buildCitationHref(chunk);
  const pageBadge = formatChunkPageBadge(chunk.page_start, chunk.page_end);
  const title = chunk.title || t('query.citations.untitled', 'Untitled');
  const percent = formatMatchPercent(chunk.score);
  const confidence = getConfidenceLabel(chunk.score);
  const preview = formatPassagePreview(chunk.content, false);

  const closeAndReturnFocus = useCallback(() => {
    setOpen(false);
    requestAnimationFrame(() => triggerRef.current?.focus());
  }, [setOpen]);

  const handleClick = (e: React.MouseEvent<HTMLButtonElement>) => {
    e.preventDefault();
    const canHover =
      typeof window === 'undefined' ||
      window.matchMedia?.('(hover: hover)').matches !== false;
    const intent = resolveClickIntent({
      metaKey: e.metaKey,
      ctrlKey: e.ctrlKey,
      shiftKey: e.shiftKey,
      button: e.button,
      canHover,
    });
    if (intent === 'preview') {
      setOpen(!open);
    } else if (intent === 'new-tab') {
      window.open(href, '_blank', 'noopener,noreferrer');
    } else {
      setOpen(false);
      router.push(href);
    }
  };

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverAnchor asChild>
        <button
          ref={triggerRef}
          type="button"
          className={cn(
            'inline-flex cursor-pointer items-center rounded-md bg-primary/10 px-1.5 py-0.5 text-xs font-medium text-primary',
            'hover:bg-primary/20 hover:underline underline-offset-2 transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/50',
            className,
          )}
          data-testid={`query-inline-citation-${index}`}
          data-href={href}
          aria-expanded={open}
          aria-haspopup="true"
          aria-controls={open ? contentId : undefined}
          aria-label={t('query.citations.openSourceAria', 'Open source {{index}}: {{title}}', {
            index,
            title: pageBadge ? `${title}, ${pageBadge}` : title,
          })}
          onClick={handleClick}
          onMouseEnter={show}
          onMouseLeave={hide}
          onFocus={show}
          onBlur={hide}
          onKeyDown={(e) => {
            if (e.key === 'Escape' && open) {
              e.preventDefault();
              closeAndReturnFocus();
            }
          }}
        >
          [{index}]
        </button>
      </PopoverAnchor>
      <PopoverContent
        id={contentId}
        className="w-80 p-0 overflow-hidden"
        align="start"
        // Hover-opened: keep focus on the chip so typing / Enter still work.
        onOpenAutoFocus={(e) => e.preventDefault()}
        onCloseAutoFocus={(e) => e.preventDefault()}
        onMouseEnter={show}
        onMouseLeave={hide}
        onKeyDown={(e) => {
          if (e.key === 'Escape') {
            e.preventDefault();
            closeAndReturnFocus();
          }
        }}
        data-testid="query-citation-card"
      >
        <div className="space-y-2.5 p-3.5">
          <div className="flex items-start gap-2">
            <FileText className="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground" aria-hidden />
            <div className="min-w-0 flex-1">
              <p className="truncate text-sm font-medium leading-tight" title={title}>
                {title}
              </p>
              <p className="mt-0.5 text-xs text-muted-foreground">
                {t('query.citations.inlineSource', 'Source #{{index}}', { index })}
                {pageBadge ? ` · ${pageBadge}` : ''}
              </p>
            </div>
            <span
              className={cn('shrink-0 text-xs font-medium tabular-nums', confidence.color)}
              title={t('query.citations.matchPercent', '{{pct}}% match', { pct: percent })}
            >
              {t(confidence.labelKey, confidence.defaultLabel)} · {percent}%
            </span>
          </div>

          {preview ? (
            <p className="line-clamp-4 rounded-lg bg-muted/40 px-2.5 py-2 text-xs leading-relaxed text-foreground/80">
              {preview}
            </p>
          ) : null}
        </div>

        <Link
          href={href}
          className="group flex items-center justify-between gap-2 border-t bg-muted/30 px-3.5 py-2 text-xs font-medium text-primary transition-colors hover:bg-primary/10 focus-visible:bg-primary/10 focus-visible:outline-none"
          data-testid="query-citation-open-source"
          onClick={() => setOpen(false)}
        >
          <span>
            {pageBadge
              ? t('query.citations.openSourcePage', 'Open source · {{page}}', { page: pageBadge })
              : t('query.citations.openSource', 'Open source')}
          </span>
          <ExternalLink
            className="h-3.5 w-3.5 transition-transform group-hover:translate-x-0.5"
            aria-hidden
          />
        </Link>
      </PopoverContent>
    </Popover>
  );
}
