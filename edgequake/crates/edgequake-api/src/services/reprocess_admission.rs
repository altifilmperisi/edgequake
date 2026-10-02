//! Reprocess admission matrix (First Principles SSOT).
//!
//! Document lifecycle states are mutually exclusive with some operations.
//! Reprocess must never race an in-flight **deletion** or **cancellation**,
//! even when `force=true`. `force` only widens admission for recoverable
//! and completed/in-flight ingest states.
//!
//! ## Matrix (status × intent)
//!
//! | Status / condition              | Default | `force` soft | `force` Full |
//! |---------------------------------|---------|--------------|--------------|
//! | `failed` / `cancelled` /
//!   `partial_failure`               | Admit   | Admit        | Admit        |
//! | `pending`/`queued` orphan       | Admit   | Admit        | Admit        |
//! | `pending`/`queued` + active task| Skip    | Skip         | Admit†       |
//! | `processing` (in-flight)        | Skip    | Skip         | Admit†       |
//! | `completed` / `indexed`         | Skip    | Admit        | Admit        |
//! | `deleting` / active deletion    | Skip‡   | Skip‡        | Skip‡        |
//! | `delete_failed`                 | Skip‡   | Skip‡        | Skip‡        |
//! | tombstoned (P0 authority)       | Skip‡   | Skip‡        | Skip‡        |
//! | cancel intent (not yet terminal)| Skip‡   | Skip‡        | Skip‡        |
//!
//! † Full restart cancels/purges the ingest task before requeue.
//! ‡ Lifecycle-exclusive — never overridden by `force` (fail closed).

use std::fmt;

/// Why a candidate was not admitted for reprocess.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReprocessSkipReason {
    /// Status is not in the default recoverable set and `force` was false.
    NotEligibleStatus,
    /// An ingest task (or processing status) is already active.
    AlreadyProcessing,
    /// Document is mid-delete (`status=deleting` or active Deletion task).
    DeletingInProgress,
    /// Previous delete failed mid-cascade — must finish/reset delete first.
    DeleteFailed,
    /// P0 authority tombstoned the document (irreversible); only finishing the
    /// delete or a fresh upload can follow. The status projection may lag.
    Tombstoned,
    /// Cancel intent is active; wait for terminal `cancelled`.
    CancellingInProgress,
    /// Targeted document_id was not found in scoped metadata.
    NotFound,
    /// SPEC-119 graph retract failed closed after early admit (issue #385).
    GraphCleanupFailed,
    /// Task create/enqueue (or a later admit write) failed after early admit.
    EnqueueFailed,
}

impl ReprocessSkipReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotEligibleStatus => "not_eligible_status",
            Self::AlreadyProcessing => "already_processing",
            Self::DeletingInProgress => "deleting_in_progress",
            Self::DeleteFailed => "delete_failed",
            Self::Tombstoned => "tombstoned",
            Self::CancellingInProgress => "cancelling_in_progress",
            Self::NotFound => "not_found",
            Self::GraphCleanupFailed => "graph_cleanup_failed",
            Self::EnqueueFailed => "enqueue_failed",
        }
    }
}

impl fmt::Display for ReprocessSkipReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Admission outcome for one document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReprocessAdmitDecision {
    Admit,
    Skip(ReprocessSkipReason),
}

impl ReprocessAdmitDecision {
    pub fn is_admit(self) -> bool {
        matches!(self, Self::Admit)
    }

    pub fn skip_reason(self) -> Option<ReprocessSkipReason> {
        match self {
            Self::Admit => None,
            Self::Skip(r) => Some(r),
        }
    }
}

/// Pure inputs for the admission decision (no I/O).
#[derive(Debug, Clone, Copy)]
pub struct ReprocessAdmitContext<'a> {
    /// Document KV `status` (lowercase-insensitive).
    pub status: Option<&'a str>,
    /// `force=true` widens completed / in-flight ingest admission.
    pub force: bool,
    /// Full reprocess (`mode=full`) may cancel an active ingest task.
    pub restart_from_scratch: bool,
    /// Pending/Processing ingest task exists for this document.
    pub has_active_ingest_task: bool,
    /// Active `TaskType::Deletion` exists for this document.
    pub has_active_deletion_task: bool,
    /// Cancellation registry has intent for the document's track_id.
    pub cancel_intent: bool,
    /// P0 authority says the document is tombstoned (irreversible), regardless
    /// of what the `status` projection currently claims.
    pub tombstoned: bool,
}

/// Statuses that are terminal for ingest and recoverable via reprocess.
pub fn is_reprocess_terminal_recoverable(status: &str) -> bool {
    matches!(
        status.to_ascii_lowercase().as_str(),
        "failed" | "cancelled" | "partial_failure"
    )
}

/// Statuses that mean "ingest finished successfully" — only with `force`.
pub fn is_reprocess_completed_status(status: &str) -> bool {
    matches!(
        status.to_ascii_lowercase().as_str(),
        "completed" | "indexed" | "processed"
    )
}

