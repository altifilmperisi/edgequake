# Lens — Product Owner

Parent: [README](../README.md) · Product: [03](../03-product-spec.md) ·
Plan: [09](../09-implementation-plan.md)

## Job to be done

Ship **verify-in-place** on Query: analysts check citations and answer entities
without abandoning the conversation. This is the adoption unlock for RAG trust.

## Outcomes by wave

| Wave | User-visible outcome |
|------|----------------------|
| W0 | No user change; safer extract of shared viewer |
| W1 | Resizable companion shell; URL shareable empty pane chrome |
| W2 | Citation → PDF/text beside chat (hero moment) |
| W3 | Show on graph → subgraph beside chat |
| W4 | Scope + quote-to-ask accelerate follow-ups |
| W5 | Hardened, accessible, localised |
| W6 | Documented, releasable |

## Prioritisation rules

1. Source pane (W2) before Graph pane (W3) — citation leave-rate is the pain.
2. Never remove full-page escape hatches.
3. No three-pane scope creep in v1.
4. Flag off = zero behaviour change for cautious rollouts.

## Acceptance demo script

1. Ask a question with citations on `/query`.
2. Click `[1]` — PDF opens beside chat on the right page; ask a follow-up.
3. Click “Show on graph” — entities light; chat still visible.
4. Copy URL, open new tab — pane restores.
5. Cmd-click a citation — full document page in new tab.
6. Shrink window — sheet works; Esc returns focus.

## Risks / comms

| Risk | Comms |
|------|-------|
| Narrow laptops feel cramped | Auto-rail history; document in release notes |
| Graph isolation bug | Hold flag until EC-157-19 green |
| Users expect full Studio | Copy: “Answer graph” + Studio link |

## KPIs

- % citation clicks that stay on `/query` (target ↑)
- Support tickets “lost my chat when I opened source” → ↓
- axe serious = 0; locale parity = 100%

Cross-ref: [00-why](../00-why.md) · [11-cross-ref](../11-cross-ref.md).
