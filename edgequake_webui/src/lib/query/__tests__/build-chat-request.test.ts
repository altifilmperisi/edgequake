import { describe, expect, it } from "vitest";
import {
  buildChatRequest,
  buildDocumentFilter,
} from "../build-chat-request";
import type { QueryRequestSettings } from "../build-chat-request";

const base: QueryRequestSettings = {
  mode: "mix",
  maxTokens: 2048,
  temperature: 0.7,
  topK: 40,
  stream: true,
};

describe("buildDocumentFilter", () => {
  it("returns undefined when nothing scoped", () => {
    expect(buildDocumentFilter(base)).toBeUndefined();
  });

  it("maps scopedDocumentIds to document_ids", () => {
    expect(
      buildDocumentFilter({ ...base, scopedDocumentIds: ["a", "b"] }),
    ).toEqual({ document_ids: ["a", "b"] });
  });

  it("preserves date filter without ids", () => {
    expect(
      buildDocumentFilter({
        ...base,
        documentFilter: { date_from: "2026-01-01" },
      }),
    ).toEqual({ date_from: "2026-01-01", document_ids: undefined });
  });
});

describe("buildChatRequest", () => {
  it("builds stream and non-stream from same settings", () => {
    const stream = buildChatRequest({
      settings: base,
      message: "hello",
      conversationId: "c1",
      language: "en",
      stream: true,
    });
    const once = buildChatRequest({
      settings: { ...base, stream: false },
      message: "hello",
      conversationId: "c1",
      language: "en",
      stream: false,
    });
    expect(stream.stream).toBe(true);
    expect(once.stream).toBe(false);
    expect(stream.message).toBe("hello");
    expect(stream.mode).toBe("mix");
    expect(stream.conversation_id).toBe("c1");
    expect(stream.content_granularity).toBe("citation");
  });

  it("uses agent granularity when fullChunkContent", () => {
    const req = buildChatRequest({
      settings: { ...base, fullChunkContent: true },
      message: "x",
    });
    expect(req.content_granularity).toBe("agent");
  });
});