/// Waiting statuses that can be stranded without a worker task (#298).
pub fn is_reprocess_orphan_waiting_status(status: &str) -> bool {
    matches!(status.to_ascii_lowercase().as_str(), "pending" | "queued")
}

/// Lifecycle-exclusive: never admit reprocess (even with `force`).
pub fn is_reprocess_lifecycle_exclusive(status: &str) -> bool {
    matches!(
        status.to_ascii_lowercase().as_str(),
        "deleting" | "delete_failed"
    )
}

/// In-flight ingest statuses (soft reprocess must not interrupt).
pub fn is_reprocess_inflight_status(status: &str) -> bool {
    matches!(
        status.to_ascii_lowercase().as_str(),
        "processing" | "preprocessing" | "converting" | "extracting" | "embedding" | "indexing"
    )
}

/// Evaluate whether a document may be admitted for reprocess.
///
/// Pure / deterministic — callers supply task/deletion/cancel facts.
pub fn evaluate_reprocess_admission(ctx: ReprocessAdmitContext<'_>) -> ReprocessAdmitDecision {
    let status = ctx.status.map(str::trim).filter(|s| !s.is_empty());

    // 1) Active deletion task always wins (status may lag).
    if ctx.has_active_deletion_task {
        return ReprocessAdmitDecision::Skip(ReprocessSkipReason::DeletingInProgress);
    }

    // 1b) Authority tombstone beats any projection status — ingest can never
    //     succeed, so refuse before spending PDF/LLM budget (ignore force).
    if ctx.tombstoned {
        return ReprocessAdmitDecision::Skip(ReprocessSkipReason::Tombstoned);
    }

    // 2) Lifecycle-exclusive statuses — fail closed, ignore force.
    if let Some(s) = status {
        if s.eq_ignore_ascii_case("delete_failed") {
            return ReprocessAdmitDecision::Skip(ReprocessSkipReason::DeleteFailed);
        }
        if s.eq_ignore_ascii_case("deleting") {
            return ReprocessAdmitDecision::Skip(ReprocessSkipReason::DeletingInProgress);
        }
    }

    // 3) Cancel in flight — wait until status is terminal cancelled.
    if ctx.cancel_intent {
        let already_cancelled = status.is_some_and(|s| s.eq_ignore_ascii_case("cancelled"));
        if !already_cancelled {
            return ReprocessAdmitDecision::Skip(ReprocessSkipReason::CancellingInProgress);
        }
    }

    let Some(s) = status else {
        // No status object: targeted lookups treat as not found upstream;
        // batch scans simply never see the row.
        return ReprocessAdmitDecision::Skip(ReprocessSkipReason::NotEligibleStatus);
    };

    // 4) Terminal recoverable — always admit.
    if is_reprocess_terminal_recoverable(s) {
        return ReprocessAdmitDecision::Admit;
    }

    // 5) Orphan waiting (pending/queued without active ingest task).
    if is_reprocess_orphan_waiting_status(s) {
        if ctx.has_active_ingest_task {
            return admit_or_skip_inflight(ctx.force, ctx.restart_from_scratch);
        }
        return ReprocessAdmitDecision::Admit;
    }

    // 6) In-flight ingest status.
    if is_reprocess_inflight_status(s) || ctx.has_active_ingest_task {
        return admit_or_skip_inflight(ctx.force, ctx.restart_from_scratch);
    }

    // 7) Completed — force only.
    if is_reprocess_completed_status(s) {
        return if ctx.force {
            ReprocessAdmitDecision::Admit
        } else {
            ReprocessAdmitDecision::Skip(ReprocessSkipReason::NotEligibleStatus)
        };
    }

    // 8) Unknown status — force admits (legacy / forward-compat), else skip.
    if ctx.force {
        ReprocessAdmitDecision::Admit
    } else {
        ReprocessAdmitDecision::Skip(ReprocessSkipReason::NotEligibleStatus)
    }
}

