/**
 * @module entity-type-colors
 * @description Canonical entity-type color palette + resolver (SPEC-102).
 *
 * WHY: Graph surfaces previously owned divergent TYPE_COLORS maps and an
 * incomplete default palette. Workspace `entity_type_colors` overrides merge
 * here so entity-type mode stays consistent everywhere.
 *
 * @implements SPEC-102 LAW-102-1 / LAW-102-3 / LAW-102-5
 * @implements FEAT-102
 */

/** Max color overrides persisted per workspace (matches entity_types cap). */
export const MAX_ENTITY_TYPE_COLORS = 50;

/**
 * Colour-blind safe shape markers (SPEC-155 W5). Pair with colour so unknown
 * / CVD-impaired viewers can still tell types apart. Okabe–Ito-style redundancy.
 */
export type EntityTypeShape =
  | "circle"
  | "square"
  | "diamond"
  | "triangle"
  | "hexagon"
  | "star"
  | "pentagon"
  | "cross";

const SHAPE_CYCLE: EntityTypeShape[] = [
  "circle",
  "square",
  "diamond",
  "triangle",
  "hexagon",
  "star",
  "pentagon",
  "cross",
];

/** Known types get stable shapes; unknown types hash into SHAPE_CYCLE. */
export const ENTITY_TYPE_SHAPES: Record<string, EntityTypeShape> = {
  PERSON: "circle",
  CREATURE: "star",
  ORGANIZATION: "square",
  LOCATION: "diamond",
  EVENT: "triangle",
  CONCEPT: "hexagon",
  METHOD: "pentagon",
  CONTENT: "cross",
  DATA: "square",
  ARTIFACT: "diamond",
  NATURALOBJECT: "hexagon",
  OTHER: "cross",
  TECHNOLOGY: "triangle",
  DOCUMENT: "square",
  PRODUCT: "circle",
  DATE: "diamond",
  DRAWING: "star",
  TABLE: "square",
  EQUATION: "cross",
  LAW: "pentagon",
  REGULATION: "pentagon",
  MACHINE: "hexagon",
  COMPONENT: "triangle",
  DEFECT: "star",
  MEASUREMENT: "diamond",
  PROCESS: "circle",
  MATERIAL: "square",
  DEFAULT: "circle",
};

/**
 * Canonical default palette (hex). Keys are UPPERCASE entity types.
 * Includes Rust `default_entity_types()` + common presets + multimodal types.
 */
export const ENTITY_TYPE_COLORS: Record<string, string> = {
  PERSON: '#3b82f6', // blue-500
  CREATURE: '#14b8a6', // teal-500
  ORGANIZATION: '#10b981', // emerald-500
  LOCATION: '#f59e0b', // amber-500
  EVENT: '#ef4444', // red-500
  CONCEPT: '#8b5cf6', // violet-500
  METHOD: '#a855f7', // purple-500
  CONTENT: '#6366f1', // indigo-500
  DATA: '#0ea5e9', // sky-500
  ARTIFACT: '#78716c', // stone-500
  NATURALOBJECT: '#84cc16', // lime-500
  OTHER: '#a1a1aa', // zinc-400 — distinct from DEFAULT fallback
  TECHNOLOGY: '#06b6d4', // cyan-500
  DOCUMENT: '#6366f1', // indigo-500
  PRODUCT: '#f97316', // orange-500
  DATE: '#eab308', // yellow-500
  DRAWING: '#475569', // slate-600
  TABLE: '#0d9488', // teal-600
  EQUATION: '#7c3aed', // violet-600
  LAW: '#64748b', // slate-500
  REGULATION: '#64748b', // slate-500
  MACHINE: '#0891b2', // cyan-600
  COMPONENT: '#0284c7', // sky-600
  DEFECT: '#dc2626', // red-600
  MEASUREMENT: '#ca8a04', // yellow-600
  PROCESS: '#9333ea', // purple-600
  MATERIAL: '#b45309', // amber-700
  DEFAULT: '#94a3b8', // slate-400
};

/** ReDoS-safe: fixed-length alternation only (#RGB | #RRGGBB). */
const HEX_COLOR_RE = /^#(?:[0-9a-fA-F]{6}|[0-9a-fA-F]{3})$/;

/**
 * Normalize an entity type key: trim, UPPERCASE, spaces/hyphens → underscores.
 */
