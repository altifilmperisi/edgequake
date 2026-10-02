/**
 * SPEC-155 — Run progress ledger (FE mirror of Rust `services/run_progress.rs`).
 *
 * First principle: fill is done/total per typed task, never a regex on the last
 * message. The server persists the ledger; the client only renders it and
 * clamps against stale polls.
 */

import type { IngestionPhaseId } from "@/lib/pipeline/ingestion-run-view";

export type RunPhaseId = "prepare" | "extract" | "materialize";
export type RunPhaseState = "pending" | "active" | "done";
export type RunTaskId =
  | "pages"
  | "figures"
  | "chunks"
  | "embeddings"
  | "entities"
  | "relationships";
export type RunTaskUnit =
  | "pages"
  | "figures"
  | "chunks"
  | "embeddings"
  | "entities"
  | "relationships";

export interface RunTaskProgress {
  id: RunTaskId;
  unit: RunTaskUnit;
  done: number;
  total: number;
  in_flight?: number | null;
}

export interface RunPhaseProgress {
  id: RunPhaseId;
  state: RunPhaseState;
  tasks: RunTaskProgress[];
  started_at?: string | null;
  finished_at?: string | null;
}

export interface RunProgress {
  seq: number;
  phases: RunPhaseProgress[];
  updated_at?: string | null;
}

const PHASE_ORDER: RunPhaseId[] = ["prepare", "extract", "materialize"];

/** Weighted fill 0–1 for one phase. Done ⇒ 1; pending ⇒ 0; active ⇒ task average. */
export function phaseFill01(phase: RunPhaseProgress | undefined): number {
  if (!phase) return 0;
  if (phase.state === "pending") return 0;
  if (phase.state === "done") return 1;
  const known = (phase.tasks ?? []).filter((t) => t.total > 0);
  if (known.length === 0) return 0.02;
  const sum = known.reduce(
    (acc, t) => acc + Math.min(1, Math.max(0, t.done / t.total)),
    0,
  );
  return Math.min(0.99, sum / known.length);
}

export function phaseFillPct(phase: RunPhaseProgress | undefined): number {
  return Math.round(phaseFill01(phase) * 100);
}

export function findPhase(
  ledger: RunProgress | null | undefined,
  id: RunPhaseId,
): RunPhaseProgress | undefined {
  return ledger?.phases?.find((p) => p.id === id);
}

export function activePhaseId(
  ledger: RunProgress | null | undefined,
): RunPhaseId {
  if (!ledger?.phases?.length) return "prepare";
  const active = [...ledger.phases]
    .filter((p) => p.state === "active")
    .sort((a, b) => PHASE_ORDER.indexOf(b.id) - PHASE_ORDER.indexOf(a.id))[0];
  if (active) return active.id;
  const done = [...ledger.phases]
    .filter((p) => p.state === "done")
    .sort((a, b) => PHASE_ORDER.indexOf(b.id) - PHASE_ORDER.indexOf(a.id))[0];
  if (done) {
    if (done.id === "prepare") return "extract";
    if (done.id === "extract") return "materialize";
    return "materialize";
  }
  return "prepare";
}

/** Map strip phase id → ledger phase (admit has no ledger work). */
export function stripPhaseToLedger(
  phase: IngestionPhaseId,
): RunPhaseId | null {
  if (phase === "admit") return null;
  return phase;
}

/**
 * Caption line for the active phase:
 *   "Prepare · pages 92/92 · figures 3/12"
 *   "Extract · 61/92 chunks, 12 in flight"
 */
export function formatPhaseCaption(
  ledger: RunProgress | null | undefined,
): string | null {
  if (!ledger) return null;
  const id = activePhaseId(ledger);
  const phase = findPhase(ledger, id);
  if (!phase) return null;
  const label =
    id === "prepare" ? "Prepare" : id === "extract" ? "Extract" : "Materialize";
  const parts: string[] = [];
  for (const t of phase.tasks ?? []) {
    if (t.total <= 0) continue;
    if (id === "extract" && t.id === "chunks") {
      const flight =
        typeof t.in_flight === "number" && t.in_flight > 0
          ? `, ${t.in_flight} in flight`
          : "";
      parts.push(`${t.done}/${t.total} chunks${flight}`);
    } else {
      parts.push(`${t.unit} ${t.done}/${t.total}`);
    }
  }
  if (parts.length === 0) return label;
  return `${label} · ${parts.join(" · ")}`;
}

/** Done-segment aria/tooltip summary: "92 pages, 12 figures". */
export function formatPhaseSummary(
  phase: RunPhaseProgress | undefined,
): string | null {
  if (!phase?.tasks?.length) return null;
  const bits = phase.tasks
    .filter((t) => t.total > 0)
    .map((t) => `${t.done} ${t.unit}`);
  return bits.length ? bits.join(", ") : null;
}

/**
 * Defence in depth: never let a stale poll regress done/total or un-done a phase.
 */
