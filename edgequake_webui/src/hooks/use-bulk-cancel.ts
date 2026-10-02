"use client";

import { useCallback, useMemo, useState } from "react";
import { toast } from "sonner";
import { useCancelDocument } from "@/hooks/use-cancel-document";
import { canCancelDocument } from "@/lib/documents/document-run-state";
import type { Document } from "@/types";

/** SPEC-155: cancel every in-flight document in the current selection. */
export function useBulkCancel(
  documents: Document[],
  selectedIds: ReadonlySet<string>,
  onDone?: () => void,
) {
  const cancelDocument = useCancelDocument();
  const [isCancelling, setIsCancelling] = useState(false);

  const targets = useMemo(
    () =>
      documents.filter((d) => selectedIds.has(d.id) && canCancelDocument(d)),
    [documents, selectedIds],
  );

  const cancelSelected = useCallback(async () => {
    if (targets.length === 0 || isCancelling) return;
    setIsCancelling(true);
    try {
      const outcomes = await Promise.all(
        targets.map((d) =>
          cancelDocument({ documentId: d.id, trackId: d.track_id }),
        ),
      );
      const done = outcomes.filter((o) => o === "cancelled").length;
      if (done > 0) {
        toast.success(
          done === 1 ? "Cancelled 1 document" : `Cancelled ${done} documents`,
        );
      }
      onDone?.();
    } finally {
      setIsCancelling(false);
    }
  }, [targets, isCancelling, cancelDocument, onDone]);

  return { cancellableCount: targets.length, cancelSelected, isCancelling };
}
