"use client";

import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { Button } from "@/components/ui/button";
import { Pencil, User } from "lucide-react";
import { memo } from "react";
import { useTranslation } from "react-i18next";
import type { ChatMessageData } from "./types";

export const UserMessage = memo(function UserMessage({
  message,
  onEdit,
}: {
  message: ChatMessageData;
  onEdit?: (content: string) => void;
}) {
  const { t } = useTranslation();
  return (
    <div
      className="flex justify-end mb-6 group/user"
      role="article"
      aria-label={t("query.yourMessage", "Your message")}
    >
      <div className="flex items-start gap-3 max-w-[95%] sm:max-w-[85%]">
        <div className="relative rounded-2xl rounded-tr-sm px-4 py-3 bg-primary text-primary-foreground">
          {onEdit ? (
            <Button
              type="button"
              variant="secondary"
              size="icon"
              className="absolute -left-10 top-1 h-7 w-7 opacity-0 group-hover/user:opacity-100 transition-opacity"
              onClick={() => onEdit(message.content)}
              aria-label={t("query.editMessage", "Edit message")}
              data-testid="query-message-edit"
            >
              <Pencil className="h-3.5 w-3.5" />
            </Button>
          ) : null}
          <p className="whitespace-pre-wrap break-words overflow-wrap-anywhere leading-relaxed">
            {message.content}
          </p>
        </div>
        <Avatar className="h-8 w-8 shrink-0 ring-2 ring-background">
          <AvatarFallback className="bg-primary/10">
            <User className="h-4 w-4" aria-hidden="true" />
          </AvatarFallback>
        </Avatar>
      </div>
    </div>
  );
});
