/**
 * Streaming-aware chain-of-thought parser (SPEC-155).
 * Handles unclosed <think>/<thinking> tags and partial tags at chunk boundaries
 * so reasoning never leaks into the visible answer while streaming.
 */

export interface ParsedCotStreaming {
  /** Completed thinking blocks plus any in-progress open block */
  thinking: string[];
  /** Answer text outside think tags (empty while an open tag has no close yet) */
  response: string;
  /** True when an opening tag is still awaiting </think> or </thinking> */
  open: boolean;
  /** Leading fragment that may be a partial opening tag (held back) */
  heldPartial: string;
}

const OPEN_TAGS = ["<think>", "<thinking>"] as const;
const CLOSE_TAGS = ["</think>", "</thinking>"] as const;

/** Longest prefix of `s` that could still become an opening tag. */
function longestPartialOpenPrefix(s: string): string {
  const lower = s.toLowerCase();
  let best = "";
  for (const tag of OPEN_TAGS) {
    for (let len = 1; len < tag.length; len++) {
      const prefix = tag.slice(0, len);
      if (lower.endsWith(prefix) && prefix.length > best.length) {
        best = s.slice(s.length - len);
      }
    }
  }
  // Also hold a trailing bare "<"
  if (!best && s.endsWith("<")) {
    return "<";
  }
  return best;
}

/**
 * Parse COT content that may still be streaming.
 * - Closed blocks → thinking[]
 * - Open unclosed block → thinking[] + open:true, response empty after open
 * - Partial `<thi` at end → held in heldPartial (not emitted as response)
 */
export function parseCOTStreaming(
  content: string | undefined | null,
): ParsedCotStreaming {
  if (!content || typeof content !== "string") {
    return { thinking: [], response: "", open: false, heldPartial: "" };
  }

  const thinking: string[] = [];
  let response = "";
  let i = 0;
  let open = false;
  let openBuf = "";

  const pushResponse = (chunk: string) => {
    if (!chunk) return;
    response += chunk;
  };

  while (i < content.length) {
    if (open) {
      // Look for matching close tag
      const closeThink = content.toLowerCase().indexOf("</think>", i);
      const closeThinking = content.toLowerCase().indexOf("</thinking>", i);
      let closeAt = -1;
      let closeLen = 0;
      if (closeThink >= 0 && (closeThinking < 0 || closeThink <= closeThinking)) {
        closeAt = closeThink;
        closeLen = "</think>".length;
      } else if (closeThinking >= 0) {
        closeAt = closeThinking;
        closeLen = "</thinking>".length;
      }

      if (closeAt < 0) {
        openBuf += content.slice(i);
        // Still open — remaining is thinking
        const trimmed = openBuf.trim();
        if (trimmed) {
          thinking.push(trimmed);
        } else if (thinking.length === 0) {
          thinking.push("");
        }
        return { thinking, response: response.trim(), open: true, heldPartial: "" };
      }

      openBuf += content.slice(i, closeAt);
      thinking.push(openBuf.trim());
      openBuf = "";
      open = false;
      i = closeAt + closeLen;
      continue;
    }

    // Not inside a think block — find next open tag
    const lower = content.toLowerCase();
    let nextOpen = -1;
    let openLen = 0;
    for (const tag of OPEN_TAGS) {
      const at = lower.indexOf(tag, i);
      if (at >= 0 && (nextOpen < 0 || at < nextOpen)) {
        nextOpen = at;
        openLen = tag.length;
      }
    }

    if (nextOpen < 0) {
      const rest = content.slice(i);
      const partial = longestPartialOpenPrefix(rest);
      if (partial) {
        pushResponse(rest.slice(0, rest.length - partial.length));
        return {
          thinking,
          response: response.trim(),
          open: false,
          heldPartial: partial,
        };
      }
      pushResponse(rest);
      break;
    }

    pushResponse(content.slice(i, nextOpen));
    open = true;
    openBuf = "";
    i = nextOpen + openLen;
  }

  // Pattern 3 (complete only): **Thinking:** blocks — apply to response residue
  // when no XML think tags remain open. Skip while streaming open tags.
  if (!open && thinking.length === 0) {
    const blockRegex =
      /\*\*Thinking:\*\*\s*([\s\S]*?)(?=\n\n\*\*[A-Z]|\n\n---|\n\n#{1,3}\s|$)/gi;
    let match: RegExpExecArray | null;
    const blocks: string[] = [];
    let cleaned = response;
    while ((match = blockRegex.exec(response)) !== null) {
      blocks.push(match[1].trim());
    }
    if (blocks.length > 0) {
      cleaned = response.replace(blockRegex, "").trim();
      return {
        thinking: blocks,
        response: cleaned,
        open: false,
        heldPartial: "",
      };
    }
  }

  return {
    thinking: thinking.filter((t, idx) => t.length > 0 || (open && idx === thinking.length - 1)),
    response: response.trim(),
    open,
    heldPartial: "",
  };
}

/**
 * Final / non-streaming parse — same shapes as the legacy ThinkingDisplay helper.
 */
export function parseCOTContent(
  content: string | undefined | null,
): { thinking: string[]; response: string } {
  const parsed = parseCOTStreaming(content);
  // Drop empty in-progress placeholder if somehow present
  const thinking = parsed.thinking.filter((t) => t.length > 0);
  // Append held partial as response only when closed (shouldn't happen mid-stream)
  const response =
    parsed.open
      ? parsed.response
      : `${parsed.response}${parsed.heldPartial}`.trim();
  return { thinking, response };
}
