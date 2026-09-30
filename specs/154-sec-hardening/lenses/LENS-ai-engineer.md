# Lens — AI Engineer

Parent: [README](../README.md) · SPEC-152 §08 · MCP scopes · Waves 1–3

## Mental model

Agent credentials are **capability tokens**, not “the user with a chat UI.”

```text
  Host (Claude / Codex / Cursor / Grok)
           |
           |  Bearer MCP access token
           |  aud=https://host/mcp
           |  scope=edgequake:read edgequake:query
           v
  EdgeQuake MCP RS
           |
           +-- tools/list, eq_fetch     need :read
           +-- eq_search, eq_retrieve   need :query
           +-- eq_ingest / delete       need :write + confirm
```

## Rules for agent-facing design

1. **Least privilege by default** — connectors ship read+query; write is opt-in
   (SPEC-152 query profile; F-154-03 fix).
2. **Untrusted corpus** — chunk text is never treated as server instructions
   (SPEC-152 untrustedContentHint direction).
3. **Workspace is a security boundary** — `enforce_workspace_claim` remains;
   Wave 2 membership bind backs it.
4. **Do not smuggle LLM provider keys** through the model — elicitation/URL-mode
   only for external auth.
5. **Confirm destructive tools** — `confirm: true` or `eq/confirm_required`
   (SPEC-152).

## Host compatibility

| Host | Auth mode | SPEC-154 impact |
|------|-----------|-----------------|
| Claude / Codex | OAuth PKCE | None if Waves keep PRM/AS |
| Cursor remote | DCR or API key | Wave 3 may require scoped key |
| Grok | Static Bearer | Wave 3 scopes on key |
| Notion-style OAuth-only | OAuth path | Must keep working (EC-MCP-47) |

## Prompt / tool hygiene

- Prefer `eq_search` → `eq_fetch` escalation; avoid dumping full docs into context
  without need (also a data-leak concern).
- Log tool name + workspace + retrieval_id; **not** full chunk bodies (SPEC-152 audit).
- Rate limits on `/mcp` protect shared tenants from agent storms (SPEC-153 peer).

## Testing as an AI engineer

- Conformance: `spec028_mcp_oauth_e2e` + SPEC-154 audience/scope gates.
- Manual: reconnect Claude after Wave 3 with a read-only key; verify write tools
  absent or failing closed.
- Never paste MCP access tokens into REST curl “for convenience” — Wave 1 will
  reject that path by design.

## Sign-off

- [ ] Read-only agent cannot mutate graph/docs
- [ ] Write agent requires explicit scope + confirm
- [ ] MCP JWT cannot call REST admin APIs
- [ ] Instructions resources do not embed secrets
