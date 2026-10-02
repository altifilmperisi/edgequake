/**
 * @module ThinkingDisplay
 * @description Chain-of-thought reasoning display component.
 * Shows LLM thinking process with collapsible sections.
 *
 * @implements FEAT0734 - Chain-of-thought display
 * @implements FEAT0750 - Collapsible thinking sections
 *
 * @enforces BR0105 - Thinking shows progressive indicators
 * @enforces BR0750 - Default collapsed for completed responses
 */
'use client';

import { cn } from '@/lib/utils';
import { Brain, ChevronDown, ChevronRight } from 'lucide-react';
import { memo, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { parseCOTContent } from '@/lib/query/parse-cot-streaming';

interface ThinkingDisplayProps {
  content: string;
  defaultExpanded?: boolean;
  className?: string;
}

// Re-export streaming-aware parser (SPEC-155) — SSOT in lib/query
export {
  parseCOTContent,
  parseCOTStreaming,
  type ParsedCotStreaming,
} from "@/lib/query/parse-cot-streaming";

/**
 * Component to display LLM chain-of-thought reasoning in a collapsible section.
 */
export const ThinkingDisplay = memo(function ThinkingDisplay({
  content,
  defaultExpanded = false,
  className,
}: ThinkingDisplayProps) {
  const { t } = useTranslation();
  const [isExpanded, setIsExpanded] = useState(defaultExpanded);

  const parsedContent = useMemo(() => parseCOTContent(content), [content]);

  // If no thinking content, just return the response without wrapper
  if (parsedContent.thinking.length === 0) {
    return null;
  }

  return (
    <div className={cn('rounded-lg border border-border bg-muted/50', className)}>
      {/* Collapsible thinking section */}
      <button
        onClick={() => setIsExpanded(!isExpanded)}
        className="flex items-center gap-2 w-full p-3 text-left hover:bg-muted/80 transition-colors rounded-t-lg"
      >
        {isExpanded ? (
          <ChevronDown className="h-4 w-4 text-muted-foreground" />
        ) : (
          <ChevronRight className="h-4 w-4 text-muted-foreground" />
        )}
        <Brain className="h-4 w-4 text-muted-foreground" />
        <span className="text-sm font-medium text-muted-foreground">
          {t('query.thinking', 'Reasoning Process')}
        </span>
        <span className="text-xs text-muted-foreground ml-auto">
          {parsedContent.thinking.length} {t('query.thinkingSteps', 'step(s)')}
        </span>
      </button>

      {/* Expanded thinking content */}
      {isExpanded && (
        <div className="p-3 pt-0 space-y-3">
          {parsedContent.thinking.map((block, index) => (
            <div
              key={index}
              className="pl-6 border-l-2 border-muted-foreground/30"
            >
              <p className="text-sm text-muted-foreground whitespace-pre-wrap">
                {block}
              </p>
            </div>
          ))}
        </div>
      )}
    </div>
  );
});

/**
 * Wrapper component that renders both thinking and response sections.
 */
export const COTRenderer = memo(function COTRenderer({
  content,
  renderResponse,
  defaultThinkingExpanded = false,
  className,
}: {
  content: string;
  renderResponse: (response: string) => React.ReactNode;
  defaultThinkingExpanded?: boolean;
  className?: string;
}) {
  const parsedContent = useMemo(() => parseCOTContent(content), [content]);

  return (
    <div className={cn('space-y-4', className)}>
      {/* Thinking section (if any) */}
      {parsedContent.thinking.length > 0 && (
        <ThinkingDisplay
          content={content}
          defaultExpanded={defaultThinkingExpanded}
        />
      )}

      {/* Main response */}
      {parsedContent.response && renderResponse(parsedContent.response)}
    </div>
  );
});

export default ThinkingDisplay;
