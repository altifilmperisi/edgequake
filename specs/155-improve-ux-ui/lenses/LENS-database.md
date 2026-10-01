# Lens — Database Expert

Parent: [README](../README.md) · Contract: [08](../08-data-contract.md) · Findings B-*

## Current risks

1. **Cross-tenant scale leak** via `reltuples` on shared AGE parents (B01).
2. **Degrees batch unscoping** (B03) — isolation defect.
3. **Silent lineage LIMIT 5000** without truncation flag (B08).
4. **Inconsistent degree semantics** across SQL paths (B05).
5. OFFSET+COUNT lists; leading-wildcard entity search.

## W3 required SQL/behaviour

```text
  totals:
    SELECT count(*) FROM ag_vertex_child
      WHERE props->>'workspace_id' = $ws  -- use existing helper
    Prefer pg_*_count_by_workspace already present

  degrees/batch:
    JOIN/filter workspace_id = tenant context ALWAYS

  communities list:
    GROUP BY props->>'community_id'
      WHERE workspace filter AND community_id IS NOT NULL

  edge id:
    prefer eq ids + rel_type; expose description/keywords from props
```

## Indexes / performance

- Reuse gin/btree from `ensure_indexes`.
- If workspace counts hot: materialise `workspace_graph_stats(workspace_id,
  node_count, edge_count, graph_version, updated_at)` maintained in merger
  (medium wave).
- Cap BFS fan-out; keep `EDGEQUAKE_GRAPH_QUERY_TIMEOUT_SECS`.

## Migrations

Cheap wave: **prefer zero DDL** (query + DTO only).  
Medium: stats table + triggers/job; optional degree columns on node props
updated by community refresh.

## Verification

Cargo e2e with two workspaces: totals differ; degrees batch cannot read
foreign node ids; communities empty when none.

## Non-goals

New AGE graph per workspace (huge migration) — stay property-isolated.
