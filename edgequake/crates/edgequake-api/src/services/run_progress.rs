//! SPEC-155 — Run progress ledger (LAW: progress is done/total per unit of work).
//!
//! First principle: a progress counter is a property of the **work unit that
//! finished**, never of the last free-text message. Pages, figures, and chunks
//! each own a typed slot; `done` only grows; entering a later phase freezes
//! earlier ones. Legacy `stage_progress` / `progress_counts` are **derived**
//! projections so old readers keep working.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use utoipa::ToSchema;

use crate::handlers::ingestion_types::IngestionProgressCounts;

/// Metadata key for the durable run-progress ledger.
pub const RUN_PROGRESS_KEY: &str = "run_progress";

/// Phase strip ids (Admit is queue-only and has no work tasks).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RunPhaseId {
    Prepare,
    Extract,
    Materialize,
}

impl RunPhaseId {
    pub const ALL: [Self; 3] = [Self::Prepare, Self::Extract, Self::Materialize];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Prepare => "prepare",
            Self::Extract => "extract",
            Self::Materialize => "materialize",
        }
    }

    fn rank(self) -> u8 {
        match self {
            Self::Prepare => 0,
            Self::Extract => 1,
            Self::Materialize => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RunPhaseState {
    Pending,
    Active,
    Done,
}

/// Count unit for a task (wire vocabulary matches `IngestionProgressCounts`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RunTaskUnit {
    Pages,
    Figures,
    Chunks,
    Entities,
    Relationships,
    Embeddings,
}

impl RunTaskUnit {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pages => "pages",
            Self::Figures => "figures",
            Self::Chunks => "chunks",
            Self::Entities => "entities",
            Self::Relationships => "relationships",
            Self::Embeddings => "embeddings",
        }
    }
}

/// Stable task id within a phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RunTaskId {
    Pages,
    Figures,
    Chunks,
    Embeddings,
    Entities,
    Relationships,
}

