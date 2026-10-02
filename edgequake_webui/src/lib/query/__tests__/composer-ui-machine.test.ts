import { describe, expect, it } from "vitest";
import {
  composerPhaseFromSession,
  composerShowsSend,
  composerShowsStop,
  createComposerUiState,
  reduceComposerUi,
} from "../composer-ui-machine";

describe("composer-ui-machine", () => {
  it("maps session facts to phase", () => {
    expect(
      composerPhaseFromSession({
        isStreaming: true,
        queuedMessage: null,
        isError: false,
      }),
    ).toBe("streaming");
    expect(
      composerPhaseFromSession({
        isStreaming: false,
        queuedMessage: "hi",
        isError: false,
      }),
    ).toBe("queued");
    expect(
      composerPhaseFromSession({
        isStreaming: false,
        queuedMessage: null,
        isError: true,
      }),
    ).toBe("error");
  });

  it("stop only while streaming", () => {
    expect(composerShowsStop("streaming")).toBe(true);
    expect(composerShowsSend("streaming")).toBe(false);
    expect(composerShowsSend("idle")).toBe(true);
  });

  it("queue survives stream_end", () => {
    let s = createComposerUiState();
    s = reduceComposerUi(s, { type: "stream_start" });
    s = reduceComposerUi(s, { type: "queue", text: "next" });
    s = reduceComposerUi(s, { type: "stream_end" });
    expect(s.phase).toBe("queued");
    expect(s.queuedText).toBe("next");
  });
});
