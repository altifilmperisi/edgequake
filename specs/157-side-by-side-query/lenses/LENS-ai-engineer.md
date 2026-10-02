# Lens — AI Engineer

Parent: [README](../README.md) · AI spec: [07](../07-ai-engineer-spec.md) ·
Data: [06](../06-data-and-db-contract.md)

## Ownership

Faithful rendering of retrieval evidence; answer-subgraph mapping; W4 scope and
quote-to-ask; honesty when evidence is missing.

## Non-goals for this lens

- Changing LightRAG / hybrid retrieval ranking
- Adding claim-entailment LLM pass in v1
- Training or eval harness changes (optional analytics only)

## Mapping responsibilities

`lib/query/answer-graph.ts` is the SSOT for:

- Entity → node id / name fallback
- Relationship endpoint resolution
- Cap N + unresolved list

Must be unit-tested with fixtures from stream context and reloaded messages.

## Interaction with streaming

- Do not block token streaming on pane mounts.
- When a new assistant message completes, Graph pane may retarget **only if**
  it was following “latest”; if user pinned an older message’s graph, keep it.

## Scope chip (W4)

| Source of scope | On companion close |
|-----------------|--------------------|
| Auto from openSource | Remove that doc id |
| User `@` mention / picker | Keep |

## Evaluation mindset

Success = humans verify more often. Instrument open rates if product analytics
exist; do not gate release on online metrics.

## Risks

| Risk | Mitigation |
|------|------------|
| Highlight wrong page | Prefer server `page_start`; never guess |
| Name collision across entities | Prefer graph ids; show unresolved |
| Quote-to-ask injects huge text | Hard truncate + ellipsis |

Cross-ref: LAW-157-1,8,10,11 · EC-157-01,07,15–18,30,31.