impl RunTaskId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pages => "pages",
            Self::Figures => "figures",
            Self::Chunks => "chunks",
            Self::Embeddings => "embeddings",
            Self::Entities => "entities",
            Self::Relationships => "relationships",
        }
    }

    pub fn unit(self) -> RunTaskUnit {
        match self {
            Self::Pages => RunTaskUnit::Pages,
            Self::Figures => RunTaskUnit::Figures,
            Self::Chunks => RunTaskUnit::Chunks,
            Self::Embeddings => RunTaskUnit::Embeddings,
            Self::Entities => RunTaskUnit::Entities,
            Self::Relationships => RunTaskUnit::Relationships,
        }
    }

    pub fn phase(self) -> RunPhaseId {
        match self {
            Self::Pages | Self::Figures => RunPhaseId::Prepare,
            Self::Chunks => RunPhaseId::Extract,
            Self::Embeddings | Self::Entities | Self::Relationships => RunPhaseId::Materialize,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct RunTaskProgress {
    pub id: RunTaskId,
    pub unit: RunTaskUnit,
    pub done: u64,
    pub total: u64,
    /// Concurrent workers currently in flight (informational; not part of fill).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub in_flight: Option<u64>,
}

impl RunTaskProgress {
    pub fn new(id: RunTaskId, done: u64, total: u64) -> Self {
        Self {
            id,
            unit: id.unit(),
            done,
            total,
            in_flight: None,
        }
    }

    pub fn with_in_flight(mut self, n: u64) -> Self {
        self.in_flight = Some(n);
        self
    }

    pub fn fraction(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        (self.done as f64 / self.total as f64).clamp(0.0, 1.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct RunPhaseProgress {
    pub id: RunPhaseId,
    pub state: RunPhaseState,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tasks: Vec<RunTaskProgress>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<DateTime<Utc>>,
}

impl RunPhaseProgress {
    fn empty(id: RunPhaseId) -> Self {
        Self {
            id,
            state: RunPhaseState::Pending,
            tasks: Vec::new(),
            started_at: None,
            finished_at: None,
        }
    }

    /// Weighted fill for this phase (0–1). Done ⇒ 1; pending ⇒ 0.
    pub fn fill01(&self) -> f64 {
        match self.state {
            RunPhaseState::Pending => 0.0,
            RunPhaseState::Done => 1.0,
            RunPhaseState::Active => {
                if self.tasks.is_empty() {
                    return 0.02;
                }
                // Equal weight per task that has a known total; ignore empty.
                let known: Vec<_> = self.tasks.iter().filter(|t| t.total > 0).collect();
                if known.is_empty() {
                    return 0.02;
                }
                let sum: f64 = known.iter().map(|t| t.fraction()).sum();
                (sum / known.len() as f64).clamp(0.0, 0.99)
            }
        }
    }

    pub fn task(&self, id: RunTaskId) -> Option<&RunTaskProgress> {
        self.tasks.iter().find(|t| t.id == id)
    }
}

/// Durable, monotonic progress ledger for one document run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct RunProgress {
    /// Monotonic write sequence (fencing + coalesce).
    pub seq: u64,
    pub phases: Vec<RunPhaseProgress>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

impl Default for RunProgress {
    fn default() -> Self {
        Self {
            seq: 0,
            phases: RunPhaseId::ALL
                .iter()
                .map(|id| RunPhaseProgress::empty(*id))
                .collect(),
            updated_at: None,
        }
    }
}

impl RunProgress {
    pub fn phase(&self, id: RunPhaseId) -> Option<&RunPhaseProgress> {
        self.phases.iter().find(|p| p.id == id)
    }

    pub fn phase_mut(&mut self, id: RunPhaseId) -> &mut RunPhaseProgress {
        if let Some(idx) = self.phases.iter().position(|p| p.id == id) {
            return &mut self.phases[idx];
        }
        self.phases.push(RunPhaseProgress::empty(id));
        self.phases.last_mut().expect("just pushed")
    }

    /// Active phase (highest-rank Active, else highest Done, else Prepare).
    pub fn active_phase(&self) -> RunPhaseId {
        if let Some(p) = self
            .phases
            .iter()
            .filter(|p| p.state == RunPhaseState::Active)
            .max_by_key(|p| p.id.rank())
        {
            return p.id;
        }
        if let Some(p) = self
            .phases
            .iter()
            .filter(|p| p.state == RunPhaseState::Done)
            .max_by_key(|p| p.id.rank())
        {
            // Prefer the next pending after the last done.
            let next = match p.id {
                RunPhaseId::Prepare => RunPhaseId::Extract,
                RunPhaseId::Extract => RunPhaseId::Materialize,
                RunPhaseId::Materialize => RunPhaseId::Materialize,
            };
            return next;
        }
        RunPhaseId::Prepare
    }
}

/// Events the ledger accepts (pure — no I/O).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunProgressEvent {
    /// Set/advance a task counter. Activates the owning phase and freezes prior ones.
    Task {
        id: RunTaskId,
        done: u64,
        total: u64,
        in_flight: Option<u64>,
    },
    /// Mark a phase complete (freeze its counters).
    CompletePhase(RunPhaseId),
    /// Wipe the ledger (reprocess / new run).
    Reset,
}

/// Apply one event. Always bumps `seq`. Never decreases `done` or shrinks a locked total.
pub fn apply_event(ledger: &mut RunProgress, event: RunProgressEvent, now: DateTime<Utc>) {
    ledger.seq = ledger.seq.saturating_add(1);
    ledger.updated_at = Some(now);

    match event {
        RunProgressEvent::Reset => {
            *ledger = RunProgress {
                seq: ledger.seq,
                ..RunProgress::default()
            };
            ledger.updated_at = Some(now);
        }
        RunProgressEvent::CompletePhase(phase_id) => {
            complete_phase(ledger, phase_id, now);
        }
        RunProgressEvent::Task {
            id,
            done,
            total,
            in_flight,
        } => {
            let phase_id = id.phase();
            activate_phase(ledger, phase_id, now);
            let phase = ledger.phase_mut(phase_id);
            upsert_task(phase, id, done, total, in_flight);
        }
    }
}

fn activate_phase(ledger: &mut RunProgress, phase_id: RunPhaseId, now: DateTime<Utc>) {
    // Freeze every earlier phase.
    for earlier in RunPhaseId::ALL {
        if earlier.rank() < phase_id.rank() {
            complete_phase(ledger, earlier, now);
        }
    }
    let phase = ledger.phase_mut(phase_id);
    if phase.state == RunPhaseState::Pending {
        phase.state = RunPhaseState::Active;
        phase.started_at = Some(now);
        phase.finished_at = None;
    } else if phase.state == RunPhaseState::Done {
        // Re-entering a done phase (rare) — reopen as active.
        phase.state = RunPhaseState::Active;
        phase.finished_at = None;
        if phase.started_at.is_none() {
            phase.started_at = Some(now);
        }
    }
    // Demote any later active phases back to pending (should not happen in happy path).
    for later in RunPhaseId::ALL {
        if later.rank() > phase_id.rank() {
            let p = ledger.phase_mut(later);
            if p.state == RunPhaseState::Active {
                p.state = RunPhaseState::Pending;
            }
        }
    }
}

fn complete_phase(ledger: &mut RunProgress, phase_id: RunPhaseId, now: DateTime<Utc>) {
    let phase = ledger.phase_mut(phase_id);
    if phase.state == RunPhaseState::Done {
        return;
    }
    if phase.state == RunPhaseState::Pending && phase.tasks.is_empty() {
        // Completing a never-started phase (e.g. non-PDF skip Prepare) — mark done empty.
        phase.state = RunPhaseState::Done;
        phase.finished_at = Some(now);
        if phase.started_at.is_none() {
            phase.started_at = Some(now);
        }
        return;
    }
    phase.state = RunPhaseState::Done;
    phase.finished_at = Some(now);
    if phase.started_at.is_none() {
        phase.started_at = Some(now);
    }
    // Clear in-flight on completion.
    for task in &mut phase.tasks {
        task.in_flight = None;
        if task.total > 0 {
            task.done = task.done.max(task.total.min(task.done));
        }
    }
}

fn upsert_task(
    phase: &mut RunPhaseProgress,
    id: RunTaskId,
    done: u64,
    total: u64,
    in_flight: Option<u64>,
) {
    if let Some(existing) = phase.tasks.iter_mut().find(|t| t.id == id) {
        // Total only grows (never shrink a known total).
        if total > existing.total {
            existing.total = total;
        }
        // done is monotonic and never exceeds a known total.
        let capped = if existing.total > 0 {
            done.min(existing.total)
        } else {
            done
        };
        existing.done = existing.done.max(capped);
        // Prefer max for in_flight reporting only when provided.
        if let Some(n) = in_flight {
            existing.in_flight = Some(n);
        }
    } else {
        let mut task = RunTaskProgress::new(id, done, total);
        if let Some(n) = in_flight {
            task = task.with_in_flight(n);
        }
        // Clamp done ≤ total when total known.
        if task.total > 0 {
            task.done = task.done.min(task.total);
        }
        phase.tasks.push(task);
    }
}

// ---------------------------------------------------------------------------
// Legacy projections (old FE / poll clients)
// ---------------------------------------------------------------------------

/// Overall weighted progress 0–1 from the ledger (never 1.0 until Materialize done).
pub fn derive_stage_progress01(ledger: &RunProgress) -> f64 {
    // Equal weight across the three work phases.
    let fills: Vec<f64> = RunPhaseId::ALL
        .iter()
        .filter_map(|id| ledger.phase(*id).map(|p| p.fill01()))
        .collect();
    if fills.is_empty() {
        return 0.0;
    }
    let avg = fills.iter().sum::<f64>() / fills.len() as f64;
    let materialize_done = ledger
        .phase(RunPhaseId::Materialize)
        .is_some_and(|p| p.state == RunPhaseState::Done);
    if materialize_done {
        1.0
    } else {
        avg.clamp(0.0, 0.99)
    }
}

/// Structured counts for the **active** task (legacy `progress_counts` slot).
pub fn derive_progress_counts(ledger: &RunProgress) -> Option<IngestionProgressCounts> {
    let active = ledger.active_phase();
    let phase = ledger.phase(active)?;
    // Prefer the most recently advanced incomplete task; else last task.
    let task = phase
        .tasks
        .iter()
        .rev()
        .find(|t| t.total > 0 && t.done < t.total)
        .or_else(|| phase.tasks.iter().rev().find(|t| t.total > 0))?;
    Some(IngestionProgressCounts {
        current: task.done,
        total: task.total,
        unit: task.unit.as_str().to_string(),
    })
}

/// Human caption for the active phase (no regex needed on the FE).
pub fn derive_stage_message(ledger: &RunProgress) -> String {
    let active = ledger.active_phase();
    let Some(phase) = ledger.phase(active) else {
        return "Processing…".to_string();
    };
    match active {
        RunPhaseId::Prepare => format_prepare_message(phase),
        RunPhaseId::Extract => format_extract_message(phase),
        RunPhaseId::Materialize => format_materialize_message(phase),
    }
}

fn format_prepare_message(phase: &RunPhaseProgress) -> String {
    let pages = phase.task(RunTaskId::Pages);
    let figures = phase.task(RunTaskId::Figures);
    match (pages, figures) {
        (Some(p), Some(f)) if f.total > 0 && f.done < f.total => {
            format!(
                "Preparing — pages {}/{}, figures {}/{}",
                p.done, p.total, f.done, f.total
            )
        }
        (Some(p), Some(f)) if f.total > 0 => {
            format!(
                "Preparing — pages {}/{}, figures {}/{}",
                p.done, p.total, f.done, f.total
            )
        }
        (Some(p), _) if p.total > 0 => {
            format!("Converting PDF to Markdown — page {}/{}", p.done, p.total)
        }
        (_, Some(f)) if f.total > 0 => {
            format!("Analyzing figures — figure {}/{}", f.done, f.total)
        }
        _ => "Preparing…".to_string(),
    }
}

fn format_extract_message(phase: &RunPhaseProgress) -> String {
    let Some(c) = phase.task(RunTaskId::Chunks) else {
        return "Extracting entities…".to_string();
    };
    if c.total == 0 {
        return "Extracting entities…".to_string();
    }
    match c.in_flight {
        Some(n) if n > 0 => format!(
            "Extracting entities — {}/{} chunks, {} in flight",
            c.done, c.total, n
        ),
        _ => format!("Extracting entities — {}/{} chunks", c.done, c.total),
    }
}

fn format_materialize_message(phase: &RunPhaseProgress) -> String {
    if let Some(e) = phase.task(RunTaskId::Embeddings) {
        if e.total > 0 && e.done < e.total {
            return format!("Embedding — {}/{}", e.done, e.total);
        }
    }
    let ent = phase.task(RunTaskId::Entities);
    let rel = phase.task(RunTaskId::Relationships);
    match (ent, rel) {
        (Some(e), Some(r)) if e.total > 0 || r.total > 0 => format!(
            "Storing in knowledge graph — entities {}/{}, relationships {}/{}",
            e.done,
            e.total.max(1),
            r.done,
            r.total.max(1)
        ),
        (Some(e), _) if e.total > 0 => {
            format!(
                "Storing in knowledge graph — entities {}/{}",
                e.done, e.total
            )
        }
        _ => "Materializing…".to_string(),
    }
}

/// Unified stage slug for `current_stage` derived from the ledger.
pub fn derive_current_stage(ledger: &RunProgress) -> &'static str {
    match ledger.active_phase() {
        RunPhaseId::Prepare => {
            let phase = ledger.phase(RunPhaseId::Prepare);
            if phase.is_some_and(|p| {
                p.task(RunTaskId::Figures)
                    .is_some_and(|f| f.total > 0 && f.done < f.total)
            }) {
                "converting"
            } else if phase.is_some_and(|p| p.state == RunPhaseState::Done) {
                "chunking"
            } else {
                "converting"
            }
        }
        RunPhaseId::Extract => "extracting",
        RunPhaseId::Materialize => {
            let phase = ledger.phase(RunPhaseId::Materialize);
            if phase.is_some_and(|p| {
                p.task(RunTaskId::Embeddings)
                    .is_some_and(|e| e.total > 0 && e.done < e.total)
            }) {
                "embedding"
            } else {
                "storing"
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Metadata helpers
// ---------------------------------------------------------------------------

pub fn run_progress_from_value(v: &Value) -> Option<RunProgress> {
    serde_json::from_value(v.clone()).ok()
}

pub fn run_progress_from_metadata(obj: &Map<String, Value>) -> Option<RunProgress> {
    obj.get(RUN_PROGRESS_KEY).and_then(run_progress_from_value)
}

pub fn insert_run_progress(meta: &mut Map<String, Value>, ledger: &RunProgress) {
    if let Ok(v) = serde_json::to_value(ledger) {
        meta.insert(RUN_PROGRESS_KEY.to_string(), v);
    }
}

pub fn clear_run_progress(meta: &mut Map<String, Value>) {
    meta.remove(RUN_PROGRESS_KEY);
}

/// Apply ledger + derived legacy fields onto document metadata.
pub fn apply_run_progress_to_metadata(meta: &mut Map<String, Value>, ledger: &RunProgress) {
    insert_run_progress(meta, ledger);
    let stage = derive_current_stage(ledger);
    let message = derive_stage_message(ledger);
    let progress = derive_stage_progress01(ledger);
    meta.insert("current_stage".to_string(), json!(stage));
    meta.insert("stage_message".to_string(), json!(message));
    meta.insert("stage_progress".to_string(), json!(progress));
    meta.insert("updated_at".to_string(), json!(Utc::now().to_rfc3339()));
    if let Some(counts) = derive_progress_counts(ledger) {
        crate::services::insert_progress_counts(meta, &counts);
    }
}

/// Monotonic merge of two ledgers (defence in depth for stale polls / WS races).
/// Keeps the higher seq; within equal seq, maxes each task's done/total.
pub fn clamp_monotonic(prev: &RunProgress, next: &RunProgress) -> RunProgress {
    if next.seq > prev.seq {
        return clamp_phases(prev, next);
    }
    if prev.seq > next.seq {
        return clamp_phases(next, prev);
    }
    // Equal seq — take next but never regress fills.
    clamp_phases(prev, next)
}

fn clamp_phases(lower_bound: &RunProgress, candidate: &RunProgress) -> RunProgress {
    let mut out = candidate.clone();
    out.seq = out.seq.max(lower_bound.seq);
    for phase_id in RunPhaseId::ALL {
        let Some(bound) = lower_bound.phase(phase_id) else {
            continue;
        };
        let phase = out.phase_mut(phase_id);
        // Once Done, stay Done.
        if bound.state == RunPhaseState::Done {
            phase.state = RunPhaseState::Done;
            if phase.finished_at.is_none() {
                phase.finished_at = bound.finished_at;
            }
        } else if bound.state == RunPhaseState::Active && phase.state == RunPhaseState::Pending {
            phase.state = RunPhaseState::Active;
        }
        for bt in &bound.tasks {
            if let Some(existing) = phase.tasks.iter_mut().find(|t| t.id == bt.id) {
                existing.done = existing.done.max(bt.done);
                existing.total = existing.total.max(bt.total);
            } else {
                phase.tasks.push(bt.clone());
            }
        }
    }
    out
}

/// Build a ledger snapshot from a single task update (convenience for writers).
pub fn apply_task(
    ledger: &mut RunProgress,
    id: RunTaskId,
    done: u64,
    total: u64,
    in_flight: Option<u64>,
) {
    apply_event(
        ledger,
        RunProgressEvent::Task {
            id,
            done,
            total,
            in_flight,
        },
        Utc::now(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> DateTime<Utc> {
        Utc::now()
    }

    #[test]
    fn pages_then_figures_keep_pages_and_prepare_fill() {
        let mut ledger = RunProgress::default();
        apply_event(
            &mut ledger,
            RunProgressEvent::Task {
                id: RunTaskId::Pages,
                done: 92,
                total: 92,
                in_flight: None,
            },
            now(),
        );
        let fill_after_pages = ledger.phase(RunPhaseId::Prepare).unwrap().fill01();
        assert!(fill_after_pages >= 0.99, "pages complete ⇒ ~full prepare");

        apply_event(
            &mut ledger,
            RunProgressEvent::Task {
                id: RunTaskId::Figures,
                done: 1,
                total: 12,
                in_flight: None,
            },
            now(),
        );
        let prepare = ledger.phase(RunPhaseId::Prepare).unwrap();
        assert_eq!(prepare.task(RunTaskId::Pages).unwrap().done, 92);
        assert_eq!(prepare.task(RunTaskId::Figures).unwrap().done, 1);
        // Prepare fill is average of pages(1.0) and figures(~0.08) ≈ 0.54 — never 0.
        let fill = prepare.fill01();
        assert!(
            fill > 0.4,
            "prepare must not collapse when figures start: {fill}"
        );
        assert!(fill < fill_after_pages || fill > 0.4);
        // Pages counter untouched.
        assert_eq!(prepare.task(RunTaskId::Pages).unwrap().total, 92);
    }

    #[test]
    fn done_is_monotonic_under_shuffled_events() {
        let mut ledger = RunProgress::default();
        let events = [
            (5u64, 10u64),
            (3, 10), // regress attempt
            (8, 10),
            (8, 10), // duplicate
            (10, 10),
        ];
        for (done, total) in events {
            apply_event(
                &mut ledger,
                RunProgressEvent::Task {
                    id: RunTaskId::Pages,
                    done,
                    total,
                    in_flight: None,
                },
                now(),
            );
        }
        let pages = ledger
            .phase(RunPhaseId::Prepare)
            .unwrap()
            .task(RunTaskId::Pages)
            .unwrap();
        assert_eq!(pages.done, 10);
        assert_eq!(pages.total, 10);
    }

    #[test]
    fn chunk_completed_not_last_started_index() {
        let mut ledger = RunProgress::default();
        // Simulate: total 92, 60 completed, 16 in flight (last started index would be 76).
        apply_event(
            &mut ledger,
            RunProgressEvent::Task {
                id: RunTaskId::Chunks,
                done: 60,
                total: 92,
                in_flight: Some(16),
            },
            now(),
        );
        let extract = ledger.phase(RunPhaseId::Extract).unwrap();
        assert_eq!(extract.state, RunPhaseState::Active);
        let chunks = extract.task(RunTaskId::Chunks).unwrap();
        assert_eq!(chunks.done, 60);
        assert_eq!(chunks.in_flight, Some(16));
        // Prepare must be frozen done.
        assert_eq!(
            ledger.phase(RunPhaseId::Prepare).unwrap().state,
            RunPhaseState::Done
        );
        let msg = derive_stage_message(&ledger);
        assert!(msg.contains("60/92"), "message={msg}");
        assert!(msg.contains("in flight"), "message={msg}");
    }

    #[test]
    fn late_page_event_after_extract_does_not_unfreeze_prepare_fill() {
        let mut ledger = RunProgress::default();
        apply_event(
            &mut ledger,
            RunProgressEvent::Task {
                id: RunTaskId::Pages,
                done: 92,
                total: 92,
                in_flight: None,
            },
            now(),
        );
        apply_event(
            &mut ledger,
            RunProgressEvent::CompletePhase(RunPhaseId::Prepare),
            now(),
        );
        apply_event(
            &mut ledger,
            RunProgressEvent::Task {
                id: RunTaskId::Chunks,
                done: 10,
                total: 50,
                in_flight: None,
            },
            now(),
        );
        // Late page event (stale convert callback).
        apply_event(
            &mut ledger,
            RunProgressEvent::Task {
                id: RunTaskId::Pages,
                done: 50,
                total: 92,
                in_flight: None,
            },
            now(),
        );
        // Pages done stays at 92 (monotonic); Prepare reactivates but fill stays high.
        let prepare = ledger.phase(RunPhaseId::Prepare).unwrap();
        assert_eq!(prepare.task(RunTaskId::Pages).unwrap().done, 92);
    }

    #[test]
    fn reset_clears_tasks_keeps_seq() {
        let mut ledger = RunProgress::default();
        apply_event(
            &mut ledger,
            RunProgressEvent::Task {
                id: RunTaskId::Chunks,
                done: 5,
                total: 10,
                in_flight: None,
            },
            now(),
        );
        let seq = ledger.seq;
        apply_event(&mut ledger, RunProgressEvent::Reset, now());
        assert!(ledger.seq > seq);
        assert!(ledger.phase(RunPhaseId::Extract).unwrap().tasks.is_empty());
    }

    #[test]
    fn clamp_monotonic_rejects_regression() {
        let mut a = RunProgress::default();
        apply_task(&mut a, RunTaskId::Chunks, 40, 100, None);
        let mut b = a.clone();
        b.seq = a.seq; // equal
                       // Simulate stale poll with lower done.
        if let Some(t) = b
            .phase_mut(RunPhaseId::Extract)
            .tasks
            .iter_mut()
            .find(|t| t.id == RunTaskId::Chunks)
        {
            t.done = 10;
        }
        let merged = clamp_monotonic(&a, &b);
        assert_eq!(
            merged
                .phase(RunPhaseId::Extract)
                .unwrap()
                .task(RunTaskId::Chunks)
                .unwrap()
                .done,
            40
        );
    }

    #[test]
    fn legacy_projections_prefer_active_incomplete_task() {
        let mut ledger = RunProgress::default();
        apply_task(&mut ledger, RunTaskId::Pages, 92, 92, None);
        apply_task(&mut ledger, RunTaskId::Figures, 3, 12, None);
        let counts = derive_progress_counts(&ledger).unwrap();
        assert_eq!(counts.unit, "figures");
        assert_eq!(counts.current, 3);
        assert_eq!(counts.total, 12);
        let progress = derive_stage_progress01(&ledger);
        assert!(progress > 0.0 && progress < 1.0);
    }

    #[test]
    fn total_does_not_shrink() {
        let mut ledger = RunProgress::default();
        apply_task(&mut ledger, RunTaskId::Pages, 5, 100, None);
        apply_task(&mut ledger, RunTaskId::Pages, 6, 50, None); // bad shrink
        let pages = ledger
            .phase(RunPhaseId::Prepare)
            .unwrap()
            .task(RunTaskId::Pages)
            .unwrap();
        assert_eq!(pages.total, 100);
        assert_eq!(pages.done, 6);
    }

    #[test]
    fn metadata_round_trip() {
        let mut ledger = RunProgress::default();
        apply_task(&mut ledger, RunTaskId::Chunks, 12, 40, Some(4));
        let mut meta = Map::new();
        apply_run_progress_to_metadata(&mut meta, &ledger);
        let recovered = run_progress_from_metadata(&meta).unwrap();
        assert_eq!(
            recovered
                .phase(RunPhaseId::Extract)
                .unwrap()
                .task(RunTaskId::Chunks)
                .unwrap()
                .done,
            12
        );
        assert_eq!(
            meta.get("current_stage").and_then(|v| v.as_str()),
            Some("extracting")
        );
        assert!(meta.get("progress_counts").is_some());
    }
}
