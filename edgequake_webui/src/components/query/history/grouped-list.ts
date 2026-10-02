import {
  historyDateGroup,
  type HistoryDateGroup,
} from "@/lib/query/format-conversation-date";
import type { ServerConversation } from "@/types";

export type HistoryListEntry =
  | { kind: "header"; id: string; group: HistoryDateGroup }
  | { kind: "row"; id: string; conversation: ServerConversation };

export function buildGroupedConversationList(
  conversations: ServerConversation[],
): HistoryListEntry[] {
  const entries: HistoryListEntry[] = [];
  let lastGroup: HistoryDateGroup | null = null;

  for (const conversation of conversations) {
    const group = historyDateGroup(conversation.updated_at);
    if (group !== lastGroup) {
      entries.push({ kind: "header", id: `header-${group}-${conversation.id}`, group });
      lastGroup = group;
    }
    entries.push({ kind: "row", id: conversation.id, conversation });
  }

  return entries;
}

export function historyGroupLabel(
  group: HistoryDateGroup,
  t: (key: string, fallback: string) => string,
): string {
  switch (group) {
    case "today":
      return t("query.history.group.today", "Today");
    case "yesterday":
      return t("query.history.group.yesterday", "Yesterday");
    case "last7":
      return t("query.history.group.last7", "Previous 7 days");
    default:
      return t("query.history.group.older", "Older");
  }
}
