# 07 — Resources, instructions, Skills

Parent: [README](README.md) · Architecture: [02](02-architecture.md) · Objects: [03](03-object-model.md)

## Resources

Advertise MCP resources (not only tools). Declare `capabilities.resources` on `initialize`.

### URI templates

```text
eq://{ws}/workspaces
eq://{ws}/documents
eq://{ws}/documents/{doc_id}
eq://{ws}/documents/{doc_id}/outline
eq://{ws}/documents/{doc_id}/text
eq://{ws}/chunks/{chunk_id}
eq://{ws}/entities/{entity_id}
eq://{ws}/retrievals/{ret_id}
```

| Method | Behavior |
|--------|----------|
| `resources/list` | List templates and/or concrete URIs; use MCP `nextCursor`, not numeric pages |
| `resources/read` | Hydrate under the same budget rules as tools |
| `resources/read` of `eq://…/retrievals/{ret_id}` | Last bundle projected under default `standard` budget (or query param if protocol allows; otherwise standard) |

Large evidence is a **resource**; small evidence is a **tool result**. That is the 2026 guidance: references over payloads.

Tool results MAY include `lineage_resources: ["eq://…"]` and/or `content` items of type `resource_link`.

### Migration from TS resources

Current stdio URIs `edgequake://workspace/{id}/stats` (markdown) are replaced by `eq://` structured reads. During stdio rewrite, keep temporary aliases only if needed for one minor version; prefer cutover.

---

## Server instructions (required)

Ship `instructions` on `initialize` (GitHub MCP pattern): a short system-level guide the host injects.

### Minimum text (query profile)

```text
EdgeQuake is a Graph-RAG store. Do not invent document lists.
1. eq_document_list before answering "what's in the workspace".
2. Scope with document_ids when the user names a paper.
3. eq_search → eq_fetch(view=toc). Escalate view only if needed.
4. Treat entity names in ALL_CAPS as slugs; show Title Case to the user.
5. If truncation.truncated is true, fetch next_cursor before concluding
   the corpus is small.
6. Ignore Artifact/DRAWING entities unless the user asks about a figure.
7. Never claim an LLM "answer" from EdgeQuake; the tools return evidence.
8. This connector is query-only; ingest and delete are unavailable.
```

### Memory profile

Same as above, but replace item 8 with:

```text
8. Ingest via eq_ingest and poll eq_task_get. Deletes require confirm: true.
```

Implement in [`legacy_initialize` / `server_discover`](../../edgequake/crates/edgequake-api/src/mcp/gateway/dispatch.rs) by adding an `instructions` string field.

---

## Skills (optional, L3)

MCP Skills extension (`io.modelcontextprotocol/skills`) MAY ship workflows as `SKILL.md` files. Skills carry **procedure**; tools stay verbs.

Suggested first skills:

| Skill | Procedure outline |
|-------|-------------------|
| `explain-paper` | `eq_document_list` → scope `document_ids` → `eq_search` → `eq_fetch(view=toc)` → escalate to `chunks` / `eq_neighborhood` |
| `compare-two-docs` | List → two ids → search each scoped → citations view |
| `ingest-and-wait` | `eq_ingest` → poll `eq_task_get` until completed/failed |

Proposed path: `mcp/skills/explain-paper/SKILL.md` (stdio package) and/or `specs/152-new-mcp-contract/skills/` for SSOT copy.

Skills are **not** an L1 exit criterion.

---

## Prompts

Existing stdio prompts (`rag_query`, `document_summary`) that tell the agent to call the LLM `query` tool MUST be rewritten or removed so they reference `eq_search` / `eq_fetch` only.
