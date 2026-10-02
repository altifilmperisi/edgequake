"use client";

import {
  BookOpen,
  GitBranch,
  Lightbulb,
  MessageSquare,
  Search,
  Sparkles,
} from "lucide-react";
import { memo, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { useQuery } from "@tanstack/react-query";

import {
  getQueryEmptyCopy,
  isChatQueryMode,
} from "@/lib/query/query-empty-copy";
import type { QueryMode } from "@/types/query";
import { apiClient } from "@/lib/api/client";

export interface QueryEmptyStateProps {
  onSuggestionClick?: (text: string) => void;
  /** Active query mode — Chat (bypass) uses chatbot copy, not KG copy. */
  mode?: QueryMode;
}

type StatsPayload = {
  entity_count?: number;
  document_count?: number;
  relationship_count?: number;
};

type EntityHit = { name?: string; label?: string; entity_type?: string };

/** Empty query chat state — no gradients (Q04/Q19); corpus-derived suggestions (Q20). */
export const QueryEmptyState = memo(function QueryEmptyState({
  onSuggestionClick,
  mode = "mix",
}: QueryEmptyStateProps) {
  const { t } = useTranslation();
  const isChat = isChatQueryMode(mode);
  const copy = getQueryEmptyCopy(mode);

  const { data: stats } = useQuery({
    queryKey: ["workspace-stats-empty"],
    queryFn: () => apiClient<StatsPayload>("/workspaces/current/stats").catch(() => null),
    staleTime: 60_000,
    enabled: !isChat,
  });

  const { data: topEntities } = useQuery({
    queryKey: ["empty-top-entities"],
    queryFn: async () => {
      try {
        const res = await apiClient<{ items?: EntityHit[] }>(
          "/graph/nodes/search?q=&limit=4",
        );
        return res.items ?? [];
      } catch {
        return [] as EntityHit[];
      }
    },
    staleTime: 60_000,
    enabled: !isChat,
  });

  const suggestionIcons = isChat
    ? [
        <MessageSquare key="0" className="h-4 w-4" />,
        <Lightbulb key="1" className="h-4 w-4" />,
        <Search key="2" className="h-4 w-4" />,
        <BookOpen key="3" className="h-4 w-4" />,
      ]
    : [
        <Search key="0" className="h-4 w-4" />,
        <Lightbulb key="1" className="h-4 w-4" />,
        <GitBranch key="2" className="h-4 w-4" />,
        <BookOpen key="3" className="h-4 w-4" />,
      ];

  const corpusSuggestions = useMemo(() => {
    if (isChat || !topEntities?.length) return null;
    return topEntities.slice(0, 4).map((e, i) => {
      const name = e.label || e.name || "";
      const display = name.replace(/_/g, " ");
      return {
        icon: suggestionIcons[i] ?? <Search className="h-4 w-4" />,
        text: t(
          "query.corpusSuggestion",
          "What do we know about {{name}}?",
          { name: display },
        ),
      };
    });
  }, [isChat, topEntities, t, suggestionIcons]);

  const suggestions =
    corpusSuggestions ??
    copy.suggestions.map((text, i) => ({
      icon: suggestionIcons[i] ?? <Search className="h-4 w-4" />,
      text: isChat
        ? t(`query.chatSuggestions.${i}`, text)
        : t(`query.suggestions.${i}`, text),
    }));

  const hasDocs = (stats?.document_count ?? 0) > 0;
  const entityCount = stats?.entity_count ?? 0;
  const relCount = stats?.relationship_count ?? 0;

  return (
    <div className="flex flex-col items-center justify-center h-full py-12 px-4">
      <div
        className="mb-6 flex h-14 w-14 items-center justify-center rounded-2xl bg-primary text-primary-foreground"
        aria-hidden="true"
      >
        {isChat ? (
          <MessageSquare className="h-7 w-7" />
        ) : (
          <Sparkles className="h-7 w-7" />
        )}
      </div>

      <h2 className="text-2xl font-bold mb-2 text-center">
        {isChat
          ? t("query.chatEmptyTitle", copy.title)
          : t("query.emptyTitle", copy.title)}
      </h2>
      <p className="text-muted-foreground text-center mb-6 max-w-lg leading-relaxed">
        {isChat
          ? t("query.chatEmptyDescription", copy.description)
          : t("query.emptyDescription", copy.description)}
      </p>

      {!isChat && stats && (
        <div
          className="flex items-center gap-4 mb-8 px-6 py-3 bg-muted/30 rounded-full border"
          role="status"
        >
          <span className="text-sm">
            <span className="font-medium">{entityCount}</span>{" "}
            <span className="text-muted-foreground">
              {t("query.stats.entities", "entities")}
            </span>
          </span>
          <span className="w-px h-4 bg-border" aria-hidden />
          <span className="text-sm">
            <span className="font-medium">{relCount}</span>{" "}
            <span className="text-muted-foreground">
              {t("query.stats.relationships", "relationships")}
            </span>
          </span>
        </div>
      )}

      {!isChat && !hasDocs && stats ? (
        <p className="text-sm text-muted-foreground mb-6 text-center max-w-md">
          {t(
            "query.emptyNoDocs",
            "No documents yet — upload documents to ground answers in your knowledge graph.",
          )}
        </p>
      ) : null}

      {onSuggestionClick && (
        <div className="w-full max-w-2xl space-y-3">
          <p className="text-sm font-medium text-muted-foreground text-center mb-3">
            {t("query.tryAsking", "Try asking:")}
          </p>
          <div
            className="grid grid-cols-1 md:grid-cols-2 gap-2"
            role="list"
            aria-label={t("query.suggestedQueries", "Suggested queries")}
          >
            {suggestions.map((suggestion, i) => (
              <button
                key={i}
                type="button"
                onClick={() => onSuggestionClick(suggestion.text)}
                className="group flex items-start gap-3 text-left px-4 py-3.5 rounded-xl border bg-card hover:bg-muted/50 hover:border-primary/30 transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/50"
                role="listitem"
                aria-label={suggestion.text}
              >
                <div
                  className="p-1.5 rounded-lg bg-muted group-hover:bg-primary/10 transition-colors shrink-0"
                  aria-hidden="true"
                >
                  {suggestion.icon}
                </div>
                <span className="text-sm leading-relaxed">{suggestion.text}</span>
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
});
