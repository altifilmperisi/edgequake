# Lens — AI Engineer

Parent: [README](../README.md) · Graph: [06](../06-graph-studio-spec.md) · Contract: [08](../08-data-contract.md) · LAW-155-11

## Mission

Make retrieval **explainable on the graph**: every grounded answer can show
which entities/relationships/chunks participated — without inventing edges.

## Available signals (today)

```text
  chat/query context event
    subgraph.entities[]   name, type, score, degree, lineage
    subgraph.relationships[]  source, target, type, score
    chunks[] score, rerank_score, lineage
  retrieval_quality / truncation
  messages.context JSONB persistence
```

**Gap:** Graph UI ignores `subgraph`. Entity ids `ent:{name}` ≠ graph node ids.

## W3 / W6 contract asks

1. API returns `graph_node_id` (workspace-scoped) on each retrieved entity.
2. Chat `done` includes `subgraph` (+ optional `retrieval_id`).
3. Keep multi `source_document_ids`.
4. UI: `setFocus({ mode: 'answer', ids })` + deep link.

## UX for trust

```text
  Answer card
    [Show on graph]  [Show sources]
         |
         v
  Graph Studio answer mode
    Retrieved nodes bright; others dimmed
    Edge path among retrieved emphasized
    Side panel: score + chunk snippets (existing citations)
```

Do **not** auto-create missing nodes — if entity not in loaded graph, offer
“Load neighbourhood” / “Search server”.

## Evaluation ideas (optional later)

- % answers with ≥1 highlightable entity
- Click-through Show-on-graph
- Acc bench: qualitative explainability (SPEC-001 adjacent — not blocking)

## Out of scope

- Changing extraction prompts / Acc scoring
- Replacing citation markdown (SPEC-142)
- MCP tool UX (SPEC-152) except shared graph ids if exposed later

## Safety

Highlighting must respect workspace isolation (same as graph fetch). Never
fetch degrees/batch without tenant (B03).
