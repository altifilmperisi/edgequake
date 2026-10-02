/**
 * Documents inventory table column widths (SPEC-099 / table-fixed layout).
 *
 * Shared by header + body `<colgroup>` so columns stay aligned.
 * Title must claim an explicit % — an empty `<col />` collapses under
 * pressure and nowrap cells spill into Status (overlapping headers/badges).
 *
 * Progressive disclosure (container queries on the inventory pane):
 *   ≥ 2xl (~42rem): all columns
 *   < 2xl: hide Updated; reweight
 *   < xl  (~36rem): hide Created too
 *   < lg  (~32rem): hide Entities + Cost; Title + Status + Actions only
 */

export const DOCUMENT_TABLE_COL_PERCENTS = {
  default: {
    checkbox: "3%",
    title: "34%",
    status: "18%",
    entities: "10%",
    created: "14%",
    updated: "13%",
    actions: "8%",
  },
  withCost: {
    checkbox: "3%",
    title: "28%",
    status: "16%",
    entities: "9%",
    cost: "8%",
    created: "12%",
    updated: "12%",
    actions: "12%",
  },
} as const;

/** Sum of column percents — must be 100 for both layouts. */
export function documentTableColPercentSum(showCostColumn: boolean): number {
  const cols = showCostColumn
    ? DOCUMENT_TABLE_COL_PERCENTS.withCost
    : DOCUMENT_TABLE_COL_PERCENTS.default;
  return Object.values(cols).reduce(
    (sum, value) => sum + Number.parseFloat(value),
    0,
  );
}

/**
 * Container-query width overrides when secondary columns are hidden.
 * Applied to BOTH default and withCost layouts (cost itself hides below lg).
 */
export const DOCUMENT_TABLE_NARROW_COL_CLASSES = {
  checkbox: "@max-2xl:w-[4%]! @max-lg:w-[5%]!",
  title: "@max-2xl:w-[38%]! @max-xl:w-[44%]! @max-lg:w-[52%]!",
  status: "@max-2xl:w-[22%]! @max-xl:w-[28%]! @max-lg:w-[34%]!",
  entities: "@max-2xl:w-[12%]! @max-lg:hidden",
  cost: "@max-lg:hidden",
  created: "@max-2xl:w-[16%]! @max-xl:hidden",
  updated: "@max-2xl:hidden",
  actions: "@max-2xl:w-[8%]! @max-lg:w-[9%]!",
} as const;

/** Hide entire column below this container max (match row cells). */
export const DOCUMENT_TABLE_COL_HIDE = {
  entities: "@max-lg:hidden",
  cost: "@max-lg:hidden",
  created: "@max-xl:hidden",
  updated: "@max-2xl:hidden",
} as const;
