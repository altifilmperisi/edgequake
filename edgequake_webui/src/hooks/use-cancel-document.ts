"use client";

import { useQueryClient } from "@tanstack/react-query";
import { useCallback } from "react";
import { toast } from "sonner";
import {
  cancelDocumentRun,
  type CancelTarget,
} from "@/lib/documents/cancel-document-run";

/** Stable `cancel(target)` for any surface; toasts only when cancel truly fails. */
export function useCancelDocument() {
  const queryClient = useQueryClient();
  return useCallback(
    async (target: CancelTarget) => {
      const outcome = await cancelDocumentRun(queryClient, target);
      if (outcome === "failed") {
        toast.error("Couldn't cancel this document. Try again, or delete it.");
      }
      return outcome;
    },
    [queryClient],
  );
}
