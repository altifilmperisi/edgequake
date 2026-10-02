/**
 * @module SideBySideViewer
 * @description Split-panel layout for viewing PDF and Markdown side-by-side.
 *
 * @implements SPEC-002 - Document Viewer with side-by-side display
 * @implements SPEC-143 - Explicit directional page sync control
 * @implements FEAT0731 - Split-panel layout with resizable divider
 * @implements FEAT0732 - View mode toggle (PDF only, Markdown only, side-by-side)
 * @implements FEAT0733 - Panel synchronization controls
 */
'use client';

import { PageSyncModeControl } from '@/components/documents/page-sync-mode-control';
import {
  ViewModeToggle,
  type ViewMode,
} from '@/components/documents/view-mode-toggle';
import type { PageSyncMode } from '@/lib/documents/page-sync-mode';
import { cn } from '@/lib/utils';
import { useCallback, useEffect, useRef, useState } from 'react';

interface SideBySideViewerProps {
  leftPanel: React.ReactNode;
  rightPanel: React.ReactNode;
  className?: string;
  height?: number;
  initialMode?: ViewMode;
  leftTitle?: string;
  rightTitle?: string;
  onModeChange?: (mode: ViewMode) => void;
  /** SPEC-143: explicit sync direction. */
  syncMode?: PageSyncMode;
  /** SPEC-143: set sync direction. */
  onSyncModeChange?: (mode: PageSyncMode) => void;
  /** SPEC-143: disable sync when document has no page markers. */
  syncAvailable?: boolean;
}

export function SideBySideViewer({
  leftPanel,
  rightPanel,
  className,
  height,
  initialMode = 'side-by-side',
  rightTitle,
  onModeChange,
  syncMode = 'pdf-to-md',
  onSyncModeChange,
  syncAvailable = true,
}: SideBySideViewerProps) {
  const [mode, setMode] = useState<ViewMode>(initialMode);
  const [leftWidth, setLeftWidth] = useState(50);
  const [isDragging, setIsDragging] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);
  const startX = useRef(0);
  const startWidth = useRef(50);

  const handleModeChange = useCallback(
    (newMode: ViewMode) => {
      setMode(newMode);
      onModeChange?.(newMode);
    },
    [onModeChange],
  );

  const handleMouseDown = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      setIsDragging(true);
      startX.current = e.clientX;
      startWidth.current = leftWidth;
      document.body.style.cursor = 'col-resize';
      document.body.style.userSelect = 'none';
    },
    [leftWidth],
  );

  const handleMouseMove = useCallback(
    (e: MouseEvent) => {
      if (!isDragging || !containerRef.current) return;
      const containerRect = containerRef.current.getBoundingClientRect();
      const containerWidth = containerRect.width;
      const deltaX = e.clientX - startX.current;
      const deltaPercent = (deltaX / containerWidth) * 100;
      const newWidth = Math.min(75, Math.max(25, startWidth.current + deltaPercent));
      setLeftWidth(newWidth);
    },
    [isDragging],
  );

  const handleMouseUp = useCallback(() => {
    setIsDragging(false);
    document.body.style.cursor = '';
    document.body.style.userSelect = '';
  }, []);

  useEffect(() => {
    if (typeof window === 'undefined' || !isDragging) return;
    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
    return () => {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };
  }, [handleMouseMove, handleMouseUp, isDragging]);

  const showSync = mode === 'side-by-side' && onSyncModeChange != null;
  const showPdf = mode === 'pdf-only' || mode === 'side-by-side';
  const showMarkdown = mode === 'markdown-only' || mode === 'side-by-side';

  return (
    <div
      data-testid="side-by-side-viewer"
      className={cn('relative flex flex-col min-h-0', className)}
    >
      <div
        ref={containerRef}
        className="flex flex-1 min-h-0"
        style={height ? { height: `${height}px` } : undefined}
      >
        {showPdf && (
          <div
            className={cn(
              'flex flex-col border-r overflow-hidden',
              mode === 'pdf-only' ? 'w-full' : '',
            )}
            style={mode === 'side-by-side' ? { width: `${leftWidth}%` } : undefined}
          >
            <div className="flex-1 overflow-hidden">{leftPanel}</div>
          </div>
        )}

        {mode === 'side-by-side' && (
          <div
            className={cn(
              'w-1 bg-border hover:bg-primary/30 cursor-col-resize transition-colors',
              'flex items-center justify-center',
              isDragging && 'bg-primary/50',
            )}
            onMouseDown={handleMouseDown}
          >
            <div className="h-8 w-0.5 rounded-full bg-muted-foreground/20" />
          </div>
        )}

        {showMarkdown && (
          <div
            className={cn(
              'flex flex-col overflow-hidden',
              mode === 'markdown-only' ? 'w-full' : 'flex-1',
            )}
          >
            {/* Pane header: same 48px row as the PDF toolbar, so the two panes
                read as one toolbar instead of two stacked bars. */}
            <div
              className="flex h-12 shrink-0 items-center justify-between gap-2 border-b bg-muted/30 px-3"
              data-testid="markdown-pane-header"
            >
              <span className="truncate text-xs font-medium text-muted-foreground">
                {rightTitle ?? 'Markdown'}
              </span>
              <div className="flex shrink-0 items-center gap-2">
                {showSync ? (
                  <PageSyncModeControl
                    mode={syncMode}
                    onModeChange={onSyncModeChange!}
                    available={syncAvailable}
                    compact
                  />
                ) : null}
                <ViewModeToggle mode={mode} onModeChange={handleModeChange} />
              </div>
            </div>
            <div
              className="flex-1 min-h-0 overflow-y-auto overflow-x-hidden"
              data-testid="md-scroll-container"
            >
              {rightPanel}
            </div>
          </div>
        )}
      </div>

      {/* PDF-only hides the Markdown header, so the switcher floats instead of
          leaving the user without a way back to split view. */}
      {mode === 'pdf-only' && (
        <div className="absolute bottom-3 right-3 z-20 rounded-lg border bg-background/95 p-0.5 shadow-md backdrop-blur">
          <ViewModeToggle mode={mode} onModeChange={handleModeChange} />
        </div>
      )}
    </div>
  );
}

export default SideBySideViewer;
