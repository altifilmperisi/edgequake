"use client";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from "@/components/ui/sheet";
import { Skeleton } from "@/components/ui/skeleton";
import { useConversations } from "@/hooks/use-conversations";
import { formatConversationDate } from "@/lib/query/format-conversation-date";
import { cn } from "@/lib/utils";
import {
  useActiveConversationId,
  useConversationFilters,
  useQueryUIStore,
} from "@/stores/use-query-ui-store";
import type { ServerConversation } from "@/types";
import {
  Archive,
  Clock,
  Loader2,
  Menu,
  MessageSquare,
  Pin,
  Plus,
  Search,
  Trash2,
} from "lucide-react";
import { memo, useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { FolderSidebar } from "./folder-sidebar";
import { useConversationDeleteUndo } from "./history/delete-undo";

interface MobileConversationItemProps {
  conversation: ServerConversation;
  isActive: boolean;
  onSelect: () => void;
  onDelete: () => void;
}

const MobileConversationItem = memo(function MobileConversationItem({
  conversation,
  isActive,
  onSelect,
  onDelete,
}: MobileConversationItemProps) {
  const { t, i18n } = useTranslation();

  const formattedDate = useMemo(
    () =>
      formatConversationDate(conversation.updated_at, i18n.language, {
        yesterday: t("common.yesterday", "Yesterday"),
      }),
    [conversation.updated_at, i18n.language, t],
  );

  return (
    <div
      className={cn(
        "group flex items-center gap-3 px-4 py-3 border-b active:bg-muted transition-colors",
        isActive && "bg-primary/5 border-l-2 border-l-primary",
      )}
      onClick={onSelect}
      role="option"
      aria-selected={isActive}
      tabIndex={0}
      data-testid={`mobile-conversation-${conversation.id}`}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          onSelect();
        }
      }}
    >
      <div
        className={cn(
          "w-10 h-10 rounded-full flex items-center justify-center shrink-0",
          isActive ? "bg-primary/15" : "bg-muted/50",
        )}
      >
        <MessageSquare
          className={cn(
            "h-5 w-5",
            isActive ? "text-primary" : "text-muted-foreground",
          )}
        />
      </div>

      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-1.5">
          <p className="text-sm font-medium truncate flex-1">
            {conversation.title}
          </p>
          {conversation.is_pinned ? (
            <Pin className="h-3 w-3 text-amber-500 shrink-0" />
          ) : null}
          {conversation.is_archived ? (
            <Archive className="h-3 w-3 text-muted-foreground shrink-0" />
          ) : null}
        </div>
        <div className="flex items-center gap-2 mt-0.5">
          <Clock className="h-3 w-3 text-muted-foreground" />
          <p className="text-xs text-muted-foreground">{formattedDate}</p>
          <span className="text-muted-foreground">·</span>
          <p className="text-xs text-muted-foreground">
            {conversation.message_count} {t("query.messages", "messages")}
          </p>
        </div>
        {conversation.last_message_preview ? (
          <p className="text-xs text-muted-foreground truncate mt-1">
            {conversation.last_message_preview}
          </p>
        ) : null}
      </div>

      <Button
        type="button"
        variant="ghost"
        size="icon"
        className="h-8 w-8 shrink-0 opacity-0 group-focus-within:opacity-100 group-hover:opacity-100"
        aria-label={t("common.delete", "Delete")}
        onClick={(e) => {
          e.stopPropagation();
          onDelete();
        }}
      >
        <Trash2 className="h-4 w-4 text-muted-foreground" />
      </Button>
    </div>
  );
});

function MobileConversationSkeleton() {
  return (
    <div className="flex items-center gap-3 px-4 py-3 border-b">
      <Skeleton className="w-10 h-10 rounded-full" />
      <div className="flex-1 space-y-2">
        <Skeleton className="h-4 w-3/4" />
        <Skeleton className="h-3 w-1/2" />
      </div>
    </div>
  );
}

interface MobileHistoryPanelProps {
  className?: string;
}

