/**
 * Documents inventory table column widths (SPEC-099 / table-fixed layout).
 *
 * Shared by header + body `<colgroup>` so columns stay aligned.
 * Title must claim an explicit % — an empty `<col />` collapses under
 * pressure and nowrap cells spill into Status (overlapping headers/badges).
 */

export const DOCUMENT_TABLE_COL_PERCENTS = {
  default: {
    // Actions is one overflow button: keep it narrow so Title/Updated headers
    // are not truncated ("Last…") at laptop widths.
    checkbox: '3%',
    title: '33%',
    status: '15%',
    entities: '11%',
    created: '15%',
    updated: '14%',
    actions: '9%',
  },
  withCost: {
    checkbox: '3%',
    title: '25%',
    status: '15%',
    entities: '10%',
    cost: '8%',
    created: '13%',
    updated: '13%',
    actions: '13%',
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
 * Narrow inventory pane (container < 42rem): "Last Updated" is hidden, so the
 * remaining columns are re-weighted (sum = 100) via important container-query
 * utilities that override the inline `<col>` widths. Literal class strings keep
 * Tailwind's scanner happy. Only applied to the default (no-cost) layout.
 */
export const DOCUMENT_TABLE_NARROW_COL_CLASSES = {
  checkbox: '@max-2xl:w-[4%]!',
  title: '@max-2xl:w-[26%]!',
  status: '@max-2xl:w-[19%]!',
  entities: '@max-2xl:w-[17%]!',
  created: '@max-2xl:w-[25%]!',
  actions: '@max-2xl:w-[9%]!',
} as const;
