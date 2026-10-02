/**
 * SPEC-157 — Query companion pane: pure state model + URL codec.
 *
 * The companion shows ONE thing beside the chat: a document source ("pdf")
 * or the answer subgraph ("graph"). Everything the user can see is
 * addressable in the query string (LAW-157-3):
 *
 *   /query?pane=pdf&doc=<id>&page=3&chunk=<cid>&lines=12-30
 *   /query?pane=graph&msg=<messageId>
 *
 * No React, no storage — unit-tested in isolation.
 */
import {
  buildDocumentCitationUrl,
  parsePageParam,
} from "@/lib/utils/document-url";

export type CompanionKind = "none" | "pdf" | "graph";

/** A place inside a document the user wants to verify. */
export type SourceLocation = {
  documentId: string;
  /** 1-indexed PDF page (first page of the cited chunk). */
  page?: number;
  /** Last page of a multi-page chunk (display only; not in the URL). */
  pageEnd?: number;
  chunkId?: string;
  startLine?: number;
  endLine?: number;
  /** Display-only fallbacks (not in the URL). */
  title?: string;
  passage?: string;
};

export type CompanionTarget = {
  kind: CompanionKind;
  source: SourceLocation | null;
  messageId: string | null;
};

export const CLOSED_TARGET: CompanionTarget = Object.freeze({
  kind: "none",
  source: null,
  messageId: null,
}) as CompanionTarget;

/** Query keys owned by the companion — everything else is preserved. */
export const COMPANION_PARAMS = [
  "pane",
  "doc",
  "page",
  "chunk",
  "lines",
  "msg",
] as const;

const ID_RE = /^[A-Za-z0-9._:-]{1,128}$/;
const MAX_PAGE = 100_000;
const MAX_LINE = 10_000_000;

type ParamsReader = Pick<URLSearchParams, "get">;

function cleanId(value: string | null | undefined): string | undefined {
  if (!value) return undefined;
  const v = value.trim();
  return ID_RE.test(v) ? v : undefined;
}

function cleanPage(value: number | undefined): number | undefined {
  if (value === undefined || !Number.isInteger(value)) return undefined;
  return value >= 1 && value <= MAX_PAGE ? value : undefined;
}

function cleanLine(value: number | undefined): number | undefined {
  if (value === undefined || !Number.isInteger(value)) return undefined;
  return value >= 0 && value <= MAX_LINE ? value : undefined;
}

/** `lines=12-30` → [12, 30]; anything malformed → undefined. */
export function parseLinesParam(
  value: string | null | undefined,
): [number, number] | undefined {
  const m = /^(\d{1,8})-(\d{1,8})$/.exec(value ?? "");
  if (!m) return undefined;
  const start = Number(m[1]);
  const end = Number(m[2]);
  return end >= start ? [start, end] : undefined;
}

/**
 * Read the companion target from a query string. Anything unrecognised or
 * incomplete sanitises to the closed target — never throws (EC-157-13).
 */
export function decodeCompanionSearch(params: ParamsReader): CompanionTarget {
  const pane = params.get("pane");
  if (pane === "pdf") {
    const documentId = cleanId(params.get("doc"));
    if (!documentId) return CLOSED_TARGET;
    const lines = parseLinesParam(params.get("lines"));
    return {
      kind: "pdf",
      messageId: null,
      source: {
        documentId,
        page: cleanPage(parsePageParam(params.get("page"))),
        chunkId: cleanId(params.get("chunk")),
        startLine: lines?.[0],
        endLine: lines?.[1],
      },
    };
  }
  if (pane === "graph") {
    const messageId = cleanId(params.get("msg"));
    if (!messageId) return CLOSED_TARGET;
    return { kind: "graph", source: null, messageId };
  }
  return CLOSED_TARGET;
}

/**
 * Write the companion target into a copy of `base`, preserving every
 * non-companion param (workspace, etc.).
 */
