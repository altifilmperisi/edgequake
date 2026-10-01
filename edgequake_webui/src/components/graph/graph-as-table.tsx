/**
 * Graph-as-table accessible alternative — SPEC-155 W5 / LAW-155-8.
 */
"use client";

import { PageHeader } from "@/components/shared/page-header";
import { PageShell } from "@/components/shared/page-shell";
import { useGraphStore } from "@/stores/use-graph-store";
import { degreeTotal } from "@/types/graph";
import { useTranslation } from "react-i18next";

export function GraphAsTable() {
  const { t } = useTranslation();
  const nodes = useGraphStore((s) => s.nodes);
  const edges = useGraphStore((s) => s.edges);
  const selectedNodeId = useGraphStore((s) => s.selectedNodeId);
  const selectNode = useGraphStore((s) => s.selectNode);

  return (
    <PageShell>
      <PageHeader
        title={t("graph.table.title", "Graph as table")}
        description={t(
          "graph.table.description",
          "Accessible list of entities and relationships. Selection syncs with the canvas.",
        )}
      />
      <div className="grid gap-6 lg:grid-cols-2">
        <section aria-labelledby="graph-table-nodes">
          <h2 id="graph-table-nodes" className="mb-2 text-sm font-medium">
            {t("graph.table.nodes", "Entities")} ({nodes.length})
          </h2>
          <div className="max-h-[50vh] overflow-auto rounded-md border">
            <table className="w-full text-sm">
              <thead className="sticky top-0 bg-muted/80 backdrop-blur">
                <tr>
                  <th className="px-3 py-2 text-start font-medium">Label</th>
                  <th className="px-3 py-2 text-start font-medium">Type</th>
                  <th className="px-3 py-2 text-end font-medium">Degree</th>
                </tr>
              </thead>
              <tbody>
                {nodes.map((n) => (
                  <tr
                    key={n.id}
                    className={
                      selectedNodeId === n.id
                        ? "bg-accent"
                        : "hover:bg-muted/40"
                    }
                    onClick={() => selectNode(n.id)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter" || e.key === " ") {
                        e.preventDefault();
                        selectNode(n.id);
                      }
                    }}
                    tabIndex={0}
                    role="button"
                    aria-selected={selectedNodeId === n.id}
                  >
                    <td className="px-3 py-1.5">{n.label}</td>
                    <td className="px-3 py-1.5 text-muted-foreground">
                      {n.node_type}
                    </td>
                    <td className="px-3 py-1.5 text-end tabular-nums">
                      {degreeTotal(n.degree) || "—"}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </section>
        <section aria-labelledby="graph-table-edges">
          <h2 id="graph-table-edges" className="mb-2 text-sm font-medium">
            {t("graph.table.edges", "Relationships")} ({edges.length})
          </h2>
          <div className="max-h-[50vh] overflow-auto rounded-md border">
            <table className="w-full text-sm">
              <thead className="sticky top-0 bg-muted/80 backdrop-blur">
                <tr>
                  <th className="px-3 py-2 text-start font-medium">Source</th>
                  <th className="px-3 py-2 text-start font-medium">Type</th>
                  <th className="px-3 py-2 text-start font-medium">Target</th>
                </tr>
              </thead>
              <tbody>
                {edges.map((e) => (
                  <tr key={e.id ?? `${e.source}-${e.relationship_type}-${e.target}`}>
                    <td className="px-3 py-1.5 font-mono text-xs">{e.source}</td>
                    <td className="px-3 py-1.5">{e.relationship_type}</td>
                    <td className="px-3 py-1.5 font-mono text-xs">{e.target}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </section>
      </div>
    </PageShell>
  );
}
