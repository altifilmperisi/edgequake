"use client";

import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";
import {
  Check,
  Copy,
  Network,
  RefreshCw,
  ThumbsDown,
  ThumbsUp,
} from "lucide-react";
import { useTranslation } from "react-i18next";

interface MessageActionsProps {
  copied: boolean;
  onCopy: () => void;
  onRegenerate?: () => void;
  onShowOnGraph?: () => void;
  onFeedback?: (rating: "up" | "down" | null) => void;
  feedback?: "up" | "down" | null;
  isLast?: boolean;
  isVisible?: boolean;
  stopped?: boolean;
  onContinue?: () => void;
}

export function MessageActions({
  copied,
  onCopy,
  onRegenerate,
  onShowOnGraph,
  onFeedback,
  feedback,
  isLast,
  isVisible = true,
  stopped,
  onContinue,
}: MessageActionsProps) {
  const { t } = useTranslation();

  return (
    <div
      className={cn(
        "flex items-center gap-1 pt-2 transition-opacity duration-200",
        isVisible ? "opacity-100" : "opacity-0 group-hover:opacity-100 group-focus-within:opacity-100",
      )}
      data-testid="query-message-actions"
    >
      {stopped && onContinue ? (
        <Button
          type="button"
          variant="outline"
          size="sm"
          className="h-7 text-xs mr-2"
          onClick={onContinue}
          data-testid="query-continue"
        >
          {t("query.continue", "Continue")}
        </Button>
      ) : null}

      {onShowOnGraph ? (
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="sm"
              className="h-7 px-2 text-xs gap-1"
              onClick={onShowOnGraph}
              data-testid="show-on-graph"
            >
              <Network className="h-3.5 w-3.5" />
              <span className="hidden sm:inline">
                {t("query.showOnGraph", "Show on graph")}
              </span>
            </Button>
          </TooltipTrigger>
          <TooltipContent>
            {t(
              "query.showOnGraphHint",
              "Highlight answer entities on the knowledge graph",
            )}
          </TooltipContent>
        </Tooltip>
      ) : null}

      <Tooltip>
        <TooltipTrigger asChild>
          <Button
            variant="ghost"
            size="icon"
            className={cn("h-7 w-7", copied && "text-green-600")}
            onClick={onCopy}
            aria-label={t("common.copy", "Copy")}
          >
            {copied ? (
              <Check className="h-3.5 w-3.5" />
            ) : (
              <Copy className="h-3.5 w-3.5" />
            )}
          </Button>
        </TooltipTrigger>
        <TooltipContent>
          {copied ? t("common.copied", "Copied!") : t("common.copy", "Copy")}
        </TooltipContent>
      </Tooltip>

      {isLast && onRegenerate ? (
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon"
              className="h-7 w-7"
              onClick={onRegenerate}
              aria-label={t("query.regenerate", "Regenerate")}
            >
              <RefreshCw className="h-3.5 w-3.5" />
            </Button>
          </TooltipTrigger>
          <TooltipContent>
            {t("query.regenerate", "Regenerate")}
          </TooltipContent>
        </Tooltip>
      ) : null}

      {onFeedback ? (
        <>
          <Button
            variant="ghost"
            size="icon"
            className={cn("h-7 w-7", feedback === "up" && "text-primary")}
            onClick={() => onFeedback(feedback === "up" ? null : "up")}
            aria-label={t("query.feedback.up", "Helpful")}
            data-testid="query-feedback-up"
          >
            <ThumbsUp className="h-3.5 w-3.5" />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            className={cn("h-7 w-7", feedback === "down" && "text-destructive")}
            onClick={() => onFeedback(feedback === "down" ? null : "down")}
            aria-label={t("query.feedback.down", "Not helpful")}
            data-testid="query-feedback-down"
          >
            <ThumbsDown className="h-3.5 w-3.5" />
          </Button>
        </>
      ) : null}
    </div>
  );
}
