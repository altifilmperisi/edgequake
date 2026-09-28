/**
 * @fileoverview Smart content renderer that adapts to document MIME type
 *
 * @implements FEAT1072 - MIME type-aware rendering
 * @implements FEAT1073 - Text highlight and scroll-to
 *
 * @see UC1503 - User views document with appropriate renderer
 * @see UC1504 - User navigates to highlighted citation
 *
 * @enforces BR1072 - Renderer selection by MIME type
 * @enforces BR1073 - Smooth scroll to highlighted content
 */
// Smart content renderer that adapts to document MIME type
'use client';

import { StreamingMarkdownRenderer } from '@/components/query/markdown';
import {
    VIRTUALIZATION_CHAR_THRESHOLD,
    VirtualizedMarkdownContent,
} from '@/components/query/markdown/VirtualizedMarkdownContent';
import { Skeleton } from '@/components/ui/skeleton';
import { useMarkdownPageObserver } from '@/hooks/use-markdown-page-observer';
import type { PageSyncDriver } from '@/hooks/use-page-sync-controller';
import { rewriteMarkdownMmAssetUrls } from '@/lib/api/edgequake/documents';
import {
    findHighlightIndex,
    resolveMarkdownHighlightRange,
    type HighlightLineRange,
} from '@/lib/utils/markdown-highlight';
import { injectPageAnchors } from '@/lib/utils/page-markers';
import type { Document } from '@/types';
import {
  Suspense,
  useEffect,
  useMemo,
  useRef,
  type RefObject,
} from 'react';
import { CodeRenderer } from './code-renderer';
import { DocumentContentEmptyState } from './document-content-empty-state';
import { PlainTextRenderer } from './plain-text-renderer';

interface ContentRendererProps {
  document: Document;
  highlightText?: string;
  startLine?: number;
  endLine?: number;
  /** SPEC-143: active page from sync controller. */
  activePage?: number;
  /** SPEC-143: any directional sync selected (page badge). */
  syncEnabled?: boolean;
  /** SPEC-143: markdown follows PDF (pdf-to-md). */
  followMarkdown?: boolean;
  /** SPEC-143: report page when user scrolls markdown (always records). */
  onPageFromMd?: (page: number) => void;
  /** SPEC-143: skip MD auto-scroll when markdown is the driver. */
  syncDriver?: PageSyncDriver;
  /** SPEC-143: user gesture ownership for the sync controller. */
  onMdGestureStart?: () => void;
  onMdGestureEnd?: () => void;
  /** Optional explicit markdown scrollport (side-by-side pane). */
  mdScrollRootRef?: RefObject<HTMLElement | null>;
}

export function ContentRenderer({
  document,
  highlightText,
  startLine,
  endLine,
  activePage,
  syncEnabled = false,
  followMarkdown = false,
  onPageFromMd,
  syncDriver = 'none',
  onMdGestureStart,
  onMdGestureEnd,
  mdScrollRootRef,
}: ContentRendererProps) {
  const contentRef = useRef<HTMLDivElement>(null);

  const renderer = useMemo(() => {
    return getRendererForDocument(
      document,
      highlightText,
      startLine,
      endLine,
      followMarkdown ? activePage : undefined,
    );
  }, [document, highlightText, startLine, endLine, followMarkdown, activePage]);

  useMarkdownPageObserver({
    containerRef: contentRef,
    scrollRootRef: mdScrollRootRef,
    enabled: Boolean(onPageFromMd),
    onPage: (page) => onPageFromMd?.(page),
    scrollToPage: followMarkdown ? activePage ?? null : null,
    skipScroll: !followMarkdown || syncDriver === 'md',
    onGestureStart: onMdGestureStart,
    onGestureEnd: onMdGestureEnd,
  });

  // Citation highlight scroll. Page sync owns the pane scrollport; scrollIntoView
  // would move that pane and its ancestors and undo the page alignment.
  useEffect(() => {
    if (followMarkdown && activePage != null && activePage >= 1) return;
    if ((!highlightText && startLine === undefined) || !contentRef.current) return;

    const timer = setTimeout(() => {
      const container = contentRef.current;
      if (!container) return;

      if (startLine !== undefined) {
        const highlightedBlock = container.querySelector('[data-highlighted="true"]');
        if (highlightedBlock) {
          highlightedBlock.scrollIntoView({ behavior: 'smooth', block: 'center' });
          return;
        }
      }

      const highlightedElements = container.querySelectorAll(
        'mark.highlight-citation, mark.highlight-match',
      );
      if (highlightedElements.length > 0) {
        highlightedElements[0].scrollIntoView({
          behavior: 'smooth',
          block: 'center',
        });
      }
    }, 100);

    return () => clearTimeout(timer);
  }, [highlightText, startLine, endLine, followMarkdown, activePage]);

  return (
    <div ref={contentRef} className="relative px-4 pt-4 pb-16 max-w-4xl">
      {syncEnabled && activePage != null && activePage >= 1 ? (
        <div
          className="sticky top-0 z-10 mb-2 -mx-1 px-2 py-0.5 text-xs text-muted-foreground bg-background/90 backdrop-blur-sm"
          data-testid="md-page-indicator"
          data-page={activePage}
        >
          Page {activePage}
        </div>
      ) : null}
      <Suspense fallback={<ContentSkeleton />}>
        {renderer}
      </Suspense>
    </div>
  );
}