export function clampMonotonic(
  rawPrev: RunProgress | null | undefined,
  rawNext: RunProgress | null | undefined,
): RunProgress | null {
  // Wire data is untrusted: a malformed ledger (no `phases` array / `seq`)
  // must degrade to "no ledger", never throw inside a React Query queryFn.
  const prev = isRunProgress(rawPrev) ? rawPrev : null;
  const next = isRunProgress(rawNext) ? rawNext : null;
  if (!next && !prev) return null;
  if (!next) return prev ?? null;
  if (!prev) return next;

  const useNextAsBase = next.seq >= prev.seq;
  const base = useNextAsBase ? structuredClone(next) : structuredClone(prev);
  const bound = useNextAsBase ? prev : next;

  for (const phaseId of PHASE_ORDER) {
    const b = bound.phases?.find((p) => p.id === phaseId);
    if (!b) continue;
    const phase = base.phases.find((p) => p.id === phaseId);
    if (!phase) {
      base.phases.push(structuredClone(b));
      continue;
    }
    phase.tasks = phase.tasks ?? [];
    if (b.state === "done") {
      phase.state = "done";
      phase.finished_at = phase.finished_at ?? b.finished_at;
    } else if (b.state === "active" && phase.state === "pending") {
      phase.state = "active";
    }
    for (const bt of b.tasks ?? []) {
      const existing = phase.tasks.find((t) => t.id === bt.id);
      if (existing) {
        existing.done = Math.max(existing.done, bt.done);
        existing.total = Math.max(existing.total, bt.total);
      } else {
        phase.tasks.push({ ...bt });
      }
    }
  }
  base.seq = Math.max(prev.seq, next.seq);
  return base;
}

/**
 * Conservative synthesis when the server has no ledger yet (old docs / old binary).
 * Never parses stage_message — uses stage + stage_progress only.
 */
export function synthesizeFromLegacy(input: {
  stage?: string | null;
  stageProgress01?: number | null;
  counts?: { unit?: string; current?: number; total?: number } | null;
}): RunProgress | null {
  const stage = (input.stage || "").toLowerCase();
  if (!stage || stage === "queued" || stage === "cleaning" || stage === "pending") {
    return null;
  }

  const prepareStages = new Set(["uploading", "converting", "preprocessing", "chunking"]);
  const extractStages = new Set(["extracting", "gleaning"]);
  const materializeStages = new Set([
    "merging",
    "summarizing",
    "embedding",
    "storing",
    "indexing",
    "projecting",
  ]);

  let active: RunPhaseId = "prepare";
  if (extractStages.has(stage)) active = "extract";
  else if (materializeStages.has(stage)) active = "materialize";
  else if (!prepareStages.has(stage) && stage === "completed") {
    active = "materialize";
  }

  const hasExplicitProgress =
    typeof input.stageProgress01 === "number" && Number.isFinite(input.stageProgress01);
  const frac = hasExplicitProgress
    ? Math.min(0.99, Math.max(0, input.stageProgress01 as number))
    : 0;

  const phases: RunPhaseProgress[] = PHASE_ORDER.map((id) => {
    if (PHASE_ORDER.indexOf(id) < PHASE_ORDER.indexOf(active)) {
      return { id, state: "done" as const, tasks: [] };
    }
    if (id === active) {
      const tasks: RunTaskProgress[] = [];
      const c = input.counts;
      if (c && typeof c.current === "number" && typeof c.total === "number" && c.total > 0) {
        const unit = (c.unit || "chunks") as RunTaskUnit;
        const idMap: Record<string, RunTaskId> = {
          pages: "pages",
          figures: "figures",
          chunks: "chunks",
          entities: "entities",
          relationships: "relationships",
          embeddings: "embeddings",
        };
        const tid = idMap[unit] ?? (active === "extract" ? "chunks" : "pages");
        tasks.push({
          id: tid,
          unit,
          done: c.current,
          total: c.total,
        });
      } else if (hasExplicitProgress && frac > 0) {
        // Synthetic 100-slot task from stage_progress so the bar is determinate.
        tasks.push({
          id: active === "extract" ? "chunks" : active === "materialize" ? "embeddings" : "pages",
          unit: active === "extract" ? "chunks" : active === "materialize" ? "embeddings" : "pages",
          done: Math.round(frac * 100),
          total: 100,
        });
      }
      return { id, state: "active" as const, tasks };
    }
    return { id, state: "pending" as const, tasks: [] };
  });

  return { seq: 0, phases };
}

/** Overall 0–1 from equal-weight phases (cap 0.99 until materialize done). */
export function overallFromLedger(ledger: RunProgress | null | undefined): number {
  if (!ledger) return 0;
  const fills = PHASE_ORDER.map((id) => phaseFill01(findPhase(ledger, id)));
  const avg = fills.reduce((a, b) => a + b, 0) / fills.length;
  const matDone = findPhase(ledger, "materialize")?.state === "done";
  return matDone ? 1 : Math.min(0.99, avg);
}

export function isRunProgress(value: unknown): value is RunProgress {
  if (!value || typeof value !== "object") return false;
  const v = value as RunProgress;
  return typeof v.seq === "number" && Array.isArray(v.phases);
}