export function encodeCompanionSearch(
  target: CompanionTarget,
  base?: ParamsReader & { toString(): string },
): URLSearchParams {
  const next = new URLSearchParams(base?.toString() ?? "");
  for (const key of COMPANION_PARAMS) next.delete(key);

  const docId = cleanId(target.source?.documentId);
  if (target.kind === "pdf" && target.source && docId) {
    const { page, startLine, endLine } = target.source;
    const chunkId = cleanId(target.source.chunkId);
    next.set("pane", "pdf");
    next.set("doc", docId);
    const p = cleanPage(page);
    if (p) next.set("page", String(p));
    if (chunkId) next.set("chunk", chunkId);
    const s = cleanLine(startLine);
    const e = cleanLine(endLine);
    if (s !== undefined && e !== undefined && e >= s) {
      next.set("lines", `${s}-${e}`);
    }
  } else if (target.kind === "graph" && cleanId(target.messageId)) {
    next.set("pane", "graph");
    next.set("msg", target.messageId as string);
  }
  return next;
}

/** Canonical string for equality — only the URL-visible projection. */
export function targetKey(target: CompanionTarget): string {
  return encodeCompanionSearch(target).toString();
}

export function isSameTarget(a: CompanionTarget, b: CompanionTarget): boolean {
  return targetKey(a) === targetKey(b);
}

/** Minimal chunk shape shared by citation chips and source rows. */
export type ChunkLike = {
  document_id: string;
  chunk_id?: string;
  page_start?: number;
  page_end?: number;
  start_line?: number;
  end_line?: number;
  content?: string;
  title?: string;
  file_path?: string;
};

function fileTitle(path?: string): string | undefined {
  const name = path?.split("/").pop();
  return name ? name.replace(/\.[^.]+$/, "") : undefined;
}

export function locationFromChunk(chunk: ChunkLike): SourceLocation {
  return {
    documentId: chunk.document_id,
    page: chunk.page_start,
    pageEnd:
      chunk.page_end !== undefined &&
      chunk.page_start !== undefined &&
      chunk.page_end > chunk.page_start
        ? chunk.page_end
        : undefined,
    chunkId: chunk.chunk_id,
    startLine: chunk.start_line,
    endLine: chunk.end_line,
    title: chunk.title ?? fileTitle(chunk.file_path),
    passage: chunk.content,
  };
}

/** Shape emitted by the sources panel (`OnDocumentClickOptions`). */
export type DocumentClickLike = {
  documentId: string;
  chunkId?: string;
  chunkContent?: string;
  page?: number;
  startLine?: number;
  endLine?: number;
};

export function locationFromDocumentClick(
  click: DocumentClickLike,
): SourceLocation {
  return {
    documentId: click.documentId,
    page: click.page,
    chunkId: click.chunkId,
    startLine: click.startLine,
    endLine: click.endLine,
    passage: click.chunkContent,
  };
}

/** Full-page viewer deep link for a location (new tab / flag-off fallback). */
export function locationToDocumentHref(location: SourceLocation): string {
  return buildDocumentCitationUrl({
    documentId: encodeURIComponent(location.documentId),
    chunkId: location.chunkId,
    page: location.page,
    chunkContent: location.passage,
    startLine: location.startLine,
    endLine: location.endLine,
  });
}

/**
 * Parse a `/documents/{id}?page=&chunk=&start_line=&end_line=&highlight=`
 * deep link (as emitted into answers) back into a location. Null when the
 * href is not a document deep link.
 */
export function locationFromDocumentHref(href: string): SourceLocation | null {
  const [path, query = ""] = href.split("#")[0].split("?");
  const m = /^\/documents\/([^/]+)\/?$/.exec(path);
  if (!m) return null;
  let documentId: string;
  try {
    documentId = decodeURIComponent(m[1]);
  } catch {
    return null;
  }
  const params = new URLSearchParams(query);
  const num = (key: string) => {
    const n = Number.parseInt(params.get(key) ?? "", 10);
    return Number.isFinite(n) && n >= 0 ? n : undefined;
  };
  return {
    documentId,
    page: parsePageParam(params.get("page")),
    chunkId: params.get("chunk") || undefined,
    startLine: num("start_line"),
    endLine: num("end_line"),
    passage: params.get("highlight") || undefined,
  };
}
