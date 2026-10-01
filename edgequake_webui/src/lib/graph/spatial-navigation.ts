/**
 * Spatial keyboard navigation for the graph canvas.
 *
 * First principle: an arrow key means a *screen direction*. From the selected
 * node we move to the best node in that direction, preferring nodes it is
 * connected to (so the keyboard walks the graph, not the page), and falling
 * back to any node when no neighbour lies that way.
 */

export type ArrowDirection = "up" | "down" | "left" | "right";

export interface NavPoint {
  id: string;
  /** Screen position in px (y grows downward). */
  x: number;
  y: number;
}

/** Half-angle of the search cone around the requested direction. */
const CONE_HALF_ANGLE = Math.PI / 3; // 60°

const UNIT: Record<ArrowDirection, { x: number; y: number }> = {
  up: { x: 0, y: -1 },
  down: { x: 0, y: 1 },
  left: { x: -1, y: 0 },
  right: { x: 1, y: 0 },
};

/** Lower is better; `null` when the candidate is outside the cone. */
function score(from: NavPoint, to: NavPoint, direction: ArrowDirection): number | null {
  const dx = to.x - from.x;
  const dy = to.y - from.y;
  const dist = Math.hypot(dx, dy);
  if (dist < 1e-6) return null;

  const u = UNIT[direction];
  const cos = (dx * u.x + dy * u.y) / dist;
  const angle = Math.acos(Math.max(-1, Math.min(1, cos)));
  if (angle > CONE_HALF_ANGLE) return null;

  // Penalise off-axis candidates so "right" prefers the node truly to the right.
  return dist * (1 + 1.5 * (angle / CONE_HALF_ANGLE));
}

function best(
  from: NavPoint,
  candidates: NavPoint[],
  direction: ArrowDirection,
): string | null {
  let bestId: string | null = null;
  let bestScore = Infinity;
  for (const c of candidates) {
    if (c.id === from.id) continue;
    const s = score(from, c, direction);
    if (s !== null && s < bestScore) {
      bestScore = s;
      bestId = c.id;
    }
  }
  return bestId;
}

/**
 * Next node when pressing `direction` on `current`.
 * @param neighbours ids connected to `current` (tried first)
 * @param all every navigable node (fallback)
 */
export function pickNodeInDirection(
  current: NavPoint,
  direction: ArrowDirection,
  neighbours: Set<string>,
  all: NavPoint[],
): string | null {
  const linked = all.filter((p) => neighbours.has(p.id));
  return best(current, linked, direction) ?? best(current, all, direction);
}

/** Entry point when nothing is selected: the node closest to the canvas centre. */
export function pickCentralNode(
  all: NavPoint[],
  centre: { x: number; y: number },
): string | null {
  let bestId: string | null = null;
  let bestDist = Infinity;
  for (const p of all) {
    const d = Math.hypot(p.x - centre.x, p.y - centre.y);
    if (d < bestDist) {
      bestDist = d;
      bestId = p.id;
    }
  }
  return bestId;
}
