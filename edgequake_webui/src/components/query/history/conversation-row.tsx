"use client";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuSub,
  DropdownMenuSubContent,
  DropdownMenuSubTrigger,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { formatConversationDate } from "@/lib/query/format-conversation-date";
import { cn } from "@/lib/utils";
import type { ServerConversation } from "@/types";
import {
  Archive,
  Download,
  Edit2,
  Folder,
  FolderInput,
  Inbox,
  MessageSquare,
  MoreVertical,
  Pin,
  PinOff,
  Share2,
  Trash2,
} from "lucide-react";
import { memo, useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { DND_CONVERSATION_TYPE } from "../folder-sidebar";

export interface ConversationRowProps {
  conversation: ServerConversation;
  isActive: boolean;
  isSelected: boolean;
  isSelectionMode: boolean;
  selectedIds: Set<string>;
  onSelect: () => void;
  onToggleSelection: () => void;
  onRename: (title: string) => void;
  onPin: () => void;
  onArchive: () => void;
  onExport: () => void;
  onShare: () => void;
  onDelete: () => void;
  onMoveToFolder: (folderId: string | null) => void;
  folders: { id: string; name: string }[];
  tabIndex?: number;
  optionId?: string;
}

export const ConversationItem = memo(function ConversationItem({
  conversation,
  isActive,
  isSelected,
  isSelectionMode,
  selectedIds,
  onSelect,
  onToggleSelection,
  onRename,
  onPin,
  onArchive,
  onExport,
  onShare,
  onDelete,
  onMoveToFolder,
  folders,
  tabIndex = -1,
  optionId,
}: ConversationRowProps) {
  const { t, i18n } = useTranslation();
  const [isEditing, setIsEditing] = useState(false);
  const [editTitle, setEditTitle] = useState(conversation.title);

  const handleSaveTitle = useCallback(() => {
    if (editTitle.trim() && editTitle !== conversation.title) {
      onRename(editTitle.trim());
    }
    setIsEditing(false);
  }, [editTitle, conversation.title, onRename]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Enter") {
        e.preventDefault();
        handleSaveTitle();
      } else if (e.key === "Escape") {
        setEditTitle(conversation.title);
        setIsEditing(false);
      }
    },
    [handleSaveTitle, conversation.title]
  );

  const formattedDate = useMemo(
    () =>
      formatConversationDate(conversation.updated_at, i18n.language, {
        yesterday: t("common.yesterday", "Yesterday"),
      }),
    [conversation.updated_at, i18n.language, t],
  );

  // Handle click - if selection mode, toggle selection; otherwise select conversation
  const handleClick = useCallback(() => {
    if (isSelectionMode) {
      onToggleSelection();
    } else {
      onSelect();
    }
  }, [isSelectionMode, onToggleSelection, onSelect]);

  // Drag start: include all selected IDs if in selection mode, otherwise just this one
  const handleDragStart = useCallback(
    (e: React.DragEvent) => {
      const ids =
        isSelectionMode && isSelected && selectedIds.size > 0
          ? Array.from(selectedIds)
          : [conversation.id];
      e.dataTransfer.setData(DND_CONVERSATION_TYPE, JSON.stringify(ids));
      e.dataTransfer.effectAllowed = "move";
    },
    [conversation.id, isSelectionMode, isSelected, selectedIds],
  );

  return (
    <div
      className={cn(
        "group relative flex items-center gap-2 px-2.5 py-2 rounded-md cursor-pointer transition-all duration-150",
        isActive && !isSelectionMode
          ? "bg-primary/10 border border-primary/20"
          : isSelected
          ? "bg-accent border border-accent-foreground/20"
          : "hover:bg-muted/60 border border-transparent"
      )}
      onClick={handleClick}
      role="option"
      id={optionId}
      aria-selected={isActive}
      tabIndex={tabIndex}
      data-testid={`conversation-item-${conversation.id}`}
      draggable
      onDragStart={handleDragStart}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          handleClick();
        }
      }}
      aria-pressed={isActive}
    >
      {/* Selection checkbox in selection mode */}
      {isSelectionMode && (
        <div
          className={cn(
            "w-4 h-4 rounded border-2 flex items-center justify-center shrink-0 transition-colors",
            isSelected
              ? "bg-primary border-primary"
              : "border-muted-foreground/40"
          )}
        >
          {isSelected && (
            <svg className="w-3 h-3 text-primary-foreground" viewBox="0 0 12 12">
              <path
                d="M10 3L4.5 8.5L2 6"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
              />
            </svg>
          )}
        </div>
      )}

      {/* Icon */}
      <div
        className={cn(
          "w-7 h-7 rounded-md flex items-center justify-center shrink-0 transition-colors",
          isActive ? "bg-primary/15" : "bg-muted/40"
        )}
      >
        <MessageSquare
          className={cn(
            "h-3.5 w-3.5",
            isActive ? "text-primary" : "text-muted-foreground"
          )}
        />
      </div>

      {/* Content */}
      <div className="flex-1 min-w-0">
        {isEditing ? (
          <Input
            value={editTitle}
            onChange={(e) => setEditTitle(e.target.value)}
            onBlur={handleSaveTitle}
            onKeyDown={handleKeyDown}
            className="h-5 text-xs py-0 px-1"
            autoFocus
            onClick={(e) => e.stopPropagation()}
          />
        ) : (
          <>
            <div className="flex items-center gap-1">
              <p className="text-xs font-medium truncate leading-tight flex-1">
                {conversation.title}
              </p>
              {conversation.is_pinned && (
                <Pin className="h-2.5 w-2.5 text-amber-500 shrink-0" />
              )}
            </div>
            <p className="text-xs text-muted-foreground leading-tight mt-0.5">
              {conversation.message_count}{" "}
              {t("query.messages", "messages")} · {formattedDate}
            </p>
          </>
        )}
      </div>

      {/* Actions dropdown (hidden in selection mode) */}
      {!isEditing && !isSelectionMode && (
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button
              variant="ghost"
              size="icon"
              className="h-6 w-6 opacity-0 group-hover:opacity-100 transition-opacity"
              onClick={(e) => e.stopPropagation()}
              aria-label={t("common.moreOptions", "More options")}
            >
              <MoreVertical className="h-3 w-3" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" className="w-40">
            <DropdownMenuItem
              onClick={(e) => {
                e.stopPropagation();
                setIsEditing(true);
              }}
            >
              <Edit2 className="h-3 w-3 mr-2" />
              {t("common.rename", "Rename")}
            </DropdownMenuItem>
            <DropdownMenuItem
              onClick={(e) => {
                e.stopPropagation();
                onPin();
              }}
            >
              {conversation.is_pinned ? (
                <>
                  <PinOff className="h-3 w-3 mr-2" />
                  {t("common.unpin", "Unpin")}
                </>
              ) : (
                <>
                  <Pin className="h-3 w-3 mr-2" />
                  {t("common.pin", "Pin")}
                </>
              )}
            </DropdownMenuItem>
            <DropdownMenuItem
              onClick={(e) => {
                e.stopPropagation();
                onArchive();
              }}
            >
              <Archive className="h-3 w-3 mr-2" />
              {conversation.is_archived
                ? t("common.unarchive", "Unarchive")
                : t("common.archive", "Archive")}
            </DropdownMenuItem>

            {/* Move to Folder Submenu */}
            <DropdownMenuSub>
              <DropdownMenuSubTrigger>
                <FolderInput className="h-3 w-3 mr-2" />
                {t("query.moveToFolder", "Move to Folder")}
              </DropdownMenuSubTrigger>
              <DropdownMenuSubContent className="w-40">
                {/* "Unfiled" (root / no folder) */}
                <DropdownMenuItem
                  onClick={(e) => {
                    e.stopPropagation();
                    onMoveToFolder(null);
                  }}
                  disabled={!conversation.folder_id}
                >
                  <Inbox className="h-3 w-3 mr-2" />
                  {t("query.folders.unfiled", "Unfiled")}
                </DropdownMenuItem>
                {folders.length > 0 && <DropdownMenuSeparator />}
                {folders.map((folder) => (
                  <DropdownMenuItem
                    key={folder.id}
                    onClick={(e) => {
                      e.stopPropagation();
                      onMoveToFolder(folder.id);
                    }}
                    disabled={conversation.folder_id === folder.id}
                  >
                    <Folder className="h-3 w-3 mr-2" />
                    <span className="truncate">{folder.name}</span>
                  </DropdownMenuItem>
                ))}
                {folders.length === 0 && (
                  <DropdownMenuItem disabled>
                    <span className="text-xs text-muted-foreground italic">
                      {t("query.folders.noFolders", "No folders yet")}
                    </span>
                  </DropdownMenuItem>
                )}
              </DropdownMenuSubContent>
            </DropdownMenuSub>

            <DropdownMenuSeparator />
            <DropdownMenuItem
              onClick={(e) => {
                e.stopPropagation();
                onExport();
              }}
            >
              <Download className="h-3 w-3 mr-2" />
              {t("common.export", "Export")}
            </DropdownMenuItem>
            <DropdownMenuItem
              onClick={(e) => {
                e.stopPropagation();
                onShare();
              }}
            >
              <Share2 className="h-3 w-3 mr-2" />
              {t("common.share", "Share")}
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem
              onClick={(e) => {
                e.stopPropagation();
                onDelete();
              }}
              className="text-destructive focus:text-destructive"
            >
              <Trash2 className="h-3 w-3 mr-2" />
              {t("common.delete", "Delete")}
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      )}
    </div>
  );
});

export function ConversationSkeleton() {
  return (
    <div className="flex items-center gap-2 px-2.5 py-2">
      <Skeleton className="w-7 h-7 rounded-md" />
      <div className="flex-1 space-y-1">
        <Skeleton className="h-3 w-3/4" />
        <Skeleton className="h-2 w-1/2" />
      </div>
    </div>
  );
}
