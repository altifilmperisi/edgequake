import { describe, expect, it } from "vitest";
import {
  formatConversationDate,
  historyDateGroup,
} from "../format-conversation-date";

describe("formatConversationDate", () => {
  it("formats recent as time", () => {
    const now = new Date();
    const text = formatConversationDate(now, "en");
    expect(text.length).toBeGreaterThan(0);
    expect(text).not.toMatch(/Yesterday/i);
  });

  it("formats yesterday with label", () => {
    const d = new Date();
    d.setDate(d.getDate() - 1);
    d.setHours(15, 0, 0, 0);
    expect(formatConversationDate(d, "en", { yesterday: "Hier" })).toBe("Hier");
  });
});

describe("historyDateGroup", () => {
  it("buckets today/yesterday/last7/older", () => {
    const now = new Date("2026-10-01T12:00:00Z");
    expect(historyDateGroup(new Date("2026-10-01T10:00:00Z"), now)).toBe("today");
    expect(historyDateGroup(new Date("2026-09-30T10:00:00Z"), now)).toBe("yesterday");
    expect(historyDateGroup(new Date("2026-09-28T10:00:00Z"), now)).toBe("last7");
    expect(historyDateGroup(new Date("2026-08-01T10:00:00Z"), now)).toBe("older");
  });
});
