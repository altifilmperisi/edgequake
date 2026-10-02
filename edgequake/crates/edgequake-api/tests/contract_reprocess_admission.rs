//! Contract: reprocess admission edge cases (deleting / cancelling / force).
//!
//! These are pure SSOT proofs — no Postgres required. Handler wiring delegates
//! to the same `evaluate_reprocess_admission` function.

use edgequake_api::services::{
    evaluate_reprocess_admission, ReprocessAdmitContext, ReprocessAdmitDecision,
    ReprocessSkipReason,
};

fn decide(
    status: Option<&str>,
    force: bool,
    full: bool,
    ingest: bool,
    deletion: bool,
    cancel: bool,
) -> ReprocessAdmitDecision {
    evaluate_reprocess_admission(ReprocessAdmitContext {
        status,
        force,
        restart_from_scratch: full,
        has_active_ingest_task: ingest,
        has_active_deletion_task: deletion,
        cancel_intent: cancel,
        tombstoned: false,
    })
}

#[test]
fn matrix_lifecycle_exclusive_never_admits() {
    let cases = [
        (Some("deleting"), ReprocessSkipReason::DeletingInProgress),
        (Some("delete_failed"), ReprocessSkipReason::DeleteFailed),
    ];
    for (status, expected) in cases {
        // Default
        assert_eq!(
            decide(status, false, false, false, false, false),
            ReprocessAdmitDecision::Skip(expected)
        );
        // force + Full must still refuse
        assert_eq!(
            decide(status, true, true, true, false, false),
            ReprocessAdmitDecision::Skip(expected)
        );
    }
}

/// The 2026-10 incident: a batch delete tombstoned the document, cleanup timed
/// out, and the row still read `failed`. The projection said "reprocess me";
/// the authority said "never". Authority wins for every status, even `force`.
#[test]
fn matrix_tombstone_beats_any_stale_status_even_force_full() {
    for status in [
        "failed",
        "cancelled",
        "partial_failure",
        "completed",
        "processing",
    ] {
        let d = evaluate_reprocess_admission(ReprocessAdmitContext {
            status: Some(status),
            force: true,
            restart_from_scratch: true,
            has_active_ingest_task: false,
            has_active_deletion_task: false,
            cancel_intent: false,
            tombstoned: true,
        });
        assert_eq!(
            d,
            ReprocessAdmitDecision::Skip(ReprocessSkipReason::Tombstoned),
            "status={status}"
        );
    }
}

#[test]
fn matrix_active_deletion_task_beats_stale_failed_status() {
    assert_eq!(
        decide(Some("failed"), true, true, false, true, false),
        ReprocessAdmitDecision::Skip(ReprocessSkipReason::DeletingInProgress)
    );
}

#[test]
fn matrix_cancelling_blocks_until_terminal_cancelled() {
    assert_eq!(
        decide(Some("processing"), true, true, true, false, true),
        ReprocessAdmitDecision::Skip(ReprocessSkipReason::CancellingInProgress)
    );
    assert!(decide(Some("cancelled"), false, false, false, false, true).is_admit());
}

#[test]
fn matrix_recoverable_defaults_admit() {
    for s in ["failed", "cancelled", "partial_failure"] {
        assert!(decide(Some(s), false, false, false, false, false).is_admit());
    }
}

#[test]
fn matrix_orphan_pending_admits_active_pending_requires_force_full() {
    assert!(decide(Some("pending"), false, false, false, false, false).is_admit());
    assert_eq!(
        decide(Some("pending"), false, false, true, false, false),
        ReprocessAdmitDecision::Skip(ReprocessSkipReason::AlreadyProcessing)
    );
    assert_eq!(
        decide(Some("pending"), true, false, true, false, false),
        ReprocessAdmitDecision::Skip(ReprocessSkipReason::AlreadyProcessing)
    );
    assert!(decide(Some("pending"), true, true, true, false, false).is_admit());
}

#[test]
fn matrix_completed_requires_force() {
    assert_eq!(
        decide(Some("completed"), false, false, false, false, false),
        ReprocessAdmitDecision::Skip(ReprocessSkipReason::NotEligibleStatus)
    );
    assert!(decide(Some("completed"), true, false, false, false, false).is_admit());
}

#[test]
fn matrix_skip_reason_keys_stable_for_api() {
    assert_eq!(
        ReprocessSkipReason::DeletingInProgress.as_str(),
        "deleting_in_progress"
    );
    assert_eq!(
        ReprocessSkipReason::CancellingInProgress.as_str(),
        "cancelling_in_progress"
    );
    assert_eq!(ReprocessSkipReason::DeleteFailed.as_str(), "delete_failed");
    assert_eq!(ReprocessSkipReason::NotFound.as_str(), "not_found");
    assert_eq!(ReprocessSkipReason::Tombstoned.as_str(), "tombstoned");
}
