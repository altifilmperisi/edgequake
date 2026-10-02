import { describe, expect, it } from "vitest";
import {
  parseCOTContent,
  parseCOTStreaming,
} from "../parse-cot-streaming";

describe("parseCOTStreaming", () => {
  it("returns empty for null/undefined", () => {
    expect(parseCOTStreaming(null)).toEqual({
      thinking: [],
      response: "",
      open: false,
      heldPartial: "",
    });
  });

  it("keeps unclosed <think> out of the response", () => {
    const p = parseCOTStreaming("<think>reasoning about X");
    expect(p.open).toBe(true);
    expect(p.thinking.join("")).toContain("reasoning about X");
    expect(p.response).toBe("");
  });

  it("splits closed think from answer", () => {
    const p = parseCOTStreaming(
      "<think>step one</think>\n\n## Answer\n\nHello",
    );
    expect(p.open).toBe(false);
    expect(p.thinking).toEqual(["step one"]);
    expect(p.response).toContain("## Answer");
    expect(p.response).toContain("Hello");
  });

  it("holds partial opening tag at chunk boundary", () => {
    const p = parseCOTStreaming("prefix <thi");
    expect(p.heldPartial.toLowerCase()).toBe("<thi");
    expect(p.response).toBe("prefix");
    expect(p.open).toBe(false);
  });

  it("handles split across conceptual chunks via accumulated string", () => {
    const a = parseCOTStreaming("<thi");
    expect(a.heldPartial.toLowerCase()).toBe("<thi");
    const b = parseCOTStreaming("<think>mid");
    expect(b.open).toBe(true);
    expect(b.response).toBe("");
    const c = parseCOTStreaming("<think>mid</think>Done");
    expect(c.open).toBe(false);
    expect(c.thinking).toEqual(["mid"]);
    expect(c.response).toBe("Done");
  });

  it("supports <thinking> tags", () => {
    const p = parseCOTStreaming("<thinking>plan</thinking>Out");
    expect(p.thinking).toEqual(["plan"]);
    expect(p.response).toBe("Out");
  });

  it("parseCOTContent wrapper drops open-only empties", () => {
    const closed = parseCOTContent("<think>a</think>b");
    expect(closed).toEqual({ thinking: ["a"], response: "b" });
    const open = parseCOTContent("<think>still");
    expect(open.thinking).toEqual(["still"]);
    expect(open.response).toBe("");
  });

  it("does not leak think markup into response while open", () => {
    const p = parseCOTStreaming(
      "Before\n<think>\nI should check sources\nmore",
    );
    expect(p.response).toBe("Before");
    expect(p.open).toBe(true);
    expect(p.thinking[0]).toContain("I should check sources");
  });
});
