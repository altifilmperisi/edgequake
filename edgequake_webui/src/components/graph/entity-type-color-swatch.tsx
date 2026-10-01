'use client';

/**
 * Editable entity-type color swatch (SPEC-102).
 *
 * Popover with native color input + hex text + reset. Parent owns persistence.
 */

import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover';
import {
  ENTITY_TYPE_COLORS,
  canonicalizeEntityTypeHex,
  isValidEntityTypeHex,
  resolveEntityTypeColor,
  resolveEntityTypeShape,
  type EntityTypeShape,
} from '@/lib/graph/entity-type-colors';
import { cn } from '@/lib/utils';
import { RotateCcw } from 'lucide-react';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';

const SHAPE_CLASS: Record<EntityTypeShape, string> = {
  circle: 'rounded-full',
  square: 'rounded-sm',
  diamond: 'rounded-sm rotate-45 scale-90',
  triangle: 'rounded-sm',
  hexagon: 'rounded-md',
  star: 'rounded-full ring-1 ring-offset-1',
  pentagon: 'rounded-md',
  cross: 'rounded-none',
};

export interface EntityTypeColorSwatchProps {
  entityType: string;
  /** Current resolved display color. */
  color: string;
  /** Workspace overrides (to detect custom vs default). */
  overrides?: Record<string, string> | null;
  onChange: (hex: string) => void;
  onReset: () => void;
  disabled?: boolean;
  className?: string;
  /** Stop click from toggling parent row visibility. */
  stopPropagation?: boolean;
}

export function EntityTypeColorSwatch({
  entityType,
  color,
  overrides,
  onChange,
  onReset,
  disabled,
  className,
  stopPropagation = true,
}: EntityTypeColorSwatchProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const [hexDraft, setHexDraft] = useState(color);
  const key = entityType.toUpperCase();
  const shape = resolveEntityTypeShape(entityType);
  const defaultHex =
    ENTITY_TYPE_COLORS[key] ?? ENTITY_TYPE_COLORS.DEFAULT;
  const isCustom =
    !!overrides &&
    Object.entries(overrides).some(
      ([k, v]) =>
        k.toUpperCase() === key &&
        canonicalizeEntityTypeHex(v)?.toLowerCase() !== defaultHex.toLowerCase(),
    );

  useEffect(() => {
    if (open) setHexDraft(color);
  }, [open, color]);

  const applyHex = (raw: string) => {
    const canonical = canonicalizeEntityTypeHex(raw);
    if (!canonical) return;
    onChange(canonical);
    setHexDraft(canonical);
  };

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <button
          type="button"
          data-testid="entity-type-color-swatch"
          data-entity-type={key}
          disabled={disabled}
          className={cn(
            'w-3.5 h-3.5 shrink-0 ring-2 ring-background shadow-sm',
            SHAPE_CLASS[shape],
            'focus:outline-none focus-visible:ring-2 focus-visible:ring-primary/50',
            disabled && 'opacity-50 cursor-not-allowed',
            className,
          )}
          style={{ backgroundColor: color }}
          data-shape={shape}
          aria-label={t(
            'graph.colors.editSwatch',
            'Edit color for {{type}}',
            { type: key },
          )}
          title={t('graph.colors.editSwatch', 'Edit color for {{type}}', {
            type: key,
          })}
          onClick={(e) => {
            // Do not preventDefault — Radix PopoverTrigger needs the click.
            if (stopPropagation) e.stopPropagation();
          }}
        />
      </PopoverTrigger>
      <PopoverContent
        className="w-56 p-3 space-y-2"
        align="start"
        onClick={(e) => e.stopPropagation()}
      >
        <p className="text-xs font-medium truncate">{key}</p>
        <div className="flex items-center gap-2">
          <input
            type="color"
            data-testid="entity-type-color-picker"
            value={canonicalizeEntityTypeHex(hexDraft) ?? color}
            onChange={(e) => applyHex(e.target.value)}
            className="h-8 w-10 cursor-pointer rounded border border-border bg-transparent p-0.5"
            aria-label={t('graph.colors.picker', 'Color picker')}
          />
          <Input
            value={hexDraft}
            onChange={(e) => setHexDraft(e.target.value)}
            onBlur={() => {
              if (isValidEntityTypeHex(hexDraft)) applyHex(hexDraft);
              else setHexDraft(color);
            }}
            onKeyDown={(e) => {
              if (e.key === 'Enter' && isValidEntityTypeHex(hexDraft)) {
                applyHex(hexDraft);
              }
            }}
            className="h-8 font-mono text-xs"
            spellCheck={false}
            aria-invalid={hexDraft.length > 0 && !isValidEntityTypeHex(hexDraft)}
          />
        </div>
        <p className="text-xs text-muted-foreground">
          {t(
            'graph.colors.defaultHint',
            'Default: {{hex}}',
            { hex: resolveEntityTypeColor(entityType) },
          )}
        </p>
        {isCustom && (
          <Button
            type="button"
            variant="ghost"
            size="sm"
            className="h-7 w-full text-xs"
            data-testid="entity-type-color-reset"
            onClick={() => {
              onReset();
              setOpen(false);
            }}
          >
            <RotateCcw className="h-3 w-3 mr-1.5" />
            {t('graph.colors.reset', 'Reset to default')}
          </Button>
        )}
      </PopoverContent>
    </Popover>
  );
}
