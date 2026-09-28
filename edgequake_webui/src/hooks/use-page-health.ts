"use client";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  getPagesHealth,
  reprocessPages,
  type ReprocessPagesRequest,
  type ReprocessPagesResponse,
} from "@/lib/api/edgequake/pages-health";

export function usePageHealth(
  documentId: string | undefined,
  enabled = true,
  opts?: { forcePollMs?: number | false },
) {
  return useQuery({
    queryKey: ["pages-health", documentId],
    queryFn: () => getPagesHealth(documentId!),
    enabled: Boolean(documentId) && enabled,
    refetchInterval: (q) => {
      if (opts?.forcePollMs) return opts.forcePollMs;
      const pages = q.state.data?.pages ?? [];
      const running = pages.some(
        (p) =>
          p.parse?.status === "running" ||
          p.figures?.status === "running" ||
          p.entities?.status === "running",
      );
      return running ? 1500 : false;
    },
  });
}

export function useReprocessPages(documentId: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: ReprocessPagesRequest) =>
      reprocessPages(documentId, body),
    onSuccess: (data: ReprocessPagesResponse) => {
      if (!data.dry_run) {
        void qc.invalidateQueries({ queryKey: ["pages-health", documentId] });
        void qc.invalidateQueries({ queryKey: ["document", documentId] });
      }
    },
  });
}
