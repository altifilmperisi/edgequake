# 07 — AI engineer specification

Parent: [README](README.md) · Product: [03](03-product-spec.md) ·
Data: [06](06-data-and-db-contract.md)

## Role of AI systems in SPEC-157

SPEC-157 does **not** change retrieval algorithms in the default path. It
changes how humans **inspect** retrieval evidence while chatting, and (W4)
how that inspection feeds the next query’s scope.

## Grounding fidelity

| Requirement | Implementation |
|-------------|----------------|
| Citation → page | Use `page_start` when present (PDF page-aware chunking) |
| Multi-page chunk | Prefer `page_start`; show badge `p.A–B` via `formatChunkPageBadge` |
| Passage highlight | Prefer `chunk_id` + content; fallback `start_line`/`end_line` |
| Text documents | No page → text/markdown viewer scrolled to lines |
| Entity focus | Prefer `graph_node_id` / `id`; fallback normalised `label`/`name` |
| Missing evidence | Empty/partial states — never invent citations |

Industry note: claim-level entailment re-pass is **out of scope**; we render
the retrieval context the model already received.

## Streaming while companion is open

```text
  Stream session (use-query-stream-session)
        |
        +-- updates messages / pending / stage
        |
        +-- MUST NOT remount CompanionShell
        |
        +-- stick-to-bottom: only if user was pinned; opening pane is not
        |   a scroll-away signal for the chat column
        |
        +-- on complete: may refresh answer-graph store for last message;
            if Graph pane showing that message, update focus without flicker
```

Composer remains usable; queued messages (existing) still apply.

## W4 — Pane context as retrieval scope

When Source pane shows document `D`:

1. Offer “Focus next question on this document” (or auto-suggest).
2. Adds `D` to `useQueryScope` document ids (existing `@` scope chips /
   `scopedDocumentIds` already flow into query settings).
3. Clear chip when companion closes **only if** it was auto-added (do not
   clear user-pinned scope).

```text
  openSource(doc D) --optional--> scope.add(D) --> next chat request
                                      document_ids / filter
```

## W4 — Quote-to-ask

From PDF text layer selection (when available):

1. Selection toolbar: “Ask about selection”.
2. Inserts quoted snippet into composer (truncated to safe length, e.g. 500 chars).
3. Does not auto-submit.
4. If no text layer (scan), action hidden or disabled with tip.

Non-PDF text viewer: same pattern on selected DOM text.

## Answer subgraph semantics

`answer-graph.ts` normative behaviour:

```text
  Input: message.context (+ optional stored SubgraphBundle)
  Output:
    nodes: unique entities with display label + type
    edges: relationships with source/target resolved to node keys
    focusIds: preferential graph_node_id list
    unresolved: names that did not resolve

  Cap: if nodes > N (e.g. 80), show first N by relevance + "Show all in Studio"
```

Neighbour expand (W3):

- Prefer existing neighbourhood / ego helpers **against a local graph**, not
  by mutating Studio’s full workspace graph.
- If expand requires a server fetch, scope by workspace and optionally by
  open document ids.

## Cost / latency

| Action | Cost impact |
|--------|-------------|
| Open Source / Graph pane | Client-only + existing GET document/PDF |
| Neighbour expand | Possible small graph API read |
| Quote-to-ask | No LLM until user sends |
| Scope chip | Narrows retrieval (can **reduce** tokens) |

No new LLM calls are introduced by opening panes.

## Safety / honesty

- Do not display a citation target the chunk does not support.
- Cross-workspace document ids → error (EC-157-07), no silent fetch of other
  tenants’ PDFs.
- Bypass / chat mode with no KG context: hide or disable “Show on graph”;
  Source still works if user attached/scoped docs later.

## Evaluation hooks (future, non-blocking)

- Log companion open events (client analytics if present): kind, time-to-ready.
- Sample citation clicks vs full-page opens to measure verify-in-place adoption.

Cross-ref: [01-first-principles](01-first-principles.md) LAW-157-1,8,10 ·
[08-edge-cases](08-edge-cases.md) · [lenses/LENS-ai-engineer.md](lenses/LENS-ai-engineer.md).
