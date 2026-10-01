/**
 * Resolve a CSS custom property to a concrete colour string (canvas / Sigma).
 * SPEC-155 LAW-155-1 — tokens are the single source for graph chrome.
 *
 * WHY normalise: design tokens are authored in `oklch()`, but Sigma's WebGL
 * colour parser only understands hex / rgb(a) / named colours. An unparsed
 * colour silently renders as black (or not at all), which is what turned the
 * graph edges into a heavy black hairball.
 */

/** Colours Sigma parses natively — returned untouched. */
const SIGMA_SAFE_COLOR = /^(#[0-9a-f]{3,8}|rgba?\(|[a-z]+$)/i;

const normalizedCache = new Map<string, string>();
let scratchCtx: CanvasRenderingContext2D | null | undefined;

function getScratchContext(): CanvasRenderingContext2D | null {
  if (scratchCtx !== undefined) return scratchCtx;
  try {
    const canvas = document.createElement("canvas");
    canvas.width = 1;
    canvas.height = 1;
    scratchCtx = canvas.getContext("2d", { willReadFrequently: true });
  } catch {
    scratchCtx = null;
  }
  return scratchCtx;
}

/**
 * Convert any CSS colour the browser understands (oklch, lab, color-mix, …)
 * to `#rrggbb` / `rgba()`. Returns the input when conversion is impossible
 * (SSR, jsdom without canvas) so callers never lose the original value.
 */
export function toSigmaColor(raw: string): string {
  if (!raw || SIGMA_SAFE_COLOR.test(raw)) return raw;
  const cached = normalizedCache.get(raw);
  if (cached) return cached;

  const ctx = typeof document !== "undefined" ? getScratchContext() : null;
  if (!ctx) return raw;

  try {
    ctx.clearRect(0, 0, 1, 1);
    // Invalid colours leave fillStyle unchanged; seed with a sentinel to detect it.
    ctx.fillStyle = "#010203";
    ctx.fillStyle = raw;
    if (ctx.fillStyle === "#010203") return raw; // unparseable → keep original
    ctx.fillRect(0, 0, 1, 1);
    const [r, g, b, a] = ctx.getImageData(0, 0, 1, 1).data;
    const out =
      a === 255
        ? `#${[r, g, b].map((v) => v.toString(16).padStart(2, "0")).join("")}`
        : `rgba(${r},${g},${b},${+(a / 255).toFixed(3)})`;
    normalizedCache.set(raw, out);
    return out;
  } catch {
    return raw;
  }
}

export function getCssToken(
  name: string,
  fallback = "#888888",
  el: Element | null = typeof document !== "undefined" ? document.documentElement : null,
): string {
  if (!el || typeof getComputedStyle === "undefined") return fallback;
  const raw = getComputedStyle(el).getPropertyValue(name).trim();
  return toSigmaColor(raw || fallback);
}

export function getGraphThemeTokens() {
  return {
    canvasBg: getCssToken("--graph-canvas-bg", "#fafafa"),
    edge: getCssToken("--graph-edge", "#6b7280"),
    labelFg: getCssToken("--graph-label-fg", "#1f2937"),
    labelBg: getCssToken("--graph-label-bg", "rgba(255,255,255,0.85)"),
    focus: getCssToken("--graph-focus", "#3b82f6"),
    dim: getCssToken("--graph-dim", "rgba(0,0,0,0.35)"),
    hullStroke: getCssToken("--graph-hull-stroke", "rgba(59,130,246,0.6)"),
    hullFill: getCssToken("--graph-hull-fill", "rgba(59,130,246,0.12)"),
    communities: Array.from({ length: 10 }, (_, i) =>
      getCssToken(`--graph-community-${i}`, "#64748b"),
    ),
  };
}
