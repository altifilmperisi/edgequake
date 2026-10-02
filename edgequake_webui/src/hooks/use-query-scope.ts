"use client";

/**
 * Document scope for the query composer: ids + display titles, persisted in
 * query settings. Single owner for `@` mentions, the picker and chip removal.
 */
import { addScopedId, removeScopedId } from "@/lib/query/mention";
import { useSettingsStore } from "@/stores/use-settings-store";
import { useCallback } from "react";
import { useShallow } from "zustand/react/shallow";

const NO_IDS: string[] = [];
const NO_TITLES: Record<string, string> = {};

export function useQueryScope() {
  const { ids, titles } = useSettingsStore(
    useShallow((s) => ({
      ids: s.querySettings.scopedDocumentIds ?? NO_IDS,
      titles: s.querySettings.scopedDocumentTitles ?? NO_TITLES,
    })),
  );
  const setQuerySettings = useSettingsStore((s) => s.setQuerySettings);

  const addDocument = useCallback(
    (doc: { id: string; title: string }) => {
      const state = useSettingsStore.getState().querySettings;
      setQuerySettings({
        scopedDocumentIds: addScopedId(state.scopedDocumentIds ?? [], doc.id),
        scopedDocumentTitles: {
          ...(state.scopedDocumentTitles ?? {}),
          [doc.id]: doc.title,
        },
      });
    },
    [setQuerySettings],
  );

  const removeDocument = useCallback(
    (id: string) => {
      const state = useSettingsStore.getState().querySettings;
      const next = removeScopedId(
        state.scopedDocumentIds ?? [],
        state.scopedDocumentTitles ?? {},
        id,
      );
      setQuerySettings({
        scopedDocumentIds: next.ids,
        scopedDocumentTitles: next.titles,
      });
    },
    [setQuerySettings],
  );

  /** Replace the whole selection (picker); titles of dropped ids are pruned. */
  const setDocumentIds = useCallback(
    (nextIds: string[]) => {
      const state = useSettingsStore.getState().querySettings;
      const keep = new Set(nextIds);
      const nextTitles = Object.fromEntries(
        Object.entries(state.scopedDocumentTitles ?? {}).filter(([id]) =>
          keep.has(id),
        ),
      );
      setQuerySettings({
        scopedDocumentIds: nextIds,
        scopedDocumentTitles: nextTitles,
      });
    },
    [setQuerySettings],
  );

  const rememberTitle = useCallback(
    (id: string, title: string) => {
      const state = useSettingsStore.getState().querySettings;
      setQuerySettings({
        scopedDocumentTitles: {
          ...(state.scopedDocumentTitles ?? {}),
          [id]: title,
        },
      });
    },
    [setQuerySettings],
  );

  return { ids, titles, addDocument, removeDocument, setDocumentIds, rememberTitle };
}
