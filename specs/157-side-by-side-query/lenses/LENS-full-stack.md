# Lens — Full Stack Developer

Parent: [README](../README.md) · Architecture: [05](../05-frontend-architecture.md) ·
Plan: [09](../09-implementation-plan.md)

## Ownership

End-to-end wiring: Query UI ↔ companion store ↔ document APIs ↔ optional chat
contract hardening. Prefer frontend-only until Graph reload quality forces W5.

## Critical paths

```text
  Citation chip --> useOpenSource --> store + URL --> DocumentSourceView
                                                      --> GET document / PDF

  Show on graph --> answer-graph.ts --> EmbeddedAnswerGraph
                                    --> optional /graph escape

  Flag OFF --> legacy router.push (must keep compiling)
```

## Implementation hazards

| Hazard | Mitigation |
|--------|------------|
| Remount PDF on resize | Controlled `currentPage`; stable React keys |
| Clobber `useGraphStore` | Isolated engine; e2e isolation gate |
| Duplicate stores | Merge in W3 before embedding |
| CLS / overflow | `min-h-0` flex chain (SPEC-100) |
| History + companion fight | Single layout owner in `QueryInterface` |

## DRY / SOLID checklist for PRs

- [ ] No new `router.push('/documents')` from Query citation paths when flag on
- [ ] No copy-paste of page sync logic from `documents/[id]/page.tsx`
- [ ] Pure functions unit-tested before UI
- [ ] Feature flag guards all new UX

## Test commands

```bash
cd edgequake_webui
bun run test                 # vitest
bun run typecheck
bun run lint
bun run test:e2e:spec157     # once script added
bun run test:locale-parity
```

Optional:

```bash
cargo test -p edgequake-api --test spec027_api_contract
```

## PR slicing

1. W0 extract (documents parity) — separate PR  
2. W1 shell + URL  
3. W2 source intents  
4. W3 graph + store merge  
5. W4 / W5 follow-ups  

Cross-ref: [02-surfaces](../02-surfaces.md) · [08-edge-cases](../08-edge-cases.md).
