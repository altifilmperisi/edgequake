---
title: "RLS superuser acceptance"
---

# RLS Superuser Acceptance — GAP-091-12 Decision Record

**Status:** ACCEPTED (recorded at SPEC-091 IW0)
**Date:** 2026-07-30
**Decision owner:** EdgeQuake maintainers

## Context

PostgreSQL Row Level Security policies exist for tenant/workspace isolation
(migrations 081, 096) and are exercised end-to-end by `e2e_postgres_rls`
(11 tests, un-ignored at SPEC-091 IW0) against a **non-superuser** `app_user`
role in CI (`postgres-tests` job of `postgres-integration.yml`).

In production, however, the EdgeQuake server connects as the database owner
(`edgequake`), which is a superuser-equivalent role for its own objects.
**PostgreSQL superusers and table owners bypass RLS unconditionally** (unless
`FORCE ROW LEVEL SECURITY` is set). RLS is therefore *inert* on the production
connection path today.

Additionally, Apache AGE stores the knowledge graph in a **single global
graph**; label-level RLS across tenants is not expressible and is explicitly
out of scope (see GAP-091-15, leak-proofed by tests in IW5).

## Decision

We **accept** that RLS is defense-in-depth only, and designate the
**application layer as the tenant/workspace isolation enforcement boundary**:

1. Fail-closed scope headers — a malformed `X-Tenant-ID` / `X-Workspace-ID`
   never wildcard-matches (SPEC-091 IW0, `middleware::ScopeHeader`,
   `isolation_context`, `task_scope`, `query_request_builder`).
2. Unconditional task scope checks (`task_scope::get_task_for_context`) —
   headerless requests resolve to the built-in default scope explicitly.
3. Relational isolation is covered by `e2e_tenant_isolation` (12 tests,
   including the malformed-header attack vector) and
   `contract_spec091_strict_scope_headers`.
4. RLS remains tested as defense-in-depth (`e2e_postgres_rls`) so a future
   non-superuser runtime role can flip it on without policy changes.

## Consequences

- Do **not** rely on RLS as the sole isolation mechanism for any new table;
  every scoped read path must also filter at the application layer.
- A future hardening wave may introduce a dedicated non-superuser runtime
  role (plus `FORCE ROW LEVEL SECURITY`) — at that point this record should
  be revisited and RLS promoted from defense-in-depth to enforcement.
- AGE-label tenant isolation remains application-layer only; cross-tenant
  graph/ANN leak tests (IW5) are the closure evidence.

## References

- `specs/091-simplify-data-layer/18-full-completeness-assessment.md` — GAP-091-12
- `edgequake/crates/edgequake-api/tests/e2e_postgres_rls.rs` — RLS suite
- `edgequake/crates/edgequake-api/tests/e2e_tenant_isolation.rs` — app-layer isolation suite
- `docs/data-layer/llm-cache-scope.md` — adjacent scope decision (GAP-091-14)
