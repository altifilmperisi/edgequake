"use client";

import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Folder, FolderInput, Inbox, Trash2, X } from "lucide-react";
import { useTranslation } from "react-i18next";

interface SelectionToolbarProps {
  selectedCount: number;
  onDelete: () => void;
  onClear: () => void;
  onMoveToFolder: (folderId: string | null) => void;
  folders: { id: string; name: string }[];
}

export function HistorySelectionToolbar({
  selectedCount,
  onDelete,
  onClear,
  onMoveToFolder,
  folders,
}: SelectionToolbarProps) {
  const { t } = useTranslation();

  return (
    <div className="flex items-center justify-between px-3 py-2 bg-accent border-b">
      <span className="text-xs font-medium">
        {selectedCount} {t("common.selected", "selected")}
      </span>
      <div className="flex items-center gap-1">
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="ghost" size="sm" className="h-6 px-2 text-xs">
              <FolderInput className="h-3 w-3 mr-1" />
              {t("query.moveToFolder", "Move")}
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" className="w-40">
            <DropdownMenuItem onClick={() => onMoveToFolder(null)}>
              <Inbox className="h-3 w-3 mr-2" />
              {t("query.folders.unfiled", "Unfiled")}
            </DropdownMenuItem>
            {folders.length > 0 ? <DropdownMenuSeparator /> : null}
            {folders.map((folder) => (
              <DropdownMenuItem
                key={folder.id}
                onClick={() => onMoveToFolder(folder.id)}
              >
                <Folder className="h-3 w-3 mr-2" />
                <span className="truncate">{folder.name}</span>
              </DropdownMenuItem>
            ))}
          </DropdownMenuContent>
        </DropdownMenu>
        <Button
          variant="ghost"
          size="sm"
          className="h-6 px-2 text-xs text-destructive hover:text-destructive"
          onClick={onDelete}
        >
          <Trash2 className="h-3 w-3 mr-1" />
          {t("common.delete", "Delete")}
        </Button>
        <Button variant="ghost" size="sm" className="h-6 px-2 text-xs" onClick={onClear}>
          <X className="h-3 w-3 mr-1" />
          {t("common.cancel", "Cancel")}
        </Button>
      </div>
    </div>
  );
}