fn admit_or_skip_inflight(force: bool, restart_from_scratch: bool) -> ReprocessAdmitDecision {
    // Soft (entities-only) must never kill an in-flight pipeline (SPEC-047 P6).
    // Full + force may purge and replace.
    if force && restart_from_scratch {
        ReprocessAdmitDecision::Admit
    } else {
        ReprocessAdmitDecision::Skip(ReprocessSkipReason::AlreadyProcessing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(
        status: Option<&str>,
        force: bool,
        full: bool,
        ingest: bool,
        deletion: bool,
        cancel: bool,
    ) -> ReprocessAdmitContext<'_> {
        ReprocessAdmitContext {
            status,
            force,
            restart_from_scratch: full,
            has_active_ingest_task: ingest,
            has_active_deletion_task: deletion,
            cancel_intent: cancel,
            tombstoned: false,
        }
    }

    #[test]
    fn failed_and_cancelled_always_admit() {
        for s in ["failed", "cancelled", "partial_failure", "FAILED"] {
            assert!(
                evaluate_reprocess_admission(ctx(Some(s), false, false, false, false, false))
                    .is_admit()
            );
        }
    }

    #[test]
    fn tombstoned_is_skipped_for_every_status_even_with_force_full() {
        for status in ["failed", "cancelled", "completed", "processing", "pending"] {
            let mut c = ctx(Some(status), true, true, false, false, false);
            c.tombstoned = true;
            assert_eq!(
                evaluate_reprocess_admission(c),
                ReprocessAdmitDecision::Skip(ReprocessSkipReason::Tombstoned),
                "status={status}"
            );
        }
    }

    #[test]
    fn deleting_always_skipped_even_with_force_full() {
        let d =
            evaluate_reprocess_admission(ctx(Some("deleting"), true, true, false, false, false));
        assert_eq!(
            d,
            ReprocessAdmitDecision::Skip(ReprocessSkipReason::DeletingInProgress)
        );
    }

    #[test]
    fn active_deletion_task_skipped_even_if_status_stale() {
        // Status still "failed" but Deletion task already enqueued.
        let d = evaluate_reprocess_admission(ctx(Some("failed"), true, true, false, true, false));
        assert_eq!(
            d,
            ReprocessAdmitDecision::Skip(ReprocessSkipReason::DeletingInProgress)
        );
    }

    #[test]
    fn delete_failed_never_reprocessed() {
        let d = evaluate_reprocess_admission(ctx(
            Some("delete_failed"),
            true,
            true,
            false,
            false,
            false,
        ));
        assert_eq!(
            d,
            ReprocessAdmitDecision::Skip(ReprocessSkipReason::DeleteFailed)
        );
    }

    #[test]
    fn cancelling_in_progress_skipped() {
        let d =
            evaluate_reprocess_admission(ctx(Some("processing"), true, true, true, false, true));
        assert_eq!(
            d,
            ReprocessAdmitDecision::Skip(ReprocessSkipReason::CancellingInProgress)
        );
    }

    #[test]
    fn cancelled_terminal_with_stale_cancel_intent_admits() {
        // Intent may linger briefly after status=cancelled.
        assert!(evaluate_reprocess_admission(ctx(
            Some("cancelled"),
            false,
            false,
            false,
            false,
            true,
        ))
        .is_admit());
    }

    #[test]
    fn orphan_pending_admits_without_force() {
        assert!(evaluate_reprocess_admission(ctx(
            Some("pending"),
            false,
            false,
            false,
            false,
            false,
        ))
        .is_admit());
    }

    #[test]
    fn pending_with_active_task_skipped_unless_force_full() {
        let soft =
            evaluate_reprocess_admission(ctx(Some("pending"), true, false, true, false, false));
        assert_eq!(
            soft,
            ReprocessAdmitDecision::Skip(ReprocessSkipReason::AlreadyProcessing)
        );
        assert!(
            evaluate_reprocess_admission(ctx(Some("pending"), true, true, true, false, false))
                .is_admit()
        );
    }

    #[test]
    fn processing_requires_force_full() {
        assert_eq!(
            evaluate_reprocess_admission(ctx(Some("processing"), false, false, true, false, false)),
            ReprocessAdmitDecision::Skip(ReprocessSkipReason::AlreadyProcessing)
        );
        assert_eq!(
            evaluate_reprocess_admission(ctx(Some("processing"), true, false, true, false, false)),
            ReprocessAdmitDecision::Skip(ReprocessSkipReason::AlreadyProcessing)
        );
        assert!(evaluate_reprocess_admission(ctx(
            Some("processing"),
            true,
            true,
            true,
            false,
            false,
        ))
        .is_admit());
    }

    #[test]
    fn completed_requires_force() {
        assert_eq!(
            evaluate_reprocess_admission(ctx(Some("completed"), false, false, false, false, false)),
            ReprocessAdmitDecision::Skip(ReprocessSkipReason::NotEligibleStatus)
        );
        assert!(evaluate_reprocess_admission(ctx(
            Some("completed"),
            true,
            false,
            false,
            false,
            false,
        ))
        .is_admit());
    }

    #[test]
    fn skip_reason_strings_are_stable_api_keys() {
        assert_eq!(
            ReprocessSkipReason::DeletingInProgress.as_str(),
            "deleting_in_progress"
        );
        assert_eq!(
            ReprocessSkipReason::CancellingInProgress.as_str(),
            "cancelling_in_progress"
        );
        assert_eq!(
            ReprocessSkipReason::GraphCleanupFailed.as_str(),
            "graph_cleanup_failed"
        );
        assert_eq!(
            ReprocessSkipReason::EnqueueFailed.as_str(),
            "enqueue_failed"
        );
    }
}
