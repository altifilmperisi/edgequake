"use client";

import { cn } from "@/lib/utils";
import { useEffect, type ReactNode } from "react";
import { Panel, usePanelRef } from "react-resizable-panels";

export interface CollapsingPanelProps {
  id: string;
  defaultSize: string | number;
  minSize: number;
  collapsedSize?: number;
  /** When true, imperatively collapse to collapsedSize (px). */
  collapsed: boolean;
  /**
   * When set (and not collapsed), imperatively resize to this percentage.
   * Needed because defaultSize is mount-only in react-resizable-panels.
   */
  sizePercent?: number;
  className?: string;
  children: ReactNode;
}

/**
 * Panel that syncs our zone-collapsed state to react-resizable-panels'
 * imperative collapse API — otherwise a railed zone keeps its % width
 * and paints a large empty void next to the rail.
 */
export function CollapsingPanel({
  id,
  defaultSize,
  minSize,
  collapsedSize = 28,
  collapsed,
  sizePercent,
  className,
  children,
}: CollapsingPanelProps) {
  const panelRef = usePanelRef();

  useEffect(() => {
    const panel = panelRef.current;
    if (!panel) return;
    if (collapsed) {
      if (!panel.isCollapsed()) panel.collapse();
      return;
    }
    if (panel.isCollapsed()) {
      panel.expand();
    }
    if (typeof sizePercent === "number" && Number.isFinite(sizePercent)) {
      const current = panel.getSize().asPercentage;
      if (Math.abs(current - sizePercent) > 1.5) {
        panel.resize(`${sizePercent}%`);
      }
    }
  }, [collapsed, sizePercent, panelRef]);

  return (
    <Panel
      id={id}
      panelRef={panelRef}
      defaultSize={defaultSize}
      minSize={collapsed ? collapsedSize : minSize}
      className={cn("flex h-full min-h-0 min-w-0 flex-col", className)}
      collapsible
      collapsedSize={collapsedSize}
    >
      {children}
    </Panel>
  );
}

export default CollapsingPanel;
