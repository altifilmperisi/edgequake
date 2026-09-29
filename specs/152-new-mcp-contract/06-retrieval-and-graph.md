# 06 — Retrieval and graph semantics

Parent: [README](README.md) · Tools: [04](04-tool-contract.md) · Objects: [03](03-object-model.md)

## Modes (MUST appear in tool descriptions)

| Mode | Retrieves | Pick when |
|------|-----------|-----------|
| `naive` | Dense chunks | Quote / passage |
| `local` | Seed entities + neighbors | “What is X?” |
| `global` | Community / theme summaries | “What is this corpus about?” |
| `hybrid` | Local ∪ global | Mixed |
| `mix` | Fused + rerank | Default if unsure |

Server MUST echo `mode_used` and `mode_reason`.

Project from existing `ModeSelection`:

| Field | Maps to |
|-------|---------|
| `ModeSelection.effective` | `mode_used` |
| `ModeSelection.requested` + `intent` / adaptive flag | `mode_reason` text |

If the server overrides the requested mode, that MUST appear in `mode_reason`, not a silent swap.

`enable_rerank` default **true** on `mix` / `hybrid` (matches today’s `ContextRetrievalRequest`).

---

## Scope

```text
scope = workspace | documents
document_ids XOR document_pattern   (prefer document_ids; reject both-set)
```

| Case | Behavior |
|------|----------|
| `document_ids` set | Filter to those documents; `cross_document` false unless multiple ids |
| `document_pattern` set | Resolve via existing document filter resolver |
| Neither | Workspace-wide; MUST compute and return `cross_document` from distinct `file_name` / `document_id` in hits |
| Query mentions a filename | SHOULD auto-scope to that document and say so in `mode_reason` or a `scope_note` field |

`DocumentFilter.document_ids` already exists in Rust ([`query_types.rs`](../../edgequake/crates/edgequake-api/src/handlers/query_types.rs)) and is applied by `QueryContextService`. MCP schemas MUST advertise it. Acceptance test 2: scoped query returns **zero** chunks whose `file_name` is an unrelated PDF.

---

## Ranking and `score_type`

Scores on hits MUST be either:

- all in `[0, 1]` with `score_type: "unit_interval"`, **or**
- raw with `score_type: "raw"` declared on the list.

**Do not clamp.** Mixing BM25-ish `10.14` and cosine `0.65` in one list without `score_type` is non-conformant.

Today’s engine mostly uses cosine / fusion scores near `[0,1]`, but mix/BM25 union paths can exceed 1 ([`l2_bm25_union.rs`](../../edgequake/crates/edgequake-query/src/l2_bm25_union.rs), keyword boost). Projection policy:

1. Collect scores for the emitted hit list.
2. If any score is outside `[0,1]` → `score_type: "raw"`.
3. Else → `score_type: "unit_interval"`.

`rerank_score` MAY appear as an optional field; it MUST NOT be mixed unlabeled with `score`.

---

## Search → hits projection

`eq_search` algorithm (normative intent):

1. Resolve mode + document filter (including `document_ids`).
2. Run the same retrieve path used by `search_context` (cache full bundle under `retrieval_id`).
3. Build candidate hits from bundle chunks (primary), then entities, then documents as needed.
4. Sort by score desc; take `limit`.
5. Snippet ≤ 240 chars; apply budget item caps if the hit list metadata itself would bloat.
6. Emit envelope with `hits`, `documents_considered`, `cross_document`, `mode_*`, `expires_at`, `score_type`.

REST `POST /query/context/search` MAY continue returning one summary row; MCP MUST NOT.

---

## Fetch views

| View | Must include | Must omit by default |
|------|--------------|----------------------|
| `toc` | Document summaries, ≤8 entities, ≤3 snippets | Full chunk bodies, large subgraph |
| `chunks` | Selected chunk text + lineage | Full graph unless `include_subgraph` |
| `entities` | Compact entities + typed edges among them | Chunk bodies |
| `citations` | Quote + page + chunk_id + score | Subgraph |
| `full` | Chunks + compact subgraph | Still budget-capped |

Id subsetting: if `ids` is non-empty, only those objects (plus minimal parent document metadata) appear.

Cursor resume: two fetch pages concatenate without duplicate chunk ids (acceptance test 9).

---

## Graph quality rules (agent-visible)

1. Deduplicate slugs (`ACTION_FUSION` == “Action Fusion”).
2. Attach every entity to `document_ids`.
3. Typed edges preferred; generic `RELATED_TO` only with low confidence and hidden unless `include_weak_edges: true`.
4. Keep figure / `DRAWING` / default `Artifact` nodes off the default path unless `include_artifacts: true`.
5. Cap neighborhood degree: return top-k neighbors by edge weight.

### Projection vs extraction

| Concern | Owner |
|---------|-------|
| Hide artifacts / weak edges / cap degree / `one_liner` | MCP projection (L1+) |
| Retype edges to `PART_OF` / `IMPLEMENTS` / … | Pipeline extraction follow-on |

Without projection rules, a perfect MCP envelope still drowns the agent. Without extraction improvement, agents still see weak graphs after filters — track as a linked follow-on, not an L1 gate.

---

## `eq_retrieve`

Compose:

1. `eq_search` (same args).
2. `eq_fetch` with `view=chunks`, same `retrieval_id` and `budget`.

Return union envelope: `hits` from step 1 + chunk view fields from step 2. Do not drop hits.
