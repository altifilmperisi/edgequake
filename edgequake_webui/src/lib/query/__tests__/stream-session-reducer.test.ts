import { describe, expect, it } from "vitest";
import {
  clearPendingAfterMerge,
  createStreamSession,
  reduceStreamSession,
} from "../stream-session-reducer";

describe("stream-session-reducer", () => {
  it("submit → thinking/retrieving with optimistic user", () => {
    let s = createStreamSession(null);
    s = reduceStreamSession(s, {
      type: "submit",
      text: "hi",
      messageId: "a1",
      mode: "mix",
    });
    expect(s.streamingState).toBe("thinking");
    expect(s.stage).toBe("retrieving");
    expect(s.optimisticUserMessage?.content).toBe("hi");
    expect(s.pendingMessage?.id).toBe("a1");
    expect(s.lastSubmittedText).toBe("hi");
  });

  it("context before tokens promotes stage to reading", () => {
    let s = createStreamSession(null);
    s = reduceStreamSession(s, {
      type: "submit",
      text: "hi",
      messageId: "a1",
      mode: "mix",
    });
    s = reduceStreamSession(s, {
      type: "context",
      context: { chunks: [{ content: "c", document_id: "d", score: 1 }], entities: [], relationships: [] },
    });
    expect(s.stage).toBe("reading");
    expect(s.pendingMessage?.context?.chunks).toHaveLength(1);
  });

  it("tokens promote to generating", () => {
    let s = createStreamSession(null);
    s = reduceStreamSession(s, {
      type: "submit",
      text: "hi",
      messageId: "a1",
      mode: "mix",
    });
    s = reduceStreamSession(s, {
      type: "token",
      content: "Hello",
      hasResponseText: true,
      cotOpen: false,
      nowMs: Date.now() + 100,
    });
    expect(s.streamingState).toBe("generating");
    expect(s.pendingMessage?.content).toContain("Hello");
  });

  it("open cot tokens stay in thinking stage", () => {
    let s = createStreamSession(null);
    s = reduceStreamSession(s, {
      type: "submit",
      text: "hi",
      messageId: "a1",
      mode: "mix",
    });
    s = reduceStreamSession(s, {
      type: "token",
      content: "<think>plan",
      hasResponseText: false,
      cotOpen: true,
      nowMs: Date.now() + 50,
    });
    expect(s.stage).toBe("thinking");
    expect(s.cotStartedAt).not.toBeNull();
  });

  it("abort keeps partial content (Q08)", () => {
    let s = createStreamSession(null);
    s = reduceStreamSession(s, {
      type: "submit",
      text: "hi",
      messageId: "a1",
      mode: "mix",
    });
    s = reduceStreamSession(s, {
      type: "token",
      content: "Partial answer",
      hasResponseText: true,
      cotOpen: false,
      nowMs: Date.now(),
    });
    s = reduceStreamSession(s, { type: "abort" });
    expect(s.stage).toBe("stopped");
    expect(s.pendingMessage?.content).toBe("Partial answer");
    expect(s.pendingMessage?.isStreaming).toBe(false);
  });

  it("abort with empty content keeps stopped pending", () => {
    let s = createStreamSession(null);
    s = reduceStreamSession(s, {
      type: "submit",
      text: "hi",
      messageId: "a1",
      mode: "mix",
    });
    s = reduceStreamSession(s, { type: "abort" });
    expect(s.pendingMessage?.stopped).toBe(true);
    expect(s.stage).toBe("stopped");
  });

  it("error marks pending as error", () => {
    let s = createStreamSession(null);
    s = reduceStreamSession(s, {
      type: "submit",
      text: "hi",
      messageId: "a1",
      mode: "mix",
    });
    s = reduceStreamSession(s, { type: "error", message: "boom" });
    expect(s.streamingState).toBe("error");
    expect(s.pendingMessage?.isError).toBe(true);
    expect(s.pendingMessage?.content).toBe("boom");
  });

  it("queue / clearQueue", () => {
    let s = createStreamSession(null);
    s = reduceStreamSession(s, { type: "queue", text: "next" });
    expect(s.queuedMessage).toBe("next");
    s = reduceStreamSession(s, { type: "clearQueue" });
    expect(s.queuedMessage).toBeNull();
  });

  it("clearPendingAfterMerge", () => {
    let s = createStreamSession(null);
    s = reduceStreamSession(s, {
      type: "submit",
      text: "hi",
      messageId: "a1",
      mode: "mix",
    });
    s = clearPendingAfterMerge(s);
    expect(s.pendingMessage).toBeNull();
    expect(s.optimisticUserMessage).toBeNull();
  });
});
