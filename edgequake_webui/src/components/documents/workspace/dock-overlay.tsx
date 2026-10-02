"use client";

import { cn } from "@/lib/utils";
import type { DockEdge } from "@/lib/documents/workspace-layout";
import { useDroppable } from "@dnd-kit/core";

const EDGES: DockEdge[] = ["left", "right", "top", "bottom", "center"];

export interface DockOverlayProps {
  zoneId: string;
  active: boolean;
}

function DockTarget({
  zoneId,
  edge,
}: {
  zoneId: string;
  edge: DockEdge;
}) {
  const id = `${zoneId}::${edge}`;
  const { setNodeRef, isOver } = useDroppable({ id, data: { zoneId, edge } });

  const positionClass =
    edge === "left"
      ? "left-0 top-0 h-full w-1/3"
      : edge === "right"
        ? "right-0 top-0 h-full w-1/3"
        : edge === "top"
          ? "left-0 top-0 h-1/3 w-full"
          : edge === "bottom"
            ? "left-0 bottom-0 h-1/3 w-full"
            : "left-1/4 top-1/4 h-1/2 w-1/2";

  return (
    <div
      ref={setNodeRef}
      className={cn(
        "absolute z-20 rounded-sm transition-colors",
        positionClass,
        isOver
          ? "bg-sky-500/35 ring-2 ring-sky-500"
          : "bg-sky-500/10 hover:bg-sky-500/20",
      )}
      data-testid={`workspace-dock-target-${edge}`}
      data-zone={zoneId}
      data-edge={edge}
      aria-hidden="true"
    />
  );
}

/**
 * 5-target dock overlay shown on the zone under the pointer while dragging.
 */
export function DockOverlay({ zoneId, active }: DockOverlayProps) {
  if (!active) return null;
  return (
    <div
      className="pointer-events-auto absolute inset-0 z-10"
      data-testid={`workspace-dock-overlay-${zoneId}`}
    >
      {EDGES.map((edge) => (
        <DockTarget key={edge} zoneId={zoneId} edge={edge} />
      ))}
    </div>
  );
}

export default DockOverlay;
