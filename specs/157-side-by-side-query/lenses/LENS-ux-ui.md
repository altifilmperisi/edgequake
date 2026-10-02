# Lens — UX / UI Designer

Parent: [README](../README.md) · UX: [04](../04-ux-ui-spec.md) · WHY: [00](../00-why.md)

## Design problem

Verification today feels like leaving the room. The companion must feel like
**bringing the evidence to the desk** — calm, secondary, always dismissible.

## Principles (UX)

1. Chat remains visually primary (slightly wider default ≈ 55/45).
2. Companion chrome is thin: title, tabs, close, one escape action.
3. Empty states teach the next click (“Select a citation…”).
4. Motion is short; reduced-motion users get instant state change.
5. Mobile sheet must not fight the composer — open after citation intent.

## Wireframe ownership

ASCII in [04](../04-ux-ui-spec.md) is normative for layout regions. High-fidelity
mocks (if produced) must not invent a third pane or Studio chrome.

## Accessibility review points

- [ ] Splitter matches APG window splitter
- [ ] Focus return to citation / Show on graph
- [ ] Live region announcements
- [ ] Contrast on pane header actions
- [ ] Touch targets ≥ 24px (WCAG 2.2)

## Content

Prefer human titles over UUIDs. Errors are actionable (“Retry”, “Open full
page”). Avoid blaming the user for missing page markers — explain text mode.

## Collaboration with Front

Token reuse only (`--background`, `--border`, `--primary`). No new palette.
Type floor 12px. Icons from existing lucide set (`Columns`, `Network`, `X`).

Cross-ref: LAW-157-6,7,13 · EC-157-11,12,21,24.
