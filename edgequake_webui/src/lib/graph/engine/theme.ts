import { getCssToken, getGraphThemeTokens } from "@/lib/theme/css-token";
import type { GraphThemeTokens } from "./types";

export function resolveGraphTheme(
  el?: Element | null,
): GraphThemeTokens {
  if (el) {
    return {
      canvasBg: getCssToken("--graph-canvas-bg", "#fafafa", el),
      edge: getCssToken("--graph-edge", "#6b7280", el),
      labelFg: getCssToken("--graph-label-fg", "#1f2937", el),
      labelBg: getCssToken("--graph-label-bg", "rgba(255,255,255,0.85)", el),
      focus: getCssToken("--graph-focus", "#3b82f6", el),
      dim: getCssToken("--graph-dim", "rgba(0,0,0,0.35)", el),
      hullStroke: getCssToken("--graph-hull-stroke", "rgba(59,130,246,0.6)", el),
      hullFill: getCssToken("--graph-hull-fill", "rgba(59,130,246,0.12)", el),
      communities: Array.from({ length: 10 }, (_, i) =>
        getCssToken(`--graph-community-${i}`, "#64748b", el),
      ),
    };
  }
  return getGraphThemeTokens();
}

export function labelColorForTheme(isDark: boolean, tokens: GraphThemeTokens): string {
  if (tokens.labelFg && tokens.labelFg !== "#1f2937") return tokens.labelFg;
  return isDark ? "#e2e8f0" : "#374151";
}