export function normalizeEntityTypeKey(raw: string | undefined | null): string {
  if (!raw) return '';
  return raw.trim().toUpperCase().replace(/[\s-]+/g, '_');
}

function hashToShapeIndex(key: string): number {
  let h = 0;
  for (let i = 0; i < key.length; i++) {
    h = (h * 31 + key.charCodeAt(i)) >>> 0;
  }
  return h % SHAPE_CYCLE.length;
}

/**
 * Resolve a distinct shape for an entity type (colour-blind fallback).
 * Unknown types get a deterministic shape from the key hash.
 */
export function resolveEntityTypeShape(
  entityType: string | undefined | null,
): EntityTypeShape {
  const key = normalizeEntityTypeKey(entityType);
  if (!key) return ENTITY_TYPE_SHAPES.DEFAULT ?? 'circle';
  if (ENTITY_TYPE_SHAPES[key]) return ENTITY_TYPE_SHAPES[key]!;
  return SHAPE_CYCLE[hashToShapeIndex(key)]!;
}

/** True when value is `#RGB` or `#RRGGBB` (case-insensitive hex digits). */
export function isValidEntityTypeHex(value: string | undefined | null): boolean {
  if (!value) return false;
  return HEX_COLOR_RE.test(value.trim());
}

/**
 * Expand `#RGB` → `#RRGGBB` and lowercase hex digits for stable storage.
 * Returns null when invalid.
 */
export function canonicalizeEntityTypeHex(
  value: string | undefined | null,
): string | null {
  if (!value) return null;
  const trimmed = value.trim();
  if (!HEX_COLOR_RE.test(trimmed)) return null;
  const body = trimmed.slice(1).toLowerCase();
  if (body.length === 3) {
    return `#${body
      .split('')
      .map((c) => c + c)
      .join('')}`;
  }
  return `#${body}`;
}

/**
 * Resolve display color: override → default palette → DEFAULT.
 *
 * @implements SPEC-102 LAW-102-1
 */
export function resolveEntityTypeColor(
  entityType: string | undefined,
  overrides?: Record<string, string> | null,
): string {
  const key = normalizeEntityTypeKey(entityType);
  if (!key) return ENTITY_TYPE_COLORS.DEFAULT;

  if (overrides) {
    const overrideRaw =
      overrides[key] ??
      overrides[entityType!] ??
      Object.entries(overrides).find(
        ([k]) => normalizeEntityTypeKey(k) === key,
      )?.[1];
    const canonical = canonicalizeEntityTypeHex(overrideRaw);
    if (canonical) return canonical;
  }

  return ENTITY_TYPE_COLORS[key] ?? ENTITY_TYPE_COLORS.DEFAULT;
}

/**
 * Back-compat alias used across graph components.
 * Prefer {@link resolveEntityTypeColor} when overrides are available.
 */
export function getEntityTypeColor(entityType: string | undefined): string {
  return resolveEntityTypeColor(entityType);
}

/**
 * Normalize a color map for persistence: UPPERCASE keys, canonical hex,
 * drop invalids, cap at {@link MAX_ENTITY_TYPE_COLORS}.
 */
export function mergeEntityTypeColorMap(
  input: Record<string, string> | null | undefined,
): Record<string, string> {
  if (!input) return {};
  const out: Record<string, string> = {};
  for (const [rawKey, rawVal] of Object.entries(input)) {
    if (Object.keys(out).length >= MAX_ENTITY_TYPE_COLORS) break;
    const key = normalizeEntityTypeKey(rawKey);
    if (!key || key === 'DEFAULT') continue;
    const hex = canonicalizeEntityTypeHex(rawVal);
    if (!hex) continue;
    if (!(key in out)) out[key] = hex;
  }
  return out;
}

/**
 * Drop overrides that equal the default palette (keeps metadata small).
 */
export function stripDefaultOverrides(
  overrides: Record<string, string> | null | undefined,
): Record<string, string> {
  const merged = mergeEntityTypeColorMap(overrides);
  const out: Record<string, string> = {};
  for (const [key, hex] of Object.entries(merged)) {
    const def = ENTITY_TYPE_COLORS[key];
    if (def && def.toLowerCase() === hex.toLowerCase()) continue;
    out[key] = hex;
  }
  return out;
}
