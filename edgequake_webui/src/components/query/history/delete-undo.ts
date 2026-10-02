"use client";

import { deleteConversation } from "@/lib/api/conversations";
import { conversationKeys } from "@/lib/api/query-keys";
import type { ServerConversation } from "@/types";
import { useQueryClient } from "@tanstack/react-query";
import { useCallback, useRef } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";

const UNDO_DELAY_MS = 5000;

/** Delayed delete with sonner undo (refetch on undo — no restore API). */
export function useConversationDeleteUndo() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const timersRef = useRef<Map<string, ReturnType<typeof setTimeout>>>(
    new Map(),
  );

  const cancelPending = useCallback((id: string) => {
    const timer = timersRef.current.get(id);
    if (timer) {
      clearTimeout(timer);
      timersRef.current.delete(id);
    }
  }, []);

  const scheduleDelete = useCallback(
    (conversation: ServerConversation, onCommitted?: () => void) => {
      cancelPending(conversation.id);

      void queryClient.invalidateQueries({
        queryKey: conversationKeys.lists(),
      });

      const timer = setTimeout(async () => {
        timersRef.current.delete(conversation.id);
        try {
          await deleteConversation(conversation.id);
          await queryClient.invalidateQueries({
            queryKey: conversationKeys.lists(),
          });
          onCommitted?.();
        } catch {
          toast.error(
            t("query.history.deleteFailed", "Failed to delete conversation"),
          );
        }
      }, UNDO_DELAY_MS);

      timersRef.current.set(conversation.id, timer);

      toast(t("query.history.deleted", "Conversation deleted"), {
        action: {
          label: t("common.undo", "Undo"),
          onClick: () => {
            cancelPending(conversation.id);
            void queryClient.invalidateQueries({
              queryKey: conversationKeys.lists(),
            });
          },
        },
        duration: UNDO_DELAY_MS,
      });
    },
    [cancelPending, queryClient, t],
  );

  return { scheduleDelete, cancelPending };
}
