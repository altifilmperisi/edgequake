# Lens — UX / UI Designer

Parent: [README](../README.md) · Design: [05](../05-design-system-spec.md) · Screens: [07](../07-screens-spec.md)

## Design north star

**Clean, elegant, minimal.** One accent, generous but purposeful whitespace,
12px floor, no decorative gradients/glow. Graph Studio is the emotional peak.

## Information architecture

```text
  Primary nav (keep groups):
    Work: Documents, Query, Graph Studio
    Observe: Pipeline, Costs
    Configure: Workspace, Settings, Knowledge(Injections), API

  Deprioritise: duplicate health widgets; “Back to Documents” on Pipeline
```

## Interaction principles

1. Progressive disclosure — advanced under “More” / tabs.
2. Honest empty — teach next action, not giant blank cards.
3. Stable spatial memory on graph — never surprise-relayout.
4. Compose while generating (Query).
5. One connection metaphor globally.

## Deliverables per wave

| Wave | UX artefacts |
|------|--------------|
| W0 | Annotate baselines |
| W1 | Token sheet + PageHeader Figma/ASCII |
| W4–W5 | Graph Studio flows (ego/path/lasso/answer) |
| W7 | Per-screen before/after |
| W8 | FR/ZH layout QA |

## Critique of current (from audit)

- Dashboard: four equal stats + empty recent = low hierarchy.
- Query: six mode pills + always-on history = tablet squeeze.
- Graph: three dense panes + icon-only toolbar without labels.
- Settings: endless card stack — needs rail nav.
- Costs: controls that lie — worse than missing controls.

## Accessibility partnership

Pair with Front on focus order, target sizes, and colour-blind graph shapes.
Prefer text+icon status over colour alone.
