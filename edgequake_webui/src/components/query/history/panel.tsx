"use client";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { ResizablePanel } from "@/components/ui/resizable-panel";
import {
  useConversation,
  useConversations,
  useDeleteConversations,
  useUpdateConversation,
} from "@/hooks/use-conversations";
import { useFolders } from "@/hooks/use-folders";
import { useMoveConversation, useMoveConversations } from "@/hooks/use-move-conversation";
import { cn } from "@/lib/utils";
import {
  useActiveConversationId,
  useConversationFilters,
  useConversationSelection,
  useHistoryPanelOpen,
  useQueryUIStore,
} from "@/stores/use-query-ui-store";
import { ChevronLeft, ChevronRight, Loader2, MessageSquare, Plus } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useInView } from "react-intersection-observer";
import { ExportDialog } from "../export-dialog";
import { FolderSidebar } from "../folder-sidebar";
import { MigrationBanner } from "../migration-banner";
import { ShareDialog } from "../share-dialog";
import { ConversationList } from "./conversation-list";
import { useConversationDeleteUndo } from "./delete-undo";
import { HistoryFilterBar } from "./filter-bar";
import { HistorySearch } from "./history-search";
import { HistorySelectionToolbar } from "./selection-toolbar";

interface ConversationHistoryPanelV2Props {
  className?: string;
}

