import { describe, expect, it } from "vitest";
import {
  CLOSED_TARGET,
  decodeCompanionSearch,
  encodeCompanionSearch,
  isSameTarget,
  locationFromChunk,
  locationFromDocumentClick,
  parseLinesParam,
  targetKey,
  type CompanionTarget,
} from "../companion-pane";

const q = (s: string) => new URLSearchParams(s);

describe("companion-pane URL codec (SPEC-157 LAW-157-3)", () => {
  it("round-trips a pdf target (companion_url_roundtrip)", () => {
    const target: CompanionTarget = {
      kind: "pdf",
      messageId: null,
      source: {
        documentId: "doc-1",
        page: 3,
        chunkId: "c_9",
        startLine: 12,
        endLine: 30,
      },
    };
    const search = encodeCompanionSearch(target).toString();
    expect(search).toBe("pane=pdf&doc=doc-1&page=3&chunk=c_9&lines=12-30");
    expect(decodeCompanionSearch(q(search))).toEqual({
      ...target,
      source: { ...target.source, },
    });
  });

  it("round-trips a graph target", () => {
    const target: CompanionTarget = {
      kind: "graph",
      source: null,
      messageId: "msg-7",
    };
    const search = encodeCompanionSearch(target);
    expect(search.toString()).toBe("pane=graph&msg=msg-7");
    expect(decodeCompanionSearch(search)).toEqual(target);
  });

  it("preserves unrelated params and replaces stale companion ones", () => {
    const base = q("foo=bar&pane=graph&msg=x&page=9");
    const next = encodeCompanionSearch(
      { kind: "pdf", messageId: null, source: { documentId: "d" } },
      base,
    );
    expect(next.get("foo")).toBe("bar");
    expect(next.get("pane")).toBe("pdf");
    expect(next.get("msg")).toBeNull();
    expect(next.get("page")).toBeNull();
  });

  it("closing strips every companion param", () => {
    const next = encodeCompanionSearch(
      CLOSED_TARGET,
      q("foo=1&pane=pdf&doc=d&page=2&chunk=c&lines=1-2&msg=m"),
    );
    expect(next.toString()).toBe("foo=1");
  });

  it.each([
    ["pane=pdf", "missing doc"],
    ["pane=pdf&doc=", "empty doc"],
    ["pane=pdf&doc=../../etc/passwd", "path traversal"],
    ["pane=pdf&doc=<script>", "markup"],
    ["pane=graph", "missing msg"],
    ["pane=weird&doc=d", "unknown pane"],
    ["doc=d&page=2", "no pane"],
  ])("sanitises %s (%s) to closed", (search) => {
    expect(decodeCompanionSearch(q(search))).toEqual(CLOSED_TARGET);
  });

  it.each([
    ["page=0"],
    ["page=-4"],
    ["page=abc"],
    ["page=1.5x"],
    ["page=999999999"],
  ])("drops invalid %s but keeps the document open", (extra) => {
    const t = decodeCompanionSearch(q(`pane=pdf&doc=d&${extra}`));
    expect(t.kind).toBe("pdf");
    expect(t.source?.page === undefined || t.source.page === 1).toBe(true);
  });

  it("ignores malformed line ranges", () => {
    expect(parseLinesParam("30-12")).toBeUndefined();
    expect(parseLinesParam("a-b")).toBeUndefined();
    expect(parseLinesParam("5")).toBeUndefined();
    expect(parseLinesParam("5-5")).toEqual([5, 5]);
    const t = decodeCompanionSearch(q("pane=pdf&doc=d&lines=9-1"));
    expect(t.source?.startLine).toBeUndefined();
  });

  it("targetKey ignores display-only fields", () => {
    const a: CompanionTarget = {
      kind: "pdf",
      messageId: null,
      source: { documentId: "d", page: 2, title: "A", passage: "x" },
    };
    const b: CompanionTarget = {
      kind: "pdf",
      messageId: null,
      source: { documentId: "d", page: 2, title: "B" },
    };
    expect(targetKey(a)).toBe(targetKey(b));
    expect(isSameTarget(a, b)).toBe(true);
    expect(isSameTarget(a, CLOSED_TARGET)).toBe(false);
  });
});

describe("location builders", () => {
  it("locationFromChunk derives a title and multi-page end", () => {
    const loc = locationFromChunk({
      document_id: "d",
      chunk_id: "c",
      page_start: 2,
      page_end: 4,
      start_line: 1,
      end_line: 9,
      content: "hello",
      file_path: "/a/b/Report.final.pdf",
    });
    expect(loc).toMatchObject({
      documentId: "d",
      chunkId: "c",
      page: 2,
      pageEnd: 4,
      title: "Report.final",
      passage: "hello",
    });
  });

  it("single-page chunks have no pageEnd", () => {
    expect(
      locationFromChunk({ document_id: "d", page_start: 2, page_end: 2 })
        .pageEnd,
    ).toBeUndefined();
  });

  it("locationFromDocumentClick maps the sources-panel payload", () => {
    expect(
      locationFromDocumentClick({
        documentId: "d",
        chunkId: "c",
        chunkContent: "p",
        page: 5,
        startLine: 1,
        endLine: 2,
      }),
    ).toEqual({
      documentId: "d",
      chunkId: "c",
      passage: "p",
      page: 5,
      startLine: 1,
      endLine: 2,
    });
  });
});
