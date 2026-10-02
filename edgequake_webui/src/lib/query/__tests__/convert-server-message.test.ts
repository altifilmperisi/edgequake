import { describe, expect, it } from "vitest";
import { convertServerMessage } from "../convert-server-message";
import type { ServerMessage } from "@/types";

function baseMsg(overrides: Partial<ServerMessage> = {}): ServerMessage {
  return {
    id: "m1",
    conversation_id: "c1",
    role: "assistant",
    content: "Hello",
    is_error: false,
    created_at: "2026-10-01T00:00:00Z",
    updated_at: "2026-10-01T00:00:00Z",
    ...overrides,
  };
}

describe("convertServerMessage SPEC-155 B2/B3", () => {
  it("maps feedback_rating", () => {
    const msg = convertServerMessage(baseMsg({ feedback_rating: "up" }));
    expect(msg.feedbackRating).toBe("up");
  });

  it("maps finish_reason interrupted to stopped", () => {
    const msg = convertServerMessage(
      baseMsg({ finish_reason: "interrupted", content: "partial" }),
    );
    expect(msg.stopped).toBe(true);
  });

  it("does not mark stop finish_reason as stopped", () => {
    const msg = convertServerMessage(baseMsg({ finish_reason: "stop" }));
    expect(msg.stopped).toBe(false);
  });
});
