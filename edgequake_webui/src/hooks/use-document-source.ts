/**
 * SPEC-157 — Load a document and (for PDFs) its extracted markdown, sharing
 * React Query keys with the full-page viewer so both warm one cache (LAW-157-5).
 */
import { getDocument, getPdfContent } from "@/lib/api/edgequake";
import { ApiRequestError } from "@/lib/api/client";
import { resolvePdfId, withPdfMarkdown } from "@/lib/documents/viewer-target";
import { useTenantStore } from "@/stores/use-tenant-store";
import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";

export function useDocumentSource(documentId: string | undefined) {
  const { selectedWorkspaceId } = useTenantStore();

  const docQuery = useQuery({
    queryKey: ["document", documentId, selectedWorkspaceId],
    queryFn: () => getDocument(documentId!),
    enabled: !!documentId && !!selectedWorkspaceId,
    retry: 1,
    staleTime: 30 * 1000,
    placeholderData: (previous) => previous,
  });

  const pdfId = resolvePdfId(docQuery.data);
  const pdfQuery = useQuery({
    queryKey: ["pdfContent", pdfId, selectedWorkspaceId],
    queryFn: () => getPdfContent(pdfId!),
    enabled: !!pdfId && !!selectedWorkspaceId,
    staleTime: 60 * 1000,
  });

  const document = useMemo(
    () => withPdfMarkdown(docQuery.data, pdfId, pdfQuery.data?.markdown_content),
    [docQuery.data, pdfId, pdfQuery.data?.markdown_content],
  );

  const error = docQuery.error;
  return {
    document,
    pdfId,
    isLoading: docQuery.isLoading,
    isMarkdownLoading: pdfQuery.isLoading,
    isError: docQuery.isError,
    /** The document no longer exists (deleted since the answer was written). */
    isNotFound: error instanceof ApiRequestError && error.status === 404,
    refetch: docQuery.refetch,
  };
}
