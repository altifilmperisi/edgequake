/**
 * SPEC-157 W2 — Source pane: verify a citation beside the chat.
 * Header (title · page · view) → viewer → cited passage with AI actions.
 */
"use client";

import {
  DocumentSourceSkeleton,
  DocumentSourceView,
  type SourceViewMode,
} from "@/components/documents/document-source-view";
import { cn } from "@/lib/utils";
import { useDocumentSource } from "@/hooks/use-document-source";
import { useQueryScope } from "@/hooks/use-query-scope";
import type { SourceLocation } from "@/lib/query/companion-pane";
import { formatChunkPageBadge } from "@/lib/utils/document-url";
import { useCompanionPaneStore } from "@/stores/use-companion-pane-store";
import { FileQuestion, FileText } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { PaneNotice } from "./pane-notice";
import { SourcePassageStrip } from "./source-passage-strip";

interface SourcePaneProps {
  location: SourceLocation;
  onQuote: (text: string) => void;
}

const VIEW_MODES: SourceViewMode[] = ["page", "text"];

export function SourcePane({ location, onQuote }: SourcePaneProps) {
  const { t } = useTranslation();
  const seq = useCompanionPaneStore((s) => s.seq);
  const scope = useQueryScope();
  const source = useDocumentSource(location.documentId);
  const [mode, setMode] = useState<SourceViewMode>("page");

  const { document } = source;
  const title =
    document?.title || document?.file_name || location.title || location.documentId;
  const pageBadge = formatChunkPageBadge(location.page, location.pageEnd);
  const inScope = scope.ids.includes(location.documentId);

  if (source.isLoading || (!document && !source.isError)) {
    return <DocumentSourceSkeleton />;
  }
  if (source.isNotFound) {
    return (
      <PaneNotice
        icon={FileQuestion}
        title={t("query.companion.sourceGone", "This document is no longer available")}
        description={t(
          "query.companion.sourceGoneHint",
          "It may have been deleted since this answer was written.",
        )}
        testId="companion-source-gone"
      />
    );
  }
  if (source.isError || !document) {
    return (
      <PaneNotice
        icon={FileQuestion}
        tone="error"
        title={t("query.companion.errorSource", "Couldn't load this source")}
        description={t(
          "query.companion.errorSourceHint",
          "Check your connection and try again.",
        )}
        action={{
          label: t("common.tryAgain", "Try again"),
          onClick: () => void source.refetch(),
        }}
        testId="companion-source-error"
      />
    );
  }

  return (
    <div className="flex h-full min-h-0 flex-col" data-testid="companion-source">
      <div className="flex shrink-0 items-center gap-2 border-b px-3 py-2">
        <FileText className="h-4 w-4 shrink-0 text-muted-foreground" aria-hidden />
        <h3
          className="min-w-0 flex-1 truncate text-sm font-medium"
          title={title}
          data-testid="companion-source-title"
        >
          {title}
        </h3>
        {pageBadge ? (
          <span
            className="shrink-0 rounded-full bg-primary/10 px-2 py-0.5 text-[11px] font-semibold text-primary"
            data-testid="companion-source-page"
          >
            {pageBadge}
          </span>
        ) : null}
        {source.pdfId ? (
          <div
            role="group"
            aria-label={t("query.companion.viewMode", "View")}
            className="flex shrink-0 rounded-md bg-muted p-0.5"
          >
            {VIEW_MODES.map((m) => (
              <button
                key={m}
                type="button"
                aria-pressed={mode === m}
                onClick={() => setMode(m)}
                className={cn(
                  "rounded px-2 py-0.5 text-[11px] font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/50",
                  mode === m
                    ? "bg-background text-foreground shadow-sm"
                    : "text-muted-foreground hover:text-foreground",
                )}
                data-testid={`companion-view-${m}`}
              >
                {m === "page"
                  ? t("query.companion.viewPage", "Page")
                  : t("query.companion.viewText", "Text")}
              </button>
            ))}
          </div>
        ) : null}
      </div>

      <div className="min-h-0 flex-1">
        {source.isMarkdownLoading && mode === "text" ? (
          <DocumentSourceSkeleton />
        ) : (
          <DocumentSourceView
            document={document}
            pdfId={source.pdfId}
            location={location}
            mode={mode}
            seq={seq}
          />
        )}
      </div>

      <SourcePassageStrip
        passage={location.passage}
        inScope={inScope}
        onToggleScope={() =>
          inScope
            ? scope.removeDocument(location.documentId)
            : scope.addDocument({ id: location.documentId, title })
        }
        onQuote={onQuote}
      />
    </div>
  );
}
