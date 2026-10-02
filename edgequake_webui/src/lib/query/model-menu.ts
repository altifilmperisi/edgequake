/**
 * Pure helpers for the composer model picker (SPEC-155 W7Q).
 * No React — unit-testable. UI state lives in the component / settings store.
 */
import type { LlmModelItem } from "@/lib/api/models";

export interface ModelMenuItem {
  fullId: string;
  provider: string;
  name: string;
  displayName: string;
  supportsStreaming: boolean;
  supportsVision: boolean;
  supportsThinking: boolean;
  deprecated: boolean;
  available: boolean;
}

export interface ModelMenuGroup {
  provider: string;
  displayName: string;
  items: ModelMenuItem[];
}

export function formatFullId(provider: string, model: string): string {
  return `${provider}/${model}`;
}

function toItem(m: LlmModelItem): ModelMenuItem {
  return {
    fullId: formatFullId(m.provider, m.name),
    provider: m.provider,
    name: m.name,
    displayName: m.display_name || m.name,
    supportsStreaming: m.capabilities?.supports_streaming !== false,
    supportsVision: Boolean(m.capabilities?.supports_vision),
    supportsThinking: Boolean(m.capabilities?.supports_thinking),
    deprecated: Boolean(m.deprecated),
    available: m.available !== false,
  };
}

function matches(item: ModelMenuItem, providerName: string, q: string): boolean {
  if (!q) return true;
  const hay = `${item.displayName} ${item.name} ${providerName}`.toLowerCase();
  return q
    .toLowerCase()
    .split(/\s+/)
    .filter(Boolean)
    .every((token) => hay.includes(token));
}

/**
 * Group models by provider, filtered by `query`. Deprecated / unavailable models
 * are hidden unless they are the current selection. The current model sorts first.
 */
export function buildModelMenu(
  models: ReadonlyArray<LlmModelItem>,
  opts: { query?: string; currentFullId?: string } = {},
): ModelMenuGroup[] {
  const query = (opts.query ?? "").trim();
  const groups = new Map<string, ModelMenuGroup>();

  for (const m of models) {
    const item = toItem(m);
    const isCurrent = item.fullId === opts.currentFullId;
    if (!isCurrent && (item.deprecated || !item.available)) continue;
    if (!matches(item, m.provider_display_name, query)) continue;

    let group = groups.get(m.provider);
    if (!group) {
      group = {
        provider: m.provider,
        displayName: m.provider_display_name || m.provider,
        items: [],
      };
      groups.set(m.provider, group);
    }
    group.items.push(item);
  }

  const out = [...groups.values()];
  for (const g of out) {
    g.items.sort((a, b) => {
      if (a.fullId === opts.currentFullId) return -1;
      if (b.fullId === opts.currentFullId) return 1;
      return a.displayName.localeCompare(b.displayName);
    });
  }
  out.sort((a, b) => {
    const aCur = a.items.some((i) => i.fullId === opts.currentFullId);
    const bCur = b.items.some((i) => i.fullId === opts.currentFullId);
    if (aCur !== bCur) return aCur ? -1 : 1;
    return a.displayName.localeCompare(b.displayName);
  });
  return out;
}

/** Flat list in render order — used for ↑/↓ navigation. */
export function flattenModelMenu(groups: ReadonlyArray<ModelMenuGroup>): ModelMenuItem[] {
  return groups.flatMap((g) => g.items);
}

/**
 * Does the effective model support SSE streaming?
 * `undefined` selection → server default model. Unknown model → assume yes.
 */
export function modelSupportsStreaming(
  models: ReadonlyArray<LlmModelItem> | undefined,
  selection: {
    provider?: string;
    model?: string;
    defaultProvider?: string;
    defaultModel?: string;
  },
): boolean {
  if (!models || models.length === 0) return true;
  const provider = selection.provider ?? selection.defaultProvider;
  const model = selection.model ?? selection.defaultModel;
  if (!provider || !model) return true;
  const hit = models.find((m) => m.provider === provider && m.name === model);
  if (!hit) return true;
  return hit.capabilities?.supports_streaming !== false;
}

/** The user's stream preference, downgraded when the model cannot stream. */
export function resolveStreamMode(
  userWantsStream: boolean,
  modelStreams: boolean,
): boolean {
  return userWantsStream && modelStreams;
}

/** Short label for the trigger: "gpt-5-nano" or "Default". */
export function modelTriggerLabel(
  selection: { provider?: string; model?: string },
  fallback: string,
): string {
  return selection.model ? selection.model : fallback;
}