function getRendererForDocument(
  doc: Document,
  highlightText?: string,
  startLine?: number,
  endLine?: number,
  syncPage?: number,
) {
  const mimeType = doc.mime_type?.toLowerCase() || '';
  const fileName = doc.file_name?.toLowerCase() || '';
  // MV-28: rewrite `![…](assets/…)` to document-scoped API URLs so figures render
  // in the detail view (ContentRenderer path — same SSOT as MarkdownViewer).
  // SPEC-143: inject page anchors after asset rewrite so MD↔PDF sync can observe them.
  const content = injectPageAnchors(
    rewriteMarkdownMmAssetUrls(doc.content || doc.content_summary || '', doc.id),
  );

  if (!content.trim()) {
    return <DocumentContentEmptyState />;
  }

  // ---------------------------------------------------------------------------
  // Markdown documents — use token-level highlighting (not HTML injection).
  //
  // WHY: applyLineHighlight wraps lines in <mark>/<span> HTML tags which
  //      DESTROYS markdown structure (headings, lists, tables) because:
  //      1. <mark># Heading</mark> is not a heading (# is inside HTML)
  //      2. The inline HTML handler only renders <br>/<wbr>, not <mark>
  //      3. All other inline HTML is rendered as literal text
  //
  // FIX: Pass highlightLineRange to StreamingMarkdownRenderer which computes
  //      token-to-line mapping and wraps matching tokens in CSS-highlighted divs.
  // ---------------------------------------------------------------------------
  if (
    doc.source_type === 'pdf' ||
    isMarkdown(mimeType) ||
    fileName.endsWith('.md') ||
    fileName.endsWith('.markdown') ||
    hasMarkdownSignature(content)
  ) {
    const highlightLineRange = resolveMarkdownHighlightRange({
      content,
      startLine,
      endLine,
      highlightText,
    });

    // WHY: For very large markdown (e.g. 1000-page PDF), tokenising the
    // entire string freezes the browser. VirtualizedMarkdownContent splits the
    // raw string into ~25 KB chunks — only visible chunks are tokenised.
    // Highlight still applies: each slice gets a local line range (SPEC-135).
    const isLargeDocument = content.length >= VIRTUALIZATION_CHAR_THRESHOLD;

    const markdownArticle = (
      pageContent: string,
      localRange?: HighlightLineRange,
    ) => (
      <article className="
        prose prose-lg dark:prose-invert max-w-none
        prose-headings:font-display prose-headings:font-semibold
        prose-h1:text-4xl prose-h1:mb-6 prose-h1:mt-0
        prose-h2:text-3xl prose-h2:mb-4 prose-h2:mt-6
        prose-h3:text-2xl prose-h3:mb-3 prose-h3:mt-5 prose-h3:scroll-mt-4
        [&>:first-child]:mt-0
        prose-p:text-base prose-p:leading-relaxed prose-p:text-foreground/90
        prose-a:text-primary prose-a:no-underline prose-a:font-medium
        hover:prose-a:underline
        prose-code:bg-muted prose-code:px-1.5 prose-code:py-0.5 
        prose-code:rounded prose-code:text-sm prose-code:font-mono
        prose-code:before:content-none prose-code:after:content-none
        prose-pre:bg-muted/50 prose-pre:border prose-pre:rounded-xl
        prose-pre:p-4 prose-pre:overflow-x-auto
        prose-blockquote:border-l-4 prose-blockquote:border-primary
        prose-blockquote:bg-muted/30 prose-blockquote:py-2 prose-blockquote:px-4
        prose-blockquote:rounded-r-lg prose-blockquote:italic
        prose-img:rounded-xl prose-img:shadow-lg prose-img:my-4 prose-img:mx-auto prose-img:max-h-[70vh]
        prose-hr:border-border prose-hr:my-8
        prose-table:border prose-table:rounded-lg
        prose-thead:bg-muted
      ">
        <StreamingMarkdownRenderer
          content={pageContent}
          className="text-sm leading-relaxed"
          highlightLineRange={localRange}
          revealPage={syncPage ?? null}
        />
      </article>
    );

    if (isLargeDocument) {
      return (
        <VirtualizedMarkdownContent
          content={content}
          highlightLineRange={highlightLineRange}
          scrollToPage={syncPage ?? null}
        >
          {(pageContent, ctx) => markdownArticle(pageContent, ctx.highlightLineRange)}
        </VirtualizedMarkdownContent>
      );
    }

    return markdownArticle(content, highlightLineRange);
  }

  // ---------------------------------------------------------------------------
  // Non-markdown paths: apply HTML-based highlighting to content
  // ---------------------------------------------------------------------------
  let processedContent = content;
  if (startLine !== undefined && endLine !== undefined) {
    processedContent = applyLineHighlight(processedContent, startLine, endLine);
  } else if (highlightText && processedContent) {
    processedContent = applyTextHighlight(processedContent, highlightText);
  }

  // Code files
  if (isCode(mimeType, fileName)) {
    const language = detectLanguage(mimeType, fileName);
    return (
      <CodeRenderer
        content={processedContent}
        language={language}
        showLineNumbers
      />
    );
  }

  // JSON/Structured data
  if (mimeType === 'application/json' || fileName.endsWith('.json')) {
    try {
      const parsed = JSON.parse(processedContent);
      return (
        <CodeRenderer
          content={JSON.stringify(parsed, null, 2)}
          language="json"
          showLineNumbers
        />
      );
    } catch {
      // Fall through to plain text if JSON parsing fails
    }
  }

  // Fallback: Plain text with smart formatting
  return <PlainTextRenderer content={processedContent} />;
}

