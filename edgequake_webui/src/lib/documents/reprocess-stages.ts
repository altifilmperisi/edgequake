/** SPEC-151 — stage closure mirror of backend LAW-151-3. */

export type ReprocessPageStage = "parse" | "figures" | "entities";

export function stageClosure(stage: ReprocessPageStage): ReprocessPageStage[] {
  switch (stage) {
    case "parse":
      return ["parse", "figures", "entities"];
    case "figures":
      return ["figures", "entities"];
    case "entities":
      return ["entities"];
  }
}

export function effectiveStages(requested: ReprocessPageStage[]): ReprocessPageStage[] {
  const set = new Set<ReprocessPageStage>();
  for (const s of requested) {
    for (const x of stageClosure(s)) set.add(x);
  }
  const order: ReprocessPageStage[] = ["parse", "figures", "entities"];
  return order.filter((s) => set.has(s));
}

export function isStageLocked(
  selected: ReprocessPageStage,
  card: ReprocessPageStage,
): boolean {
  const effective = stageClosure(selected);
  if (card === selected) return false;
  return effective.includes(card) && card !== selected;
}
