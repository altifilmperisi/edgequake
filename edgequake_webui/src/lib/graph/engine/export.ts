import type Sigma from "sigma";
import { downloadAsPNG } from "@sigma/export-image";
import { resolveGraphTheme } from "./theme";

export interface ExportImageOptions {
  format?: "png" | "svg";
  pixelRatio?: number;
  fileName?: string;
  backgroundColor?: string;
}

/**
 * PNG via @sigma/export-image (all layers). Never grab the first canvas (G04).
 * SVG remains a structured stub until a dedicated SVG exporter lands.
 */
export async function exportGraphImage(
  sigma: Sigma,
  options: ExportImageOptions = {},
): Promise<{ ok: boolean; format: "png" | "svg"; reason?: string }> {
  const format = options.format ?? "png";
  if (format === "svg") {
    return { ok: false, format, reason: "svg_not_implemented" };
  }

  const theme = resolveGraphTheme();
  const backgroundColor =
    options.backgroundColor ?? theme.canvasBg ?? "#ffffff";

  await downloadAsPNG(sigma, {
    fileName: options.fileName ?? `edgequake-graph-${Date.now()}`,
    backgroundColor,
  });

  return { ok: true, format: "png" };
}
