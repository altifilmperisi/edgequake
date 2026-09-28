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

import { Button } from '@/components/ui/button';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import { PageSyncModeControl } from '@/components/documents/page-sync-mode-control';
import type { PageSyncMode } from '@/lib/documents/page-sync-mode';
import { cn } from '@/lib/utils';
import {
  Columns2,
  PanelLeftClose,
  PanelRightClose,
} from 'lucide-react';
import { useCallback, useEffect, useRef, useState } from 'react';

type ViewMode = 'side-by-side' | 'pdf-only' | 'markdown-only';

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

  return (
    <div data-testid="side-by-side-viewer" className={cn('flex flex-col min-h-0', className)}>
      <div className="flex items-center justify-end gap-1 px-2 py-1 border-b bg-muted/20">
        <TooltipProvider>
          {showSync ? (
            <span className="mr-1 inline-flex">
              <PageSyncModeControl
                mode={syncMode}
                onModeChange={onSyncModeChange!}
                available={syncAvailable}
                compact
              />
            </span>
          ) : null}
          <div className="flex items-center gap-0.5 bg-background rounded p-0.5">
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant={mode === 'pdf-only' ? 'secondary' : 'ghost'}
                  size="icon"
                  className="h-6 w-6"
                  onClick={() => handleModeChange('pdf-only')}
                >
                  <PanelRightClose className="h-3.5 w-3.5" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>PDF Only</TooltipContent>
            </Tooltip>

            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant={mode === 'side-by-side' ? 'secondary' : 'ghost'}
                  size="icon"
                  className="h-6 w-6"
                  onClick={() => handleModeChange('side-by-side')}
                >
                  <Columns2 className="h-3.5 w-3.5" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>Split View</TooltipContent>
            </Tooltip>

            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant={mode === 'markdown-only' ? 'secondary' : 'ghost'}
                  size="icon"
                  className="h-6 w-6"
                  onClick={() => handleModeChange('markdown-only')}
                >
                  <PanelLeftClose className="h-3.5 w-3.5" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>Markdown Only</TooltipContent>
            </Tooltip>
          </div>
        </TooltipProvider>
      </div>

      <div
        ref={containerRef}
        className="flex flex-1 min-h-0"
        style={height ? { height: `${height}px` } : undefined}
      >
        {(mode === 'pdf-only' || mode === 'side-by-side') && (
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

        {(mode === 'markdown-only' || mode === 'side-by-side') && (
          <div
            className={cn(
              'flex flex-col overflow-hidden',
              mode === 'markdown-only' ? 'w-full' : 'flex-1',
            )}
          >
            <div
              className="flex-1 min-h-0 overflow-y-auto overflow-x-hidden"
              data-testid="md-scroll-container"
            >
              {rightPanel}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

export default SideBySideViewer;
