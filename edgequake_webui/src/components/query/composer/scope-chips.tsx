"use client";

import { FileText, X } from "lucide-react";
import { useTranslation } from "react-i18next";

interface ScopeChipsProps {
  ids: readonly string[];
  titles: Readonly<Record<string, string>>;
  onRemove: (id: string) => void;
}

/** Documents the answer is scoped to — added via `@` or the scope picker. */
export function ScopeChips({ ids, titles, onRemove }: ScopeChipsProps) {
  const { t } = useTranslation();
  if (ids.length === 0) return null;

  return (
    <ul
      className="flex flex-wrap gap-1.5 px-3 pt-3"
      aria-label={t("query.scope.chips", "Scoped documents")}
      data-testid="query-scope-chips"
    >
      {ids.map((id) => {
        const title = titles[id] ?? t("query.scope.unknownDoc", "Document");
        return (
          <li
            key={id}
            className="group inline-flex h-7 max-w-[16rem] items-center gap-1.5 rounded-full bg-primary/[0.07] pl-2.5 pr-1 text-xs font-medium text-foreground/90 ring-1 ring-primary/15 transition-colors hover:bg-primary/[0.11]"
            data-testid="query-scope-chip-doc"
          >
            <FileText className="h-3 w-3 shrink-0 text-primary/80" aria-hidden />
            <span className="truncate" title={title}>
              {title}
            </span>
            <button
              type="button"
              onClick={() => onRemove(id)}
              className="inline-flex h-5 w-5 shrink-0 items-center justify-center rounded-full text-muted-foreground transition-colors hover:bg-foreground/10 hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/40"
              aria-label={t("query.scope.removeDoc", "Remove {{title}} from scope", {
                title,
              })}
            >
              <X className="h-3 w-3" aria-hidden />
            </button>
          </li>
        );
      })}
    </ul>
  );
}
