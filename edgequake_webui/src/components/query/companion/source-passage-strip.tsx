/**
 * SPEC-157 W4 — the cited passage under the viewer, with the two AI actions:
 * quote it into the composer, or scope the next questions to this document.
 */
"use client";

import { Button } from "@/components/ui/button";
import { stripMarkdownSyntax } from "@/lib/citations/passage-text";
import { cn } from "@/lib/utils";
import { Check, ChevronDown, Crosshair, Quote } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

interface SourcePassageStripProps {
  passage: string | undefined;
  inScope: boolean;
  onToggleScope: () => void;
  onQuote: (text: string) => void;
}

const QUOTE_MAX_CHARS = 600;

export function SourcePassageStrip({
  passage,
  inScope,
  onToggleScope,
  onQuote,
}: SourcePassageStripProps) {
  const { t } = useTranslation();
  const [expanded, setExpanded] = useState(false);
  const text = stripMarkdownSyntax(passage ?? "").trim();

  return (
    <div
      className="shrink-0 border-t bg-muted/30"
      data-testid="companion-passage-strip"
    >
      {text ? (
        <div className="px-3 pt-2.5">
          <button
            type="button"
            onClick={() => setExpanded((v) => !v)}
            aria-expanded={expanded}
            className="group flex w-full items-start gap-2 text-start focus-visible:outline-none"
          >
            <Quote
              className="mt-0.5 h-3.5 w-3.5 shrink-0 text-primary/60"
              aria-hidden
            />
            <span
              className={cn(
                "min-w-0 flex-1 text-xs leading-relaxed text-foreground/80",
                !expanded && "line-clamp-2",
                expanded && "max-h-40 overflow-y-auto",
              )}
              data-testid="companion-passage-text"
            >
              {text}
            </span>
            <ChevronDown
              className={cn(
                "mt-0.5 h-3.5 w-3.5 shrink-0 text-muted-foreground transition-transform",
                expanded && "rotate-180",
              )}
              aria-hidden
            />
            <span className="sr-only">
              {expanded
                ? t("query.companion.collapsePassage", "Collapse passage")
                : t("query.companion.expandPassage", "Expand passage")}
            </span>
          </button>
        </div>
      ) : null}
      <div className="flex flex-wrap items-center gap-2 px-3 py-2.5">
        {text ? (
          <Button
            type="button"
            size="sm"
            variant="secondary"
            className="h-7 gap-1.5 text-xs"
            onClick={() => onQuote(text.slice(0, QUOTE_MAX_CHARS))}
            data-testid="companion-quote-ask"
          >
            <Quote className="h-3 w-3" aria-hidden />
            {t("query.companion.askAboutThis", "Ask about this")}
          </Button>
        ) : null}
        <Button
          type="button"
          size="sm"
          variant={inScope ? "default" : "outline"}
          className="h-7 gap-1.5 text-xs"
          aria-pressed={inScope}
          onClick={onToggleScope}
          data-testid="companion-scope-toggle"
        >
          {inScope ? (
            <Check className="h-3 w-3" aria-hidden />
          ) : (
            <Crosshair className="h-3 w-3" aria-hidden />
          )}
          {inScope
            ? t("query.companion.inScope", "In scope")
            : t("query.companion.scopeToDoc", "Scope to this document")}
        </Button>
      </div>
    </div>
  );
}
