"use client";

import { Button } from "@/components/ui/button";
import { Loader2, MessageSquare } from "lucide-react";
import type { ServerConversation } from "@/types";
import { useVirtualizer } from "@tanstack/react-virtual";
import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type Ref,
  type RefObject,
} from "react";
import { useTranslation } from "react-i18next";
import { ConversationItem, ConversationSkeleton } from "./conversation-row";
import {
  buildGroupedConversationList,
  historyGroupLabel,
  type HistoryListEntry,
} from "./grouped-list";

const HEADER_HEIGHT = 28;
const ROW_HEIGHT = 52;

export interface ConversationListProps {
  parentRef: RefObject<HTMLDivElement | null>;
  conversations: ServerConversation[];
  isLoading: boolean;
  isFetchingNextPage: boolean;
  hasNextPage: boolean;
  loadMoreRef: Ref<HTMLDivElement | null>;
  activeConversationId: string | null;
  isSelectionMode: boolean;
  selectedIds: Set<string>;
  folders: { id: string; name: string }[];
  onSelect: (id: string) => void;
  onToggleSelection: (id: string) => void;
  onRename: (id: string, title: string) => void;
  onPin: (id: string, pinned: boolean) => void;
  onArchive: (id: string, archived: boolean) => void;
  onExport: (id: string) => void;
  onShare: (id: string) => void;
  onDelete: (conversation: ServerConversation) => void;
  onMoveToFolder: (conversationId: string, folderId: string | null) => void;
  onNewConversation: () => void;
  searchActive: boolean;
}

