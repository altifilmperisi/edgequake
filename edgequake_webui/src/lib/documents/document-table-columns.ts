/**
 * Documents inventory table column widths (SPEC-099 / table-fixed layout).
 *
 * Shared by header + body `<colgroup>` so columns stay aligned.
 * Title must claim an explicit % — an empty `<col />` collapses under
 * pressure and nowrap cells spill into Status (overlapping headers/badges).
 */

export const DOCUMENT_TABLE_COL_PERCENTS = {
  default: {
    checkbox: '3%',
    title: '31%',
    status: '15%',
    entities: '11%',
    created: '14%',
    updated: '14%',
    actions: '12%',
  },
  withCost: {
    checkbox: '3%',
    title: '23%',
    status: '15%',
    entities: '10%',
    cost: '8%',
    created: '13%',
    updated: '13%',
    actions: '15%',
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