export function MobileHistoryPanel({ className }: MobileHistoryPanelProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const { filters, setFilters } = useConversationFilters();
  const setActiveConversation = useQueryUIStore((s) => s.setActiveConversation);
  const activeConversationId = useActiveConversationId();
  const { scheduleDelete } = useConversationDeleteUndo();

  const {
    data,
    isLoading,
    isFetchingNextPage,
    hasNextPage,
    fetchNextPage,
  } = useConversations({
    mode: filters.mode ?? undefined,
    archived: filters.archived,
    pinned: filters.pinned ?? undefined,
    folder_id: filters.folderId ?? undefined,
    unfiled: filters.unfiled || undefined,
    search: filters.search || undefined,
  });

  const conversations = useMemo(
    () => data?.pages.flatMap((page) => page.items) ?? [],
    [data],
  );

  const handleSelectConversation = useCallback(
    (conversationId: string) => {
      setActiveConversation(conversationId);
      setOpen(false);
    },
    [setActiveConversation],
  );

  const handleNewConversation = useCallback(() => {
    setActiveConversation(null);
    setOpen(false);
  }, [setActiveConversation]);

  const handleScroll = useCallback(
    (e: React.UIEvent<HTMLDivElement>) => {
      const target = e.currentTarget;
      const scrolledToBottom =
        target.scrollHeight - target.scrollTop <= target.clientHeight + 100;

      if (scrolledToBottom && hasNextPage && !isFetchingNextPage) {
        fetchNextPage();
      }
    },
    [hasNextPage, isFetchingNextPage, fetchNextPage],
  );

  return (
    <Sheet open={open} onOpenChange={setOpen}>
      <SheetTrigger asChild>
        <Button
          variant="ghost"
          size="icon"
          className={cn("md:hidden h-9 w-9", className)}
          aria-label={t("query.history.open", "Open conversation history")}
          data-testid="query-mobile-history"
        >
          <Menu className="h-5 w-5" />
        </Button>
      </SheetTrigger>
      <SheetContent side="left" size="md" className="w-full sm:w-96 p-0 flex flex-col">
        <SheetHeader className="border-b bg-muted/20 px-5 py-3">
          <div className="flex items-center justify-between gap-2">
            <SheetTitle className="text-base">
              {t("query.history.title", "History")}
            </SheetTitle>
            <Button
              variant="ghost"
              size="icon"
              className="h-8 w-8 shrink-0"
              onClick={handleNewConversation}
              aria-label={t("query.history.newConversation", "New conversation")}
            >
              <Plus className="h-4 w-4" />
            </Button>
          </div>
        </SheetHeader>

        <div className="px-5 py-3 border-b">
          <div className="relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
            <Input
              placeholder={t("query.history.search", "Search conversations...")}
              value={filters.search}
              onChange={(e) => setFilters({ search: e.target.value })}
              className="h-10 pl-10 text-sm"
              data-testid="query-mobile-history-search"
            />
          </div>
        </div>

        <div className="px-5 py-3 border-b">
          <FolderSidebar />
        </div>

        <div
          className="flex-1 overflow-y-auto"
          role="listbox"
          aria-label={t("query.history.title", "History")}
          onScroll={handleScroll}
        >
          {isLoading ? (
            <div>
              {Array.from({ length: 5 }).map((_, i) => (
                <MobileConversationSkeleton key={i} />
              ))}
            </div>
          ) : conversations.length === 0 ? (
            <div className="py-16 text-center px-4">
              <div className="w-16 h-16 mx-auto rounded-full bg-muted/50 flex items-center justify-center mb-4">
                <MessageSquare className="h-8 w-8 text-muted-foreground/50" />
              </div>
              <p className="text-sm text-muted-foreground mb-2">
                {filters.search
                  ? t("query.history.noResults", "No conversations found")
                  : t("query.history.empty", "No conversations yet")}
              </p>
              {!filters.search ? (
                <Button variant="outline" size="sm" onClick={handleNewConversation}>
                  {t("query.history.startFirst", "Start your first conversation")}
                </Button>
              ) : null}
            </div>
          ) : (
            <div>
              {conversations.map((conversation) => (
                <MobileConversationItem
                  key={conversation.id}
                  conversation={conversation}
                  isActive={conversation.id === activeConversationId}
                  onSelect={() => handleSelectConversation(conversation.id)}
                  onDelete={() => {
                    scheduleDelete(conversation, () => {
                      if (activeConversationId === conversation.id) {
                        setActiveConversation(null);
                      }
                    });
                  }}
                />
              ))}

              {isFetchingNextPage ? (
                <div className="flex items-center justify-center py-4">
                  <Loader2 className="h-5 w-5 animate-spin text-muted-foreground" />
                </div>
              ) : null}
            </div>
          )}
        </div>
      </SheetContent>
    </Sheet>
  );
}

export default MobileHistoryPanel;
