# 11 — Migration and risks

Parent: [README](README.md) · Plan: [09](09-implementation-plan.md)

## Compatibility window

| Item | Policy |
|------|--------|
| Aliases `edgequake_search\|fetch\|retrieve` | One minor version; enforce `budget=standard`, subgraph off |
| After window | Remove aliases or return explicit deprecation error |
| SPEC-028 e2e | Update gradually to prefer `eq_*` while aliases keep CI green |
| TS tool names | Hard cut in Phase D with major `@edgequake/mcp-server` bump (e.g. 0.2 → 0.3 or 1.0) |

Hosts with allowlists (`allowed_tools`) MUST add `eq_*` names before aliases disappear. Update examples: `specs/028/.../mcp/cursor.example.json`, `grok.example.json`, `claude-cowork.example.md`.

---

## Protocol deviation: text channel

**Risk:** Hosts that only parse `content[0].text` as JSON will break when text becomes a summary.

**Mitigation:**

1. Document the override in README and `instructions`.
2. Keep `structuredContent` complete and schema-valid (hosts on MCP 2026-07-28 prefer it).
3. Summary still includes retrieval ids and top hit ids so text-only hosts can call fetch.
4. Measure Grok / Cursor behavior in Phase A dogfood.

---

## TTL blast radius

Raising retrieval cache from 15m → 1h affects REST `GET /api/v1/query/context/{id}`.

| Option | Pros | Cons |
|--------|------|------|
| Raise to 1h | Matches contract recommendation | Longer memory of bundles; more RAM |
| Keep 15m; echo `expires_at` | No REST surprise | Agents must re-search more often |

**Normative:** Echoed `expires_at` is authoritative. Choose option in Phase B and document in CHANGELOG.

---

## Dual implementation drift

**Risk:** Stdio package reimplements budgets differently from Rust gateway.

**Mitigation:** Phase D bridge-only architecture. Ban local ranking in `mcp/src/tools` after rewrite. CI: same conformance fixtures against `/mcp`.

---

## Schema / OpenAPI drift

**Risk:** `tools.rs` hand-written JSON drifts from `specs/152/schemas`.

**Mitigation:** Conformance test loads schemas from disk; optional codegen later. Update or replace `specs/028/.../mcp/tool-schemas.json` pointer in Phase D.

---

## Graph filter false negatives

**Risk:** Hiding all `Artifact` entities hides legitimate non-figure artifacts.

**Mitigation:** `include_artifacts: true` escape hatch; map only `DRAWING` / figure-OCR labels by default if product taxonomy distinguishes them. Document in tool description.

---

## Scope auto-detect

**Risk:** Filename auto-scope misfires on short tokens.

**Mitigation:** Phase B: only auto-scope when query contains a `file_name` substring matching exactly one document; else set `cross_document` and optionally `eq/scope_required` under a future strict mode. Do not block L1 on strict mode.

---

## Write tools on query connectors

**Risk:** Accidental delete via memory tools on a public Grok connector.

**Mitigation:** Query profile omits tools at `tools/list`; scopes omit `edgequake:write`; instructions say query-only.

---

## Honest risks (what this spec does not fix)

1. Extraction quality / `RELATED_TO` soup — projection filters only.
2. Hosts that ignore `structuredContent` and demand megabyte text — summary is intentional lossy.
3. Multi-workspace agents without header discipline — still footgun; instructions + claim enforcement help.
4. Live Keycloak / client quirks from SPEC-028 deferred ECs — still operational work.

---

## Rollback

Phase A/B changes are additive (new tools + projection). Rollback path:

1. Feature flag `EDGEQUAKE_MCP_EQ_CONTRACT=0` to restore L0 `call_tool_result` + three tools only (optional implementer choice).
2. Or revert the MCP gateway commits; REST query-context unchanged if projection-only.

Prefer flag during Phase A dogfood; remove flag once L1 is default.
