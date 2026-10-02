import { describe, expect, it } from "vitest";
import {
  addScopedId,
  detectMention,
  initialMentionNav,
  isMentionOpen,
  reduceMentionNav,
  removeMention,
  removeScopedId,
} from "../mention";

describe("detectMention", () => {
  it("detects @ at start", () => {
    expect(detectMention("@sol", 4)).toEqual({ start: 0, end: 4, query: "sol" });
  });

  it("detects @ after whitespace", () => {
    expect(detectMention("ask about @rep", 14)).toEqual({
      start: 10,
      end: 14,
      query: "rep",
    });
  });

  it("ignores emails", () => {
    expect(detectMention("me@example.com", 14)).toBeNull();
  });

  it("allows spaces in the query (titles)", () => {
    expect(detectMention("@sol ai", 7)?.query).toBe("sol ai");
  });

  it("closes on newline", () => {
    expect(detectMention("@sol\nmore", 9)).toBeNull();
  });

  it("closes when caret is before the @", () => {
    expect(detectMention("hello @x", 3)).toBeNull();
  });

  it("caps runaway queries", () => {
    expect(detectMention(`@${"a".repeat(60)}`, 61)).toBeNull();
  });

  it("empty query right after @", () => {
    expect(detectMention("hi @", 4)?.query).toBe("");
  });
});

describe("removeMention", () => {
  it("removes token and reports caret", () => {
    const m = detectMention("see @rep now", 8)!;
    expect(removeMention("see @rep now", m)).toEqual({
      text: "see now",
      caret: 4,
    });
  });

  it("removes token at end", () => {
    const m = detectMention("see @rep", 8)!;
    expect(removeMention("see @rep", m)).toEqual({ text: "see ", caret: 4 });
  });
});

describe("scoped ids", () => {
  it("dedupes", () => {
    expect(addScopedId(["a"], "a")).toEqual(["a"]);
    expect(addScopedId(["a"], "b")).toEqual(["a", "b"]);
  });

  it("removes id + title", () => {
    expect(removeScopedId(["a", "b"], { a: "A", b: "B" }, "a")).toEqual({
      ids: ["b"],
      titles: { b: "B" },
    });
  });
});

describe("reduceMentionNav", () => {
  it("wraps", () => {
    let s = initialMentionNav;
    s = reduceMentionNav(s, { type: "move", delta: -1, count: 3 });
    expect(s.activeIndex).toBe(2);
    s = reduceMentionNav(s, { type: "move", delta: 1, count: 3 });
    expect(s.activeIndex).toBe(0);
  });

  it("set moves the highlight (hover)", () => {
    expect(
      reduceMentionNav(initialMentionNav, { type: "set", index: 2 }).activeIndex,
    ).toBe(2);
  });

  it("empty list stays at 0", () => {
    expect(
      reduceMentionNav(initialMentionNav, { type: "move", delta: 1, count: 0 })
        .activeIndex,
    ).toBe(0);
  });

  it("dismiss hides until the token changes", () => {
    const match = detectMention("@a", 2)!;
    const dismissed = reduceMentionNav(initialMentionNav, {
      type: "dismiss",
      start: match.start,
    });
    expect(isMentionOpen(match, dismissed)).toBe(false);
    const other = detectMention("x @a", 4)!;
    expect(isMentionOpen(other, dismissed)).toBe(true);
    expect(isMentionOpen(null, initialMentionNav)).toBe(false);
  });
});
