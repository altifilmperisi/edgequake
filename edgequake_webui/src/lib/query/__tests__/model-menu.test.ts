import { describe, expect, it } from "vitest";
import type { LlmModelItem } from "@/lib/api/models";
import {
  buildModelMenu,
  flattenModelMenu,
  modelSupportsStreaming,
  resolveStreamMode,
} from "../model-menu";

function model(
  provider: string,
  name: string,
  over: Partial<LlmModelItem> & {
    streaming?: boolean;
  } = {},
): LlmModelItem {
  const { streaming = true, ...rest } = over;
  return {
    provider,
    provider_display_name: provider.toUpperCase(),
    name,
    display_name: name,
    model_type: "llm",
    description: "",
    deprecated: false,
    capabilities: {
      context_length: 8000,
      max_output_tokens: 1000,
      supports_vision: false,
      supports_function_calling: false,
      supports_json_mode: false,
      supports_streaming: streaming,
      supports_system_message: true,
      embedding_dimension: 0,
    },
    cost: {
      input_per_1k: 0,
      output_per_1k: 0,
      embedding_per_1k: 0,
      image_per_unit: 0,
    },
    tags: [],
    ...rest,
  };
}

const MODELS = [
  model("openai", "gpt-5-nano"),
  model("openai", "gpt-4o-mini", { deprecated: true }),
  model("ollama", "gemma3", { streaming: false }),
  model("ollama", "qwen", { available: false }),
];

describe("buildModelMenu", () => {
  it("hides deprecated/unavailable and groups by provider", () => {
    const groups = buildModelMenu(MODELS);
    expect(groups.map((g) => g.provider)).toEqual(["ollama", "openai"]);
    expect(flattenModelMenu(groups).map((i) => i.name)).toEqual([
      "gemma3",
      "gpt-5-nano",
    ]);
  });

  it("keeps a deprecated current selection, sorted first", () => {
    const groups = buildModelMenu(MODELS, { currentFullId: "openai/gpt-4o-mini" });
    expect(groups[0]!.provider).toBe("openai");
    expect(groups[0]!.items[0]!.name).toBe("gpt-4o-mini");
  });

  it("filters by multi-token query across provider + name", () => {
    expect(
      flattenModelMenu(buildModelMenu(MODELS, { query: "openai nano" })).map(
        (i) => i.name,
      ),
    ).toEqual(["gpt-5-nano"]);
    expect(buildModelMenu(MODELS, { query: "zzz" })).toEqual([]);
  });
});

describe("streaming resolution", () => {
  it("reads capability for explicit selection", () => {
    expect(
      modelSupportsStreaming(MODELS, { provider: "ollama", model: "gemma3" }),
    ).toBe(false);
    expect(
      modelSupportsStreaming(MODELS, { provider: "openai", model: "gpt-5-nano" }),
    ).toBe(true);
  });

  it("falls back to the server default model", () => {
    expect(
      modelSupportsStreaming(MODELS, {
        defaultProvider: "ollama",
        defaultModel: "gemma3",
      }),
    ).toBe(false);
  });

  it("assumes streaming when unknown", () => {
    expect(modelSupportsStreaming(undefined, {})).toBe(true);
    expect(
      modelSupportsStreaming(MODELS, { provider: "x", model: "y" }),
    ).toBe(true);
  });

  it("resolveStreamMode needs both", () => {
    expect(resolveStreamMode(true, true)).toBe(true);
    expect(resolveStreamMode(true, false)).toBe(false);
    expect(resolveStreamMode(false, true)).toBe(false);
  });
});
