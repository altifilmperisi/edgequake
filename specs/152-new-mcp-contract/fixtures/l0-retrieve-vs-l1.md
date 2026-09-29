# Fixture — L0 `edgequake_retrieve` vs L1 `eq_retrieve`

Synthetic side-by-side (no live corpus). Shows why L0 is non-conformant and what L1 MUST look like for the same user intent: *explain Action Fusion in sol_pi*.

---

## L0 today (non-conformant shape)

`tools/call` name: `edgequake_retrieve`

**Arguments (typical):**

```json
{
  "query": "Action Fusion",
  "mode": "mix",
  "include_subgraph": true,
  "content_granularity": "agent"
}
```

**CallToolResult (abridged):**

```json
{
  "content": [
    {
      "type": "text",
      "text": "{\"retrieval_id\":\"ret_aaa\",\"bundle\":{\"subgraph\":{\"entities\":[{\"id\":\"ent:ACTION_FUSION\",\"description\":\"<2000 chars of figure OCR and method prose>\"},{\"id\":\"ent:DRAWING_12\",\"description\":\"<page-long vision dump>\"}],\"relationships\":[{\"relation_type\":\"RELATED_TO\"}]},\"chunks\":[{\"id\":\"doc-sol-chunk-7\",\"content\":\"<full page>\"},{\"id\":\"doc-ten-chunk-3\",\"content\":\"<bleed from other PDF>\"}],\"documents\":[…]},\"truncation\":{\"is_truncated\":false,\"dropped\":{\"chunks\":0,\"entities\":0,\"relationships\":0}}}"
    }
  ],
  "structuredContent": {
    "retrieval_id": "ret_aaa",
    "bundle": {
      "subgraph": { "entities": ["… dozens …"], "relationships": ["… RELATED_TO …"] },
      "chunks": ["… full texts, possibly cross-doc …"],
      "documents": ["…"]
    }
  }
}
```

**Defects visible in this dump:**

| Defect | Manifestation |
|--------|----------------|
| No hits | Single opaque `retrieval_id` + megabundle |
| Dual JSON | `content[0].text` clones `structuredContent` |
| Subgraph on | Figure / DRAWING hubs in default path |
| Silent bleed | `doc-ten-chunk-3` with no `cross_document` / scope |
| Fake truncation | `dropped` all zeros even when host must cut |
| No list tool | Agent never saw the four PDF filenames |

---

## L1 target (conformant shape)

### Step 1 — `eq_document_list`

```json
{
  "ok": true,
  "view": "document_list",
  "budget_used": "standard",
  "documents": [
    { "id": "doc-sol", "title": "SoL-Pi", "file_name": "sol_pi_2609.20519v1.pdf", "status": "completed" },
    { "id": "doc-ten", "title": "Ten", "file_name": "ten_2609.18461v1.pdf", "status": "completed" },
    { "id": "doc-c", "title": "C", "file_name": "c.pdf", "status": "completed" },
    { "id": "doc-d", "title": "D", "file_name": "d.pdf", "status": "completed" }
  ],
  "truncation": { "truncated": false }
}
```

**Summary text (≤2 KiB):**

```text
ok document_list budget=standard count=4
- doc-sol sol_pi_2609.20519v1.pdf completed
- doc-ten ten_2609.18461v1.pdf completed
- doc-c c.pdf completed
- doc-d d.pdf completed
```

### Step 2 — `eq_search` scoped

```json
{
  "query": "Action Fusion",
  "mode": "local",
  "document_ids": ["doc-sol"],
  "budget": "standard",
  "limit": 8
}
```

**structuredContent:**

```json
{
  "ok": true,
  "view": "search",
  "budget_used": "standard",
  "retrieval_id": "ret_bbb",
  "expires_at": "2026-09-29T14:30:00Z",
  "mode_used": "local",
  "mode_reason": "requested local",
  "cross_document": false,
  "score_type": "unit_interval",
  "hits": [
    {
      "kind": "chunk",
      "id": "doc-sol-chunk-7",
      "document_id": "doc-sol",
      "title": "Action Fusion",
      "snippet": "Merges edit/write and run into one provider request…",
      "score": 0.91
    },
    {
      "kind": "entity",
      "id": "ent:ws:action_fusion",
      "document_id": "doc-sol",
      "title": "Action Fusion",
      "snippet": "Method — merges edit/write + run",
      "score": 0.88
    }
  ],
  "documents_considered": ["doc-sol"],
  "truncation": { "truncated": false },
  "stats": { "total_ms": 180, "cached": false }
}
```

### Step 3 — `eq_fetch` incremental

```json
{
  "retrieval_id": "ret_bbb",
  "ids": ["doc-sol-chunk-7"],
  "view": "chunks",
  "budget": "standard",
  "include_subgraph": false
}
```

**structuredContent (abridged):**

```json
{
  "ok": true,
  "view": "chunks",
  "budget_used": "standard",
  "retrieval_id": "ret_bbb",
  "chunks": [
    {
      "id": "doc-sol-chunk-7",
      "document_id": "doc-sol",
      "file_name": "sol_pi_2609.20519v1.pdf",
      "page": 5,
      "score": 0.91,
      "text": "… budget-capped chunk body …"
    }
  ],
  "entities": [],
  "relationships": [],
  "truncation": { "truncated": false },
  "lineage_resources": ["eq://ws/chunks/doc-sol-chunk-7"]
}
```

**Summary text:**

```text
ok chunks budget=standard retrieval=ret_bbb
chunk doc-sol-chunk-7 score=0.91 page=5 sol_pi_2609.20519v1.pdf
```

### Optional sugar — `eq_retrieve`

Same scope/budget; returns **both** `hits` (from search) and `chunks` (from fetch view). Structured JSON ≤ 24 KiB; if more chunks exist, `truncation.truncated=true` with `next_cursor` and omitted counts — never mid-object gateway slice.

---

## Size intuition

| Artifact | L0 order of magnitude | L1 order of magnitude |
|----------|----------------------|------------------------|
| `content[0].text` | 50–300 KiB JSON clone | ≤ 2 KiB summary |
| `structuredContent` | Uncapped bundle | ≤ 24 KiB at `standard` |
| Search rows | 1 session | ≤ `limit` hits |
| Cross-doc chunks | Silent | Forbidden under `document_ids` |
