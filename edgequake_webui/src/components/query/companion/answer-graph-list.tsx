/**
 * SPEC-157 — Accessible list view of the answer subgraph (LAW-157-11).
 * Also the WebGL-failure fallback and the keyboard path to select/expand.
 */
"use client";

import { Badge } from "@/components/ui/badge";
import { formatEntityLabel } from "@/lib/graph/label-utils";
import { cn } from "@/lib/utils";
import type { AnswerGraphModel } from "@/lib/query/answer-graph";
import { degreeTotal } from "@/types/graph";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";

interface AnswerGraphListProps {
  model: AnswerGraphModel;
  selectedNodeId: string | null;
  onSelect: (nodeId: string) => void;
}

export function AnswerGraphList({
  model,
  selectedNodeId,
  onSelect,
}: AnswerGraphListProps) {
  const { t } = useTranslation();
  const labelOf = useMemo(() => {
    const map = new Map(model.nodes.map((n) => [n.id, formatEntityLabel(n.label)]));
    return (id: string) => map.get(id) ?? id;
  }, [model.nodes]);
  const answerIds = useMemo(() => new Set(model.answerNodeIds), [model]);

  return (
    <div
      className="h-full overflow-y-auto px-3 py-3 space-y-5"
      data-testid="companion-graph-list"
    >
      <section aria-labelledby="companion-list-entities">
        <h3
          id="companion-list-entities"
          className="mb-1.5 text-[11px] font-semibold uppercase tracking-wide text-muted-foreground"
        >
          {t("query.companion.entities", "Entities")} ({model.nodes.length})
        </h3>
        <ul className="space-y-1">
          {model.nodes.map((n) => (
            <li key={n.id}>
              <button
                type="button"
                onClick={() => onSelect(n.id)}
                aria-pressed={selectedNodeId === n.id}
                className={cn(
                  "flex w-full items-center gap-2 rounded-md border px-2.5 py-1.5 text-start text-sm transition-colors",
                  "hover:bg-muted/60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/50",
                  selectedNodeId === n.id
                    ? "border-primary/40 bg-primary/5"
                    : "border-transparent bg-muted/30",
                )}
              >
                <span className="min-w-0 flex-1 truncate font-medium">
                  {formatEntityLabel(n.label)}
                </span>
                <Badge variant="secondary" className="shrink-0 text-[10px]">
                  {n.node_type}
                </Badge>
                {!answerIds.has(n.id) ? (
                  <span className="shrink-0 text-[10px] text-muted-foreground">
                    {t("query.companion.context", "context")}
                  </span>
                ) : (
                  <span className="shrink-0 text-[10px] tabular-nums text-muted-foreground">
                    {degreeTotal(n.degree)}
                  </span>
                )}
              </button>
            </li>
          ))}
        </ul>
      </section>

      {model.edges.length > 0 ? (
        <section aria-labelledby="companion-list-rels">
          <h3
            id="companion-list-rels"
            className="mb-1.5 text-[11px] font-semibold uppercase tracking-wide text-muted-foreground"
          >
            {t("query.companion.relationships", "Relationships")} (
            {model.edges.length})
          </h3>
          <ul className="space-y-1 text-xs text-muted-foreground">
            {model.edges.map((e) => (
              <li key={e.id} className="rounded-md bg-muted/30 px-2.5 py-1.5">
                <span className="font-medium text-foreground">
                  {labelOf(e.source)}
                </span>{" "}
                <span className="uppercase tracking-wide">
                  {e.relationship_type.replace(/_/g, " ")}
                </span>{" "}
                <span className="font-medium text-foreground">
                  {labelOf(e.target)}
                </span>
              </li>
            ))}
          </ul>
        </section>
      ) : null}
    </div>
  );
}
