/**
 * SPEC-157 W3 — Graph pane: the answer's evidence subgraph beside the chat.
 * Canvas (Sigma, isolated) or an accessible list; expand neighbours on demand;
 * "Open in Graph Studio" is the escape hatch to the full workspace graph.
 */
"use client";

import { EmbeddedAnswerGraph } from "@/components/graph/embedded-answer-graph";
import { Button } from "@/components/ui/button";
import { useAnswerGraphModel } from "@/hooks/use-answer-graph-model";
import { studioAnswerHref } from "@/hooks/use-open-answer-graph";
import {
  buildAnswerGraphModel,
  subgraphFromContext,
  type AnswerGraphModel,
} from "@/lib/query/answer-graph";
import type { QueryMessage } from "@/lib/query/query-interface-types";
import { cn } from "@/lib/utils";
import { ExternalLink, Network, Share2 } from "lucide-react";
import Link from "next/link";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { AnswerGraphList } from "./answer-graph-list";
import { GraphNodeCard } from "./graph-node-card";
import { PaneNotice } from "./pane-notice";

type GraphView = "canvas" | "list";

interface GraphPaneProps {
  messageId: string;
  messages: QueryMessage[];
  /** The conversation is still loading — don't claim the answer is missing. */
  isLoading: boolean;
}

export function GraphPane({ messageId, messages, isLoading }: GraphPaneProps) {
  const { t } = useTranslation();
  const message = messages.find((m) => m.id === messageId);
  const base = useMemo(
    () =>
      message
        ? buildAnswerGraphModel(subgraphFromContext(message.context))
        : null,
    [message],
  );

  if (!message && isLoading) {
    return <PaneNotice icon={Share2} title={t("query.companion.loading", "Loading…")} />;
  }
  if (!base || base.nodes.length === 0) {
    return (
      <PaneNotice
        icon={Network}
        title={t("query.companion.emptyGraph", "No graph for this answer")}
        description={t(
          "query.companion.emptyGraphHint",
          "This answer didn't retrieve any entities from the knowledge graph.",
        )}
        testId="companion-graph-empty"
      />
    );
  }
  // Remount per answer so local selection/expansion never leaks across answers.
  return <AnswerGraphView key={messageId} messageId={messageId} base={base} />;
}

function AnswerGraphView({
  messageId,
  base,
}: {
  messageId: string;
  base: AnswerGraphModel;
}) {
  const { t } = useTranslation();
  const { model, expand, expanding, expanded } = useAnswerGraphModel(base);
  const [view, setView] = useState<GraphView>("canvas");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const selected = model.nodes.find((n) => n.id === selectedId) ?? null;

  const handleExpand = async (nodeId: string) => {
    const outcome = await expand(nodeId);
    if (outcome === "added") return;
    toast.info(
      outcome === "failed"
        ? t("query.companion.expandFailed", "Couldn't load neighbours for this entity")
        : t("query.companion.expandNone", "No new neighbours to show"),
    );
  };

  const list = (
    <AnswerGraphList
      model={model}
      selectedNodeId={selectedId}
      onSelect={setSelectedId}
    />
  );

  return (
    <div className="flex h-full min-h-0 flex-col" data-testid="companion-graph">
      <div className="flex shrink-0 items-center gap-2 border-b px-3 py-2">
        <p
          className="min-w-0 flex-1 truncate text-xs text-muted-foreground"
          data-testid="companion-graph-summary"
        >
          {t("query.companion.graphSummary", "{{nodes}} entities · {{edges}} relationships", {
            nodes: model.nodes.length,
            edges: model.edges.length,
          })}
          {base.hiddenCount > 0
            ? ` · ${t("query.companion.moreInStudio", "+{{count}} more in Studio", {
                count: base.hiddenCount,
              })}`
            : ""}
        </p>
        <div
          role="group"
          aria-label={t("query.companion.viewMode", "View")}
          className="flex shrink-0 rounded-md bg-muted p-0.5"
        >
          {(["canvas", "list"] as const).map((v) => (
            <button
              key={v}
              type="button"
              aria-pressed={view === v}
              onClick={() => setView(v)}
              className={cn(
                "rounded px-2 py-0.5 text-[11px] font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/50",
                view === v
                  ? "bg-background text-foreground shadow-sm"
                  : "text-muted-foreground hover:text-foreground",
              )}
              data-testid={`companion-graph-view-${v}`}
            >
              {v === "canvas"
                ? t("query.companion.viewCanvas", "Graph")
                : t("query.companion.viewList", "List")}
            </button>
          ))}
        </div>
      </div>

      <div className="min-h-0 flex-1">
        {view === "canvas" ? (
          <EmbeddedAnswerGraph
            nodes={model.nodes}
            edges={model.edges}
            selectedNodeId={selectedId}
            onSelect={setSelectedId}
            onExpand={handleExpand}
            fallback={list}
          />
        ) : (
          list
        )}
      </div>

      {selected ? (
        <GraphNodeCard
          node={selected}
          isAnswerNode={model.answerNodeIds.includes(selected.id)}
          isExpanding={expanding === selected.id}
          isExpanded={expanded.has(selected.id)}
          onExpand={() => void handleExpand(selected.id)}
          onClose={() => setSelectedId(null)}
        />
      ) : null}

      <div className="flex shrink-0 items-center justify-end border-t bg-muted/30 px-3 py-2">
        <Button asChild variant="outline" size="sm" className="h-7 gap-1.5 text-xs">
          <Link href={studioAnswerHref(messageId)} data-testid="companion-open-studio">
            <ExternalLink className="h-3 w-3" aria-hidden />
            {t("query.companion.openStudio", "Open in Graph Studio")}
          </Link>
        </Button>
      </div>
    </div>
  );
}