export function ConversationList({
  parentRef,
  conversations,
  isLoading,
  isFetchingNextPage,
  hasNextPage,
  loadMoreRef,
  activeConversationId,
  isSelectionMode,
  selectedIds,
  folders,
  onSelect,
  onToggleSelection,
  onRename,
  onPin,
  onArchive,
  onExport,
  onShare,
  onDelete,
  onMoveToFolder,
  onNewConversation,
  searchActive,
}: ConversationListProps) {
  const { t } = useTranslation();
  const listRef = useRef<HTMLDivElement>(null);
  const entries = useMemo(
    () => buildGroupedConversationList(conversations),
    [conversations],
  );

  const rowEntryIndices = useMemo(
    () =>
      entries
        .map((entry, index) => (entry.kind === "row" ? index : -1))
        .filter((index) => index >= 0),
    [entries],
  );

  const [focusedRowIdx, setFocusedRowIdx] = useState(0);

  useEffect(() => {
    if (focusedRowIdx >= rowEntryIndices.length) {
      setFocusedRowIdx(Math.max(0, rowEntryIndices.length - 1));
    }
  }, [focusedRowIdx, rowEntryIndices.length]);

  const virtualizer = useVirtualizer({
    count: entries.length + (hasNextPage ? 1 : 0),
    getScrollElement: () => parentRef.current,
    estimateSize: (index) => {
      if (index >= entries.length) return ROW_HEIGHT;
      return entries[index].kind === "header" ? HEADER_HEIGHT : ROW_HEIGHT;
    },
    overscan: 6,
  });

  const focusRow = useCallback(
    (rowIdx: number) => {
      const entryIndex = rowEntryIndices[rowIdx];
      if (entryIndex == null) return;
      setFocusedRowIdx(rowIdx);
      virtualizer.scrollToIndex(entryIndex, { align: "auto" });
      const conv = entries[entryIndex];
      if (conv.kind === "row") {
        document
          .getElementById(`history-option-${conv.conversation.id}`)
          ?.focus();
      }
    },
    [entries, rowEntryIndices, virtualizer],
  );

  const handleListKeyDown = useCallback(
    (event: React.KeyboardEvent) => {
      if (rowEntryIndices.length === 0) return;

      if (event.key === "ArrowDown") {
        event.preventDefault();
        focusRow(Math.min(focusedRowIdx + 1, rowEntryIndices.length - 1));
      } else if (event.key === "ArrowUp") {
        event.preventDefault();
        focusRow(Math.max(focusedRowIdx - 1, 0));
      } else if (event.key === "Home") {
        event.preventDefault();
        focusRow(0);
      } else if (event.key === "End") {
        event.preventDefault();
        focusRow(rowEntryIndices.length - 1);
      }
    },
    [focusRow, focusedRowIdx, rowEntryIndices.length],
  );

  if (isLoading) {
    return (
      <div className="p-2 space-y-1">
        {Array.from({ length: 5 }).map((_, i) => (
          <ConversationSkeleton key={i} />
        ))}
      </div>
    );
  }

  if (conversations.length === 0) {
    return (
      <div className="py-10 text-center">
        <div className="w-10 h-10 mx-auto rounded-full bg-muted/50 flex items-center justify-center mb-2">
          <MessageSquare className="h-5 w-5 text-muted-foreground/50" />
        </div>
        <p className="text-xs text-muted-foreground">
          {searchActive
            ? t("query.history.noResults", "No conversations found")
            : t("query.history.empty", "No conversations yet")}
        </p>
        {!searchActive ? (
          <Button
            variant="link"
            size="sm"
            onClick={onNewConversation}
            className="mt-1 text-xs text-primary"
          >
            {t("query.history.startFirst", "Start your first conversation")}
          </Button>
        ) : null}
      </div>
    );
  }

  return (
    <div
      ref={listRef}
      role="listbox"
      aria-label={t("query.history.title", "History")}
      tabIndex={0}
      onKeyDown={handleListKeyDown}
      data-testid="query-history-listbox"
      className="outline-none"
      style={{
        height: `${virtualizer.getTotalSize()}px`,
        width: "100%",
        position: "relative",
      }}
    >
      {virtualizer.getVirtualItems().map((virtualItem) => {
        const isLoader = virtualItem.index >= entries.length;
        if (isLoader) {
          return (
            <div
              key="loader"
              ref={loadMoreRef}
              style={{
                position: "absolute",
                top: 0,
                left: 0,
                width: "100%",
                height: `${virtualItem.size}px`,
                transform: `translateY(${virtualItem.start}px)`,
              }}
              className="flex items-center justify-center py-2"
            >
              {isFetchingNextPage ? (
                <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
              ) : null}
            </div>
          );
        }

        const entry = entries[virtualItem.index] as HistoryListEntry;

        if (entry.kind === "header") {
          return (
            <div
              key={entry.id}
              role="presentation"
              className="px-3 pt-2 pb-0.5 text-[10px] font-semibold uppercase tracking-wide text-muted-foreground"
              style={{
                position: "absolute",
                top: 0,
                left: 0,
                width: "100%",
                height: `${virtualItem.size}px`,
                transform: `translateY(${virtualItem.start}px)`,
              }}
              data-testid={`history-group-${entry.group}`}
            >
              {historyGroupLabel(entry.group, t)}
            </div>
          );
        }

        const { conversation } = entry;
        const rowIdx = rowEntryIndices.indexOf(virtualItem.index);

        return (
          <div
            key={entry.id}
            style={{
              position: "absolute",
              top: 0,
              left: 0,
              width: "100%",
              height: `${virtualItem.size}px`,
              transform: `translateY(${virtualItem.start}px)`,
              padding: "0 0.5rem",
            }}
          >
            <ConversationItem
              conversation={conversation}
              isActive={conversation.id === activeConversationId}
              isSelected={selectedIds.has(conversation.id)}
              isSelectionMode={isSelectionMode}
              selectedIds={selectedIds}
              onSelect={() => onSelect(conversation.id)}
              onToggleSelection={() => onToggleSelection(conversation.id)}
              onRename={(title) => onRename(conversation.id, title)}
              onPin={() => onPin(conversation.id, !conversation.is_pinned)}
              onArchive={() =>
                onArchive(conversation.id, !conversation.is_archived)
              }
              onExport={() => onExport(conversation.id)}
              onShare={() => onShare(conversation.id)}
              onDelete={() => onDelete(conversation)}
              onMoveToFolder={(folderId) =>
                onMoveToFolder(conversation.id, folderId)
              }
              folders={folders}
              tabIndex={rowIdx === focusedRowIdx ? 0 : -1}
              optionId={`history-option-${conversation.id}`}
            />
          </div>
        );
      })}
    </div>
  );
}
