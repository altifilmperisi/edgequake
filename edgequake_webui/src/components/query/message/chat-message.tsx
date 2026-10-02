"use client";

import { memo } from "react";
import { AssistantMessage } from "./assistant-message";
import { UserMessage } from "./user-message";
import type { ChatMessageProps } from "./types";

export type { ChatMessageData, ChatMessageProps } from "./types";

export const ChatMessage = memo(function ChatMessage({
  message,
  isLast,
  onCopy,
  onRegenerate,
  onRetry,
  onContinue,
  onEdit,
  onFeedback,
  showMetadata = true,
  stage,
  stageDetail,
}: ChatMessageProps) {
  if (message.role === "user") {
    return <UserMessage message={message} onEdit={onEdit} />;
  }

  return (
    <AssistantMessage
      message={message}
      isLast={isLast}
      onCopy={onCopy}
      onRegenerate={onRegenerate}
      onRetry={onRetry}
      onContinue={onContinue}
      onFeedback={onFeedback}
      showMetadata={showMetadata}
      stage={stage}
      stageDetail={stageDetail}
    />
  );
});

export default ChatMessage;
