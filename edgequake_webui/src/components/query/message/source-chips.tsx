"use client";

import { Button } from "@/components/ui/button";
import type { QueryContext } from "@/types";
import { useTranslation } from "react-i18next";
import { InlineCitation } from "../citations/citation-popover";
import { buildCitationResolver } from "@/lib/citations/resolve-citation";
import { useMemo } from "react";

interface SourceChipsProps {
  context?: QueryContext | null;
  onOpenSources?: () => void;
  /** Show even while streaming (Q10 — sources before tokens) */
  compact?: boolean;
}

/** Compact source chip row shown as soon as context arrives. */
export function SourceChips({
  context,
  onOpenSources,
  compact = true,
}: SourceChipsProps) {
  const { t } = useTranslation();
  const chunks = context?.chunks ?? [];
  const entities = context?.entities ?? [];
  const resolveCitation = useMemo(
    () => buildCitationResolver(context),
    [context],
  );

  if (chunks.length === 0 && entities.length === 0) {
    if (context) {
      return (
        <p
          className="text-xs text-muted-foreground mb-2"
          data-testid="query-no-sources"
        >
          {t("query.citations.noSourcesFound", "No sources found")}
        </p>
      );
    }
    return null;
  }

  const docIds = [
    ...new Set(chunks.map((c) => c.document_id).filter(Boolean)),
  ] as string[];

  return (
    <div
      className="flex flex-wrap items-center gap-1.5 mb-2"
      data-testid="query-source-chips"
    >
      <span className="text-xs text-muted-foreground shrink-0">
        {t("query.citations.sourcesLabel", "Sources")}
      </span>
      {chunks.slice(0, 4).map((chunk, i) => {
        const resolved = resolveCitation?.(String(i + 1));
        if (resolved) {
          return (
            <InlineCitation
              key={chunk.chunk_id ?? `${chunk.document_id}-${i}`}
              index={resolved.index}
              chunk={resolved.chunk}
              className="bg-secondary text-secondary-foreground"
            />
          );
        }
        const name =
          chunk.file_path?.split("/").pop()?.replace(/\.[^.]+$/, "") ||
          chunk.document_id?.slice(0, 8) ||
          "doc";
        return (
          <span
            key={chunk.chunk_id ?? `${chunk.document_id}-${i}`}
            className="inline-flex max-w-[10rem] truncate rounded-md bg-secondary px-2 py-0.5 text-xs"
          >
            {name}
          </span>
        );
      })}
      {docIds.length > 4 || chunks.length > 4 ? (
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="h-6 px-2 text-xs"
          onClick={onOpenSources}
        >
          {t("query.citations.more", "+{{count}} more", {
            count: Math.max(0, chunks.length - 4),
          })}
        </Button>
      ) : null}
      {!compact && entities.length > 0 ? (
        <span className="text-xs text-muted-foreground">
          {t("query.citations.entityCount", "{{count}} entities", {
            count: entities.length,
          })}
        </span>
      ) : null}
    </div>
  );
}
