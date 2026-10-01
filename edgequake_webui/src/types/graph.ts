/** Graph visualization and PDF parser backend types. */

/** SPEC-155 degree SSOT — prefer object; bare number is total (compat). */
export type DegreeValue =
  | number
  | {
      in: number;
      out: number;
      total: number;
    };

export function degreeTotal(degree: DegreeValue | undefined | null): number {
  if (degree == null) return 0;
  if (typeof degree === "number") return degree;
  return degree.total ?? (degree.in ?? 0) + (degree.out ?? 0);
}

export interface GraphNode {
  id: string;
  label: string;
  node_type: string;
  description?: string;
  /** Degree SSOT `{in,out,total}` or bare total (SPEC-155 W3). */
  degree?: DegreeValue;
  /** Server community id (SPEC-155 W3/W5). Prefer over client Louvain. */
  community_id?: string;
  properties?: Record<string, unknown>;
  created_at?: string;
  updated_at?: string;
}

export interface GraphEdge {
  id: string;
  source: string;
  target: string;
  relationship_type: string;
  weight: number;
  description?: string;
  keywords?: string[];
  source_ids: string[];
  properties?: Record<string, unknown>;
  created_at: string;
}

export interface KnowledgeGraph {
  nodes: GraphNode[];
  edges: GraphEdge[];
  metadata: {
    node_count: number;
    edge_count: number;
    entity_types: string[];
    relationship_types: string[];
  };
  /** Whether the graph was truncated due to max_nodes limit */
  is_truncated?: boolean;
  /** Total node count in storage (before truncation) */
  total_nodes?: number;
  /** Total edge count in storage (before truncation) */
  total_edges?: number;
  max_nodes?: number;
}

export type PdfParserBackend = "vision" | "edgeparse" | "auto";
export type WorkspacePdfParserBackendUpdate = PdfParserBackend | "none";