// Helper functions
function isMarkdown(mimeType: string): boolean {
  return (
    mimeType.includes('markdown') ||
    mimeType === 'text/markdown' ||
    mimeType === 'text/x-markdown'
  );
}

function hasMarkdownSignature(content: string): boolean {
  if (!content) return false;
  // Check for common markdown patterns
  const markdownPatterns = [
    /^#{1,6}\s+/m,        // Headers
    /\*\*[^*]+\*\*/,      // Bold
    /\*[^*]+\*/,          // Italic
    /\[[^\]]+\]\([^)]+\)/, // Links
    /```[\s\S]*```/,      // Code blocks
    /^\s*[-*+]\s+/m,      // Lists
  ];
  return markdownPatterns.some((pattern) => pattern.test(content));
}

function isCode(mimeType: string, fileName: string): boolean {
  const codeMimeTypes = [
    'text/x-python',
    'text/x-java',
    'text/x-c',
    'text/x-c++',
    'text/javascript',
    'application/javascript',
    'text/typescript',
    'application/typescript',
    'text/x-rust',
    'text/x-go',
    'text/x-ruby',
    'text/x-php',
    'text/x-sql',
    'text/x-sh',
    'text/x-yaml',
    'application/x-yaml',
    'text/css',
    'text/html',
    'application/xml',
    'text/xml',
  ];

  const codeExtensions = [
    '.py', '.js', '.ts', '.tsx', '.jsx', '.java', '.c', '.cpp', '.h', '.hpp',
    '.rs', '.go', '.rb', '.php', '.sql', '.sh', '.bash', '.zsh', '.yaml', '.yml',
    '.css', '.scss', '.sass', '.less', '.html', '.xml', '.toml', '.ini', '.conf',
  ];

  return (
    codeMimeTypes.some((type) => mimeType.includes(type)) ||
    codeExtensions.some((ext) => fileName.endsWith(ext))
  );
}

