/**
 * SPEC-157 LAW-157-11 — one compact, honest state block for every companion
 * pane (loading failures, missing documents, empty graphs…).
 */
"use client";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import type { LucideIcon } from "lucide-react";

interface PaneNoticeProps {
  icon: LucideIcon;
  title: string;
  description?: string;
  action?: { label: string; onClick: () => void };
  tone?: "neutral" | "error";
  testId?: string;
}

export function PaneNotice({
  icon: Icon,
  title,
  description,
  action,
  tone = "neutral",
  testId,
}: PaneNoticeProps) {
  return (
    <div
      className="flex h-full flex-col items-center justify-center gap-3 px-8 text-center"
      role={tone === "error" ? "alert" : "status"}
      data-testid={testId}
    >
      <span
        className={cn(
          "flex h-11 w-11 items-center justify-center rounded-full",
          tone === "error"
            ? "bg-destructive/10 text-destructive"
            : "bg-muted text-muted-foreground",
        )}
      >
        <Icon className="h-5 w-5" aria-hidden />
      </span>
      <div className="space-y-1">
        <p className="text-sm font-medium text-foreground">{title}</p>
        {description ? (
          <p className="mx-auto max-w-xs text-xs leading-relaxed text-muted-foreground">
            {description}
          </p>
        ) : null}
      </div>
      {action ? (
        <Button type="button" variant="outline" size="sm" onClick={action.onClick}>
          {action.label}
        </Button>
      ) : null}
    </div>
  );
}
