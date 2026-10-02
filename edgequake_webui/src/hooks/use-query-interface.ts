"use client";

import { useQueryConversationLifecycle } from "@/hooks/use-query-conversation-lifecycle";
import { useQueryImages } from "@/hooks/use-query-images";
import { useQueryStreamSession } from "@/hooks/use-query-stream-session";
import { useStickToBottom } from "@/hooks/use-stick-to-bottom";
import { convertServerMessage } from "@/lib/query/convert-server-message";
import { mergeQueryMessages } from "@/lib/query/merge-query-messages";
import type { QueryMessage } from "@/lib/query/query-interface-types";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

/** Composes query page lifecycle, streaming, scroll, and image hooks (SPEC-155 W7Q). */
export function useQueryInterface() {
  const [input, setInput] = useState("");
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const draftKeyRef = useRef<string>("query-draft:new");

  const clearPendingState = useCallback(() => {
    // no-op placeholder — stream session owns pending
  }, []);

  const {
    activeConversation,
    activeConversationId,
    isLoadingConversation,
    querySettings,
    setQuerySettings,
    store,
  } = useQueryConversationLifecycle({
    onTenantContextChange: clearPendingState,
  });

  // Per-conversation draft persistence (Q18)
  useEffect(() => {
    draftKeyRef.current = `query-draft:${activeConversationId ?? "new"}`;
    try {
      const saved = localStorage.getItem(draftKeyRef.current);
      if (saved != null) setInput(saved);
      else setInput("");
    } catch {
      /* ignore */
    }
  }, [activeConversationId]);

  useEffect(() => {
    try {
      if (input) localStorage.setItem(draftKeyRef.current, input);
      else localStorage.removeItem(draftKeyRef.current);
    } catch {
      /* ignore */
    }
  }, [input]);

  const baseMessages = useMemo(() => {
    return (activeConversation?.messages ?? []).map(convertServerMessage);
  }, [activeConversation?.messages]);

  // Temporary messages list for regenerate (before stream session merges)
  const [messagesForHook, setMessagesForHook] = useState<QueryMessage[]>([]);
  useEffect(() => {
    setMessagesForHook(baseMessages);
  }, [baseMessages]);

  const stream = useQueryStreamSession({
    querySettings,
    setQuerySettings,
    activeConversationId,
    store,
    messages: messagesForHook,
  });

  const messages = useMemo(() => {
    return mergeQueryMessages(
      baseMessages,
      stream.optimisticUserMessage,
      stream.pendingMessage,
    );
  }, [baseMessages, stream.optimisticUserMessage, stream.pendingMessage]);

  const { scrollRef, scrollAnchorRef, showJumpPill, jumpToLatest } =
    useStickToBottom(stream.streamingState, messages);

  const images = useQueryImages();

  // Drain queue when stream completes
  useEffect(() => {
    if (
      stream.queuedMessage &&
      !stream.isStreamingOrLoading &&
      stream.streamingState !== "error"
    ) {
      const next = stream.queuedMessage;
      stream.clearQueue();
      void stream.submitQuery(next);
    }
  }, [
    stream.queuedMessage,
    stream.isStreamingOrLoading,
    stream.streamingState,
    stream.clearQueue,
    stream.submitQuery,
  ]);

  const handleInputChange = useCallback((value: string) => {
    setInput(value);
  }, []);

  const handleEditMessage = useCallback(
    (messageId: string) => {
      const text = stream.beginEditFromMessage(messageId);
      if (!text) return;
      setInput(text);
      requestAnimationFrame(() => inputRef.current?.focus());
    },
    [stream],
  );

  const handleSubmit = async () => {
    if (!input.trim() || stream.isStreamingOrLoading) return;

    const queryText = input.trim();
    const currentImages = images.getPayload();
    setInput("");
    try {
      localStorage.removeItem(draftKeyRef.current);
    } catch {
      /* ignore */
    }
    images.clearImages();

    if (inputRef.current) {
      inputRef.current.style.height = "auto";
    }

    await stream.submitQuery(queryText, currentImages);
  };

  const handleSuggestionClick = useCallback((text: string) => {
    setInput(text);
    inputRef.current?.focus();
  }, []);

  const handleNewConversation = useCallback(() => {
    store.setActiveConversation(null);
    stream.setPendingMessage(null);
    stream.setOptimisticUserMessage(null);
    stream.clearEditAnchor();
    setInput("");
    try {
      localStorage.removeItem("query-draft:new");
    } catch {
      /* ignore */
    }
  }, [store, stream]);

  const isLoading = stream.isStreamingOrLoading || isLoadingConversation;

  return {
    input,
    streamingState: stream.streamingState,
    stage: stream.stage,
    stageDetail: stream.stageDetail,
    pendingMessage: stream.pendingMessage,
    messages,
    isLoading,
    isStreaming: stream.isStreamingOrLoading,
    querySettings,
    setQuerySettings,
    scrollRef,
    scrollAnchorRef,
    showJumpPill,
    jumpToLatest,
    inputRef,
    handleInputChange,
    handleSubmit,
    handleStop: stream.handleStop,
    handleRegenerate: stream.handleRegenerate,
    handleRetry: stream.handleRetry,
    handleSuggestionClick,
    handleEditMessage,
    handleNewConversation,
    queuedMessage: stream.queuedMessage,
    queueMessage: stream.queueMessage,
    clearQueue: stream.clearQueue,
    ...images,
  };
}