function detectLanguage(mimeType: string, fileName: string): string {
  // Language mapping
  const mimeToLang: Record<string, string> = {
    'text/x-python': 'python',
    'text/x-java': 'java',
    'text/javascript': 'javascript',
    'application/javascript': 'javascript',
    'text/typescript': 'typescript',
    'application/typescript': 'typescript',
    'text/x-rust': 'rust',
    'text/x-go': 'go',
    'text/x-ruby': 'ruby',
    'text/x-php': 'php',
    'text/x-sql': 'sql',
    'text/x-sh': 'bash',
    'text/x-yaml': 'yaml',
    'application/x-yaml': 'yaml',
    'text/css': 'css',
    'text/html': 'html',
    'application/xml': 'xml',
    'text/xml': 'xml',
  };

  // Try MIME type first
  for (const [mime, lang] of Object.entries(mimeToLang)) {
    if (mimeType.includes(mime)) {
      return lang;
    }
  }

  // Fall back to file extension
  const ext = fileName.split('.').pop()?.toLowerCase();
  const extToLang: Record<string, string> = {
    py: 'python',
    js: 'javascript',
    ts: 'typescript',
    tsx: 'typescript',
    jsx: 'javascript',
    java: 'java',
    c: 'c',
    cpp: 'cpp',
    h: 'c',
    hpp: 'cpp',
    rs: 'rust',
    go: 'go',
    rb: 'ruby',
    php: 'php',
    sql: 'sql',
    sh: 'bash',
    bash: 'bash',
    zsh: 'bash',
    yaml: 'yaml',
    yml: 'yaml',
    css: 'css',
    scss: 'scss',
    sass: 'sass',
    less: 'less',
    html: 'html',
    xml: 'xml',
    json: 'json',
    toml: 'toml',
    ini: 'ini',
    conf: 'bash',
  };

  return ext && extToLang[ext] ? extToLang[ext] : 'text';
}

function ContentSkeleton() {
  return (
    <div className="space-y-4">
      <Skeleton className="h-8 w-3/4" />
      <Skeleton className="h-4 w-full" />
      <Skeleton className="h-4 w-full" />
      <Skeleton className="h-4 w-5/6" />
      <Skeleton className="h-32 w-full mt-6" />
      <Skeleton className="h-4 w-full" />
      <Skeleton className="h-4 w-4/5" />
    </div>
  );
}

/**
 * Highlight specific line range in content using stabilo highlighter style.
 * Wraps each line in a span with data-line-number for scrolling.
 */
function applyLineHighlight(content: string, startLine: number, endLine: number): string {
  const lines = content.split('\n');
  
  return lines.map((line, idx) => {
    const lineNumber = idx + 1;
    const isHighlighted = lineNumber >= startLine && lineNumber <= endLine;
    
    // Escape HTML entities
    const escapedLine = line
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#039;');
    
    if (isHighlighted) {
      return `<mark class="highlight-citation" data-line-number="${lineNumber}">${escapedLine}</mark>`;
    }
    return `<span data-line-number="${lineNumber}">${escapedLine}</span>`;
  }).join('\n');
}

/**
 * Apply text highlighting to content by wrapping matching text in <mark> tags.
 * Uses fuzzy matching to find the best match position.
 */
function applyTextHighlight(content: string, searchText: string): string {
  const matchIndex = findHighlightIndex(content, searchText);
  if (matchIndex === -1) return content;
  
  // Calculate highlight range (show some context around the match)
  const highlightLength = Math.min(searchText.length, 200);
  const start = matchIndex;
  const end = Math.min(start + highlightLength, content.length);
  
  // Wrap the matched text in a highlight mark
  const before = content.slice(0, start);
  const matched = content.slice(start, end);
  const after = content.slice(end);
  
  return `${before}<mark class="highlight-match bg-yellow-200 dark:bg-yellow-800/50 px-0.5 rounded">${matched}</mark>${after}`;
}
