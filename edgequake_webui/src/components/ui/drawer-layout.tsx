/**
 * Shared layout primitives for Sheet/drawer settings panels.
 * Keeps spacing, typography, and section rhythm consistent across drawers.
 */
'use client';

import { Badge } from '@/components/ui/badge';
import { Label } from '@/components/ui/label';
import { Slider } from '@/components/ui/slider';
import { Switch } from '@/components/ui/switch';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import { cn } from '@/lib/utils';
import { Info } from 'lucide-react';
import type { ReactNode } from 'react';

/** Scrollable body inside a sheet with comfortable gutters. */
export function DrawerBody({
  className,
  children,
  ...props
}: React.ComponentProps<'div'>) {
  return (
    <div
      className={cn('px-5 py-5 sm:px-6 space-y-6', className)}
      {...props}
    >
      {children}
    </div>
  );
}

/** Section block: icon + title, then a bordered content card. */
export function DrawerSection({
  icon,
  title,
  children,
  className,
  contentClassName,
}: {
  icon?: ReactNode;
  title: ReactNode;
  children: ReactNode;
  className?: string;
  contentClassName?: string;
}) {
  return (
    <section className={cn('space-y-3', className)}>
      <div className="flex items-center gap-2">
        {icon}
        <h3 className="text-xs font-semibold uppercase tracking-[0.08em] text-muted-foreground">
          {title}
        </h3>
      </div>
      <div
        className={cn(
          'rounded-xl border bg-muted/15 p-4 space-y-4',
          contentClassName,
        )}
      >
        {children}
      </div>
    </section>
  );
}

/** Label + optional hint above a control. */
export function DrawerField({
  label,
  hint,
  htmlFor,
  children,
  className,
  trailing,
}: {
  label: ReactNode;
  hint?: ReactNode;
  htmlFor?: string;
  children: ReactNode;
  className?: string;
  trailing?: ReactNode;
}) {
  return (
    <div className={cn('space-y-2', className)}>
      <div className="flex items-center justify-between gap-3">
        <Label htmlFor={htmlFor} className="text-sm font-medium leading-none">
          {label}
        </Label>
        {trailing}
      </div>
      {hint ? (
        <p className="text-xs text-muted-foreground leading-snug">{hint}</p>
      ) : null}
      {children}
    </div>
  );
}

/** Toggle row: title + description on the left, switch on the right. */
export function DrawerToggleRow({
  id,
  label,
  description,
  checked,
  onCheckedChange,
  disabled,
  'data-testid': testId,
}: {
  id: string;
  label: ReactNode;
  description?: ReactNode;
  checked: boolean;
  onCheckedChange: (checked: boolean) => void;
  disabled?: boolean;
  'data-testid'?: string;
}) {
  return (
    <div className="flex items-start justify-between gap-4">
      <div className="min-w-0 flex-1 space-y-1 pr-1">
        <Label htmlFor={id} className="text-sm font-medium cursor-pointer">
          {label}
        </Label>
        {description ? (
          <p className="text-xs text-muted-foreground leading-snug">
            {description}
          </p>
        ) : null}
      </div>
      <Switch
        id={id}
        data-testid={testId}
        checked={checked}
        onCheckedChange={onCheckedChange}
        disabled={disabled}
        className="mt-0.5 shrink-0"
      />
    </div>
  );
}

/** Slider with value badge, optional help tooltip, and end labels. */
export function DrawerSliderField({
  label,
  value,
  displayValue,
  onValueChange,
  min,
  max,
  step,
  hint,
  startLabel,
  endLabel,
  icon,
}: {
  label: ReactNode;
  value: number;
  displayValue?: ReactNode;
  onValueChange: (value: number) => void;
  min: number;
  max: number;
  step?: number;
  hint?: ReactNode;
  startLabel?: ReactNode;
  endLabel?: ReactNode;
  icon?: ReactNode;
}) {
  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between gap-3">
        <div className="flex items-center gap-1.5 min-w-0">
          {icon}
          <Label className="text-sm font-medium">{label}</Label>
          {hint ? (
            <TooltipProvider>
              <Tooltip>
                <TooltipTrigger aria-label={`${String(label)} help`}>
                  <Info className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                </TooltipTrigger>
                <TooltipContent side="top" className="max-w-[220px]">
                  <p className="text-xs leading-snug">{hint}</p>
                </TooltipContent>
              </Tooltip>
            </TooltipProvider>
          ) : null}
        </div>
        <Badge variant="secondary" className="font-mono text-xs h-5 px-2 shrink-0">
          {displayValue ?? value}
        </Badge>
      </div>
      <Slider
        value={[value]}
        onValueChange={([next]) => onValueChange(next)}
        min={min}
        max={max}
        step={step}
        className="w-full"
      />
      {(startLabel || endLabel) && (
        <div className="flex justify-between gap-3 text-xs text-muted-foreground">
          <span>{startLabel}</span>
          <span className="text-right">{endLabel}</span>
        </div>
      )}
    </div>
  );
}