export function ConversationHistoryPanelV2({
  className,
}: ConversationHistoryPanelV2Props) {
  const { t } = useTranslation();
  const historyOpen = useHistoryPanelOpen();
  const toggleHistoryPanel = useQueryUIStore((s) => s.toggleHistoryPanel);
  const setActiveConversation = useQueryUIStore((s) => s.setActiveConversation);
  const activeConversationId = useActiveConversationId();
  const { filters, sort } = useConversationFilters();
  const selection = useConversationSelection();

  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);
  const [conversationToDelete, setConversationToDelete] = useState<
    import("@/types").ServerConversation | null
  >(null);
  const [showFilters, setShowFilters] = useState(false);
  const [exportDialogOpen, setExportDialogOpen] = useState(false);
  const [shareDialogOpen, setShareDialogOpen] = useState(false);
  const [selectedConversationForAction, setSelectedConversationForAction] =
    useState<string | null>(null);
  const parentRef = useRef<HTMLDivElement>(null);

  const { ref: loadMoreRef, inView } = useInView({ threshold: 0.1 });
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
    date_from: filters.dateFrom ?? undefined,
    date_to: filters.dateTo ?? undefined,
    sort: sort.field,
    order: sort.order,
  });

  const updateConversation = useUpdateConversation();
  const deleteConversationsBatch = useDeleteConversations();
  const moveConversation = useMoveConversation();
  const moveConversations = useMoveConversations();

  const { data: folders } = useFolders();
  const folderList = useMemo(() => {
    if (!folders) return [];
    return folders
      .sort((a, b) => a.position - b.position)
      .map((f) => ({ id: f.id, name: f.name }));
  }, [folders]);

  const { data: conversationForAction } = useConversation(
    selectedConversationForAction,
  );

  const conversations = useMemo(
    () => data?.pages.flatMap((page) => page.items) ?? [],
    [data],
  );

  useEffect(() => {
    if (inView && hasNextPage && !isFetchingNextPage) {
      fetchNextPage();
    }
  }, [inView, hasNextPage, isFetchingNextPage, fetchNextPage]);

  const handleNewConversation = useCallback(() => {
    setActiveConversation(null);
  }, [setActiveConversation]);

  const handleDeleteConfirm = useCallback(() => {
    if (selection.isSelectionMode && selection.selectedIds.size > 0) {
      deleteConversationsBatch.mutate(Array.from(selection.selectedIds));
      selection.clearSelection();
      selection.setSelectionMode(false);
    } else if (conversationToDelete) {
      scheduleDelete(conversationToDelete, () => {
        if (activeConversationId === conversationToDelete.id) {
          setActiveConversation(null);
        }
      });
      setConversationToDelete(null);
    }
    setDeleteDialogOpen(false);
  }, [
    activeConversationId,
    conversationToDelete,
    deleteConversationsBatch,
    scheduleDelete,
    selection,
    setActiveConversation,
  ]);

  if (!historyOpen) {
    return (
      <div
        className={cn(
          "flex flex-col items-center justify-start py-3 w-10 border-l bg-card/50 shrink-0 transition-all duration-200",
          className,
        )}
      >
        <Button
          variant="ghost"
          size="icon"
          className="h-7 w-7 hover:bg-muted"
          onClick={toggleHistoryPanel}
          aria-label={t("query.history.expand", "Expand history")}
        >
          <ChevronLeft className="h-3.5 w-3.5" />
        </Button>
        <div className="mt-3 flex flex-col items-center gap-1.5">
          <MessageSquare className="h-3.5 w-3.5 text-muted-foreground" />
          <span
            className="text-xs text-muted-foreground font-medium"
            style={{ writingMode: "vertical-rl", textOrientation: "mixed" }}
          >
            {t("query.history.title", "History")}
          </span>
        </div>
      </div>
    );
  }

  return (
    <ResizablePanel
      side="right"
      defaultWidth={280}
      minWidth={240}
      maxWidth={500}
      storageKey="conversation-history-panel-width"
      ariaLabel={t("query.history.resize", "Resize history panel")}
      className="flex"
    >
      <aside
        className={cn(
          "flex flex-col w-full h-full min-h-0 border-l bg-card/50 backdrop-blur-sm transition-all duration-200",
          className,
        )}
        aria-label={t("query.history.title", "Conversation history")}
      >
        <MigrationBanner />

        <div className="flex items-center justify-between px-3 py-2 border-b bg-muted/20 shrink-0">
          <h2 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
            {t("query.history.title", "History")}
          </h2>
          <div className="flex items-center gap-0.5">
            <Button
              variant="ghost"
              size="icon"
              className="h-6 w-6 hover:bg-muted"
              onClick={handleNewConversation}
              aria-label={t("query.history.newConversation", "New conversation")}
              data-testid="query-history-new"
            >
              <Plus className="h-3.5 w-3.5" />
            </Button>
            <Button
              variant="ghost"
              size="icon"
              className="h-6 w-6 hover:bg-muted"
              onClick={toggleHistoryPanel}
              aria-label={t("query.history.collapse", "Collapse history")}
            >
              <ChevronRight className="h-3.5 w-3.5" />
            </Button>
          </div>
        </div>

        {selection.isSelectionMode && selection.selectedIds.size > 0 ? (
          <HistorySelectionToolbar
            selectedCount={selection.selectedIds.size}
            onDelete={() => setDeleteDialogOpen(true)}
            onClear={() => {
              selection.clearSelection();
              selection.setSelectionMode(false);
            }}
            onMoveToFolder={(folderId) =>
              moveConversations.mutate({
                conversationIds: Array.from(selection.selectedIds),
                folderId,
              })
            }
            folders={folderList}
          />
        ) : null}

        <HistorySearch />

        <div className="p-2 border-b shrink-0">
          <FolderSidebar />
        </div>

        {showFilters ? (
          <HistoryFilterBar onClose={() => setShowFilters(false)} />
        ) : null}

        <div
          ref={parentRef}
          className="flex-1 overflow-auto"
          style={{ contain: "strict" }}
          data-testid="conversation-history-scroll"
        >
          <ConversationList
            parentRef={parentRef}
            conversations={conversations}
            isLoading={isLoading}
            isFetchingNextPage={isFetchingNextPage}
            hasNextPage={Boolean(hasNextPage)}
            loadMoreRef={loadMoreRef}
            activeConversationId={activeConversationId}
            isSelectionMode={selection.isSelectionMode}
            selectedIds={selection.selectedIds}
            folders={folderList}
            onSelect={setActiveConversation}
            onToggleSelection={selection.toggleSelection}
            onRename={(id, title) =>
              updateConversation.mutate({ id, data: { title } })
            }
            onPin={(id, is_pinned) =>
              updateConversation.mutate({ id, data: { is_pinned } })
            }
            onArchive={(id, is_archived) =>
              updateConversation.mutate({ id, data: { is_archived } })
            }
            onExport={(id) => {
              setSelectedConversationForAction(id);
              setExportDialogOpen(true);
            }}
            onShare={(id) => {
              setSelectedConversationForAction(id);
              setShareDialogOpen(true);
            }}
            onDelete={(conversation) => {
              setConversationToDelete(conversation);
              setDeleteDialogOpen(true);
            }}
            onMoveToFolder={(conversationId, folderId) =>
              moveConversation.mutate({ conversationId, folderId })
            }
            onNewConversation={handleNewConversation}
            searchActive={Boolean(filters.search)}
          />
        </div>

        <Dialog open={deleteDialogOpen} onOpenChange={setDeleteDialogOpen}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>
                {selection.isSelectionMode && selection.selectedIds.size > 0
                  ? t("query.history.deleteMultipleTitle", "Delete {{count}} conversations?", {
                      count: selection.selectedIds.size,
                    })
                  : t("query.history.deleteTitle", "Delete conversation?")}
              </DialogTitle>
              <DialogDescription>
                {t(
                  "query.history.deleteDescription",
                  "This action cannot be undone. This will permanently delete the conversation and all its messages.",
                )}
              </DialogDescription>
            </DialogHeader>
            <DialogFooter>
              <Button variant="outline" onClick={() => setDeleteDialogOpen(false)}>
                {t("common.cancel", "Cancel")}
              </Button>
              <Button variant="destructive" onClick={handleDeleteConfirm}>
                {deleteConversationsBatch.isPending ? (
                  <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                ) : null}
                {t("common.delete", "Delete")}
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>

        <ExportDialog
          conversation={conversationForAction ?? null}
          open={exportDialogOpen}
          onOpenChange={(open) => {
            setExportDialogOpen(open);
            if (!open) setSelectedConversationForAction(null);
          }}
        />

        <ShareDialog
          conversation={conversationForAction ?? null}
          open={shareDialogOpen}
          onOpenChange={(open) => {
            setShareDialogOpen(open);
            if (!open) setSelectedConversationForAction(null);
          }}
        />
      </aside>
    </ResizablePanel>
  );
}

export default ConversationHistoryPanelV2;
