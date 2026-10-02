/**
 * Tiny dependency-free PDF writer + Range-aware route for hermetic viewer tests.
 *
 * Every page carries a padded content stream, so the file is large enough for
 * "how many bytes did first paint transfer?" to be a meaningful assertion.
 */
import type { Page } from "@playwright/test";

export function buildMockPdf(pageCount: number, padBytesPerPage = 60_000): Buffer {
  const objects: string[] = [];
  // Layout mirrors a well-formed (linearized / object-stream) PDF: catalog,
  // page tree and every page dictionary sit together at the front, content
  // streams follow. pdf.js walks the whole page tree on open, so interleaving
  // page dicts with large streams would force it to read the entire file.
  // 1 = catalog, 2 = pages tree, 3 = font, 4..3+N = pages, 4+N.. = contents.
  const firstPageId = 4;
  const firstContentId = firstPageId + pageCount;
  const pageObjIds: number[] = [];
  for (let i = 0; i < pageCount; i += 1) pageObjIds.push(firstPageId + i);

  objects[1] = "<< /Type /Catalog /Pages 2 0 R >>";
  objects[2] = `<< /Type /Pages /Count ${pageCount} /Kids [${pageObjIds
    .map((id) => `${id} 0 R`)
    .join(" ")}] >>`;
  objects[3] = "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>";

  const filler = "% mock-padding ".padEnd(79, "x") + "\n";
  for (let i = 0; i < pageCount; i += 1) {
    const pageId = firstPageId + i;
    const contentId = firstContentId + i;
    const text = `BT /F1 36 Tf 72 700 Td (Mock page ${i + 1}) Tj ET\n`;
    const padding = filler.repeat(Math.ceil(padBytesPerPage / filler.length));
    const stream = text + padding;
    objects[pageId] =
      `<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] ` +
      `/Resources << /Font << /F1 3 0 R >> >> /Contents ${contentId} 0 R >>`;
    objects[contentId] = `<< /Length ${stream.length} >>\nstream\n${stream}endstream`;
  }

  let out = "%PDF-1.4\n";
  const offsets: number[] = [];
  for (let id = 1; id < objects.length; id += 1) {
    offsets[id] = out.length;
    out += `${id} 0 obj\n${objects[id]}\nendobj\n`;
  }
  const xrefAt = out.length;
  out += `xref\n0 ${objects.length}\n0000000000 65535 f \n`;
  for (let id = 1; id < objects.length; id += 1) {
    out += `${String(offsets[id]).padStart(10, "0")} 00000 n \n`;
  }
  out += `trailer\n<< /Size ${objects.length} /Root 1 0 R >>\nstartxref\n${xrefAt}\n%%EOF\n`;
  return Buffer.from(out, "latin1");
}

/** Bytes assumed delivered before pdf.js cancels its initial probe request. */
const PROBE_WASTE = 131_072;

export type PdfServeStats = {
  totalBytes: number;
  requests: number;
  rangeRequests: number;
  /** Body bytes actually sent to the browser. */
  bytesServed: number;
  /** Every request as `range|full` (debugging aid for failed assertions). */
  log: string[];
};

/** Serve `pdf` at the download route honouring `Range` (206 / 200 fallback). */
export async function mockRangePdfRoute(
  page: Page,
  pdf: Buffer,
  { supportRange = true }: { supportRange?: boolean } = {},
): Promise<PdfServeStats> {
  const stats: PdfServeStats = {
    totalBytes: pdf.length,
    requests: 0,
    rangeRequests: 0,
    bytesServed: 0,
    log: [],
  };
  const cors = {
    "access-control-allow-origin": "*",
    "access-control-allow-headers": "*",
    "access-control-expose-headers": "content-range, accept-ranges, content-length",
  };
  await page.route("**/api/v1/documents/pdf/*/download**", async (route) => {
    const req = route.request();
    if (req.method() === "OPTIONS") {
      await route.fulfill({ status: 204, headers: cors });
      return;
    }
    stats.requests += 1;
    const range = req.headers()["range"];
    stats.log.push(range ?? "full");
    const m = supportRange ? /^bytes=(\d+)-(\d*)$/.exec(range ?? "") : null;
    if (!m) {
      // A range-capable server's unranged probe is cancelled by pdf.js once
      // the headers show `Accept-Ranges`; model that as one chunk of waste.
      stats.bytesServed += supportRange ? Math.min(pdf.length, PROBE_WASTE) : pdf.length;
      await route.fulfill({
        status: 200,
        headers: { ...cors, "content-type": "application/pdf", "accept-ranges": supportRange ? "bytes" : "none" },
        body: pdf,
      });
      return;
    }
    stats.rangeRequests += 1;
    const start = Number(m[1]);
    const end = Math.min(m[2] === "" ? pdf.length - 1 : Number(m[2]), pdf.length - 1);
    const body = pdf.subarray(start, end + 1);
    stats.bytesServed += body.length;
    await route.fulfill({
      status: 206,
      headers: {
        ...cors,
        "content-type": "application/pdf",
        "accept-ranges": "bytes",
        "content-range": `bytes ${start}-${end}/${pdf.length}`,
      },
      body,
    });
  });
  return stats;
}
