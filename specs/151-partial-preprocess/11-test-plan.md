# 11 — Test plan

Parent: [README](README.md) · Edge cases: [09](09-edge-cases.md) · UI: [05](05-ui-spec.md)

## Layers

| Layer | Location | Focus |
|-------|----------|-------|
| Unit (Rust) | `edgequake-pdf`, `edgequake-pipeline`, `edgequake-api` | splice, closure, reuse, planner |
| Unit (TS) | `src/lib/documents/__tests__` | page-range, stages, health agg |
| Integration | `edgequake-api` tests | health derive, admission 409 |
| E2E mocked | `edgequake_webui/e2e/spec151-partial-reprocess.spec.ts` | UX flows |
| E2E live (tagged) | same file `@live` | optional real stack |

## Screenshot protocol

1. Helper `spec151Screenshot(name)` → `specs/151-partial-preprocess/e2e/screenshots/`.
2. Capture: strip states, selection, dialog stages, impact, errors, progress, done.
3. Agent **reads each PNG** with the Read tool (visual inspect).
4. Record findings in [e2e/screenshots/ANALYSIS.md](e2e/screenshots/ANALYSIS.md).
5. Fix UI defects; re-capture until ANALYSIS is clean.

## Named tests

See [09](09-edge-cases.md) Test column (`T-151-*`).

## Gates

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p edgequake-pdf --lib
cargo test -p edgequake-pipeline --lib
cargo test -p edgequake-storage --lib
cargo test -p edgequake-api --lib
cd edgequake_webui && bun run test && bunx tsc --noEmit
cd edgequake_webui && pnpm exec playwright test e2e/spec151-partial-reprocess.spec.ts
```
