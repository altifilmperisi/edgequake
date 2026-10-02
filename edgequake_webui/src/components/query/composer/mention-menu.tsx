"use client";

import { cn } from "@/lib/utils";
import { FLOATING_EDGE } from "@/components/ui/surface";
import type { DocumentSearchItem } from "@/types";
import { AtSign, FileText, Loader2 } from "lucide-react";
import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";

export const MENTION_LISTBOX_ID = "query-mention-listbox";
export const mentionOptionId = (id: string) => `query-mention-opt-${id}`;

interface MentionMenuProps {
  query: string;
  items: DocumentSearchItem[];
  activeIndex: number;
  isLoading: boolean;
  onHover: (index: number) => void;
  onPick: (doc: DocumentSearchItem) => void;
}

/** Floating `@` document list. Keyboard is owned by the textarea (combobox pattern). */
export function MentionMenu({
  query,
  items,
  activeIndex,
  isLoading,
  onHover,
  onPick,
}: MentionMenuProps) {
  const { t } = useTranslation();
  const activeRef = useRef<HTMLLIElement | null>(null);

  useEffect(() => {
    activeRef.current?.scrollIntoView({ block: "nearest" });
  }, [activeIndex, items]);

  return (
    <div
      className={cn(
        "overflow-hidden rounded-2xl bg-popover text-popover-foreground",
        FLOATING_EDGE,
        "animate-in fade-in-0 slide-in-from-bottom-1 duration-150",
      )}
      data-testid="query-mention-menu"
    >
      <div className="flex items-center gap-2 px-3 pb-1 pt-2.5 text-[11px] font-medium uppercase tracking-wide text-muted-foreground">
        <AtSign className="h-3 w-3" aria-hidden />
        <span>{t("query.mention.title", "Scope to document")}</span>
        {isLoading ? (
          <Loader2 className="ml-auto h-3 w-3 animate-spin" aria-hidden />
        ) : null}
      </div>

      <ul
        id={MENTION_LISTBOX_ID}
        role="listbox"
        aria-label={t("query.mention.list", "Matching documents")}
        className="max-h-[240px] overflow-y-auto overscroll-contain p-1.5 pt-0.5"
      >
        {items.map((doc, index) => {
          const active = index === activeIndex;
          return (
            <li
              key={doc.id}
              id={mentionOptionId(doc.id)}
              ref={active ? activeRef : undefined}
              role="option"
              aria-selected={active}
              data-testid="query-mention-option"
              onMouseMove={() => !active && onHover(index)}
              onMouseDown={(e) => {
                // Keep textarea focus; select on mousedown so blur can't swallow it.
                e.preventDefault();
                onPick(doc);
              }}
              className={cn(
                "flex cursor-pointer items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm",
                "transition-colors duration-100",
                active ? "bg-accent text-accent-foreground" : "text-foreground/85",
              )}
            >
              <FileText
                className={cn(
                  "h-4 w-4 shrink-0",
                  active ? "text-primary" : "text-muted-foreground",
                )}
                aria-hidden
              />
              <span className="min-w-0 flex-1 truncate" title={doc.title}>
                {highlight(doc.title, query)}
              </span>
              {active ? (
                <kbd className="shrink-0 rounded border bg-background px-1.5 py-0.5 text-[10px] font-medium text-muted-foreground">
                  ↵
                </kbd>
              ) : null}
            </li>
          );
        })}

        {!isLoading && items.length === 0 ? (
          <li className="px-3 py-4 text-center text-xs text-muted-foreground" role="presentation">
            {query
              ? t("query.mention.none", "No document matches “{{query}}”", { query })
              : t("query.mention.empty", "No completed documents yet")}
          </li>
        ) : null}
      </ul>

      <div className="flex items-center gap-3 border-t bg-muted/30 px-3 py-1.5 text-[11px] text-muted-foreground">
        <span>↑↓ {t("query.mention.navigate", "navigate")}</span>
        <span>↵ {t("query.mention.select", "select")}</span>
        <span>esc {t("query.mention.dismiss", "dismiss")}</span>
      </div>
    </div>
  );
}

function highlight(title: string, query: string) {
  const q = query.trim();
  if (!q) return title;
  const i = title.toLowerCase().indexOf(q.toLowerCase());
  if (i < 0) return title;
  return (
    <>
      {title.slice(0, i)}
      <mark className="rounded-sm bg-primary/15 px-0 text-inherit">
        {title.slice(i, i + q.length)}
      </mark>
      {title.slice(i + q.length)}
    </>
  );
}
