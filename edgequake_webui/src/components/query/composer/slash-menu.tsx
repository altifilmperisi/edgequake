"use client";

import { cn } from "@/lib/utils";
import { FLOATING_EDGE } from "@/components/ui/surface";
import { Plus } from "lucide-react";
import { useTranslation } from "react-i18next";

interface SlashMenuProps {
  onNewChat: () => void;
  onClose: () => void;
}

/** Lightweight slash-command menu (SPEC-155). */
export function SlashMenu({ onNewChat, onClose }: SlashMenuProps) {
  const { t } = useTranslation();

  return (
    <div
      className={cn(
        "overflow-hidden rounded-2xl bg-popover p-1.5",
        FLOATING_EDGE,
        "animate-in fade-in-0 slide-in-from-bottom-1 duration-150",
      )}
      data-testid="query-slash-menu"
      role="menu"
    >
      <button
        type="button"
        role="menuitem"
        data-testid="query-slash-new-chat"
        className="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-left text-sm transition-colors hover:bg-accent focus-visible:bg-accent focus-visible:outline-none"
        onClick={() => {
          onNewChat();
          onClose();
        }}
      >
        <Plus className="h-4 w-4 text-muted-foreground" aria-hidden />
        {t("query.slash.newChat", "New chat")}
      </button>
    </div>
  );
}
