//! SPEC-155: run_progress ledger is monotonic under shuffled chunk completions.
//!
//! Asserts that list/track payloads expose `run_progress` where `chunks.done`
//! equals completed count (never last-started index), and that figures never
//! wipe pages.

use chrono::Utc;
use edgequake_api::services::run_progress::{
    apply_event, apply_run_progress_to_metadata, derive_progress_counts, derive_stage_message,
    run_progress_from_metadata, RunPhaseId, RunPhaseState, RunProgress, RunProgressEvent,
    RunTaskId,
};
use serde_json::Map;

#[test]
fn shuffled_chunk_completions_are_monotonic() {
    let mut ledger = RunProgress::default();
    // Simulate 16-way fanout: start many, complete in random order.
    let order = [3usize, 0, 7, 1, 15, 2, 14, 4, 13, 5, 12, 6, 11, 8, 10, 9];
    let total = 16u64;
    let mut completed = 0u64;
    for (i, _idx) in order.iter().enumerate() {
        // "Started" heartbeat with last-started index would have been i+1 —
        // we deliberately send completed only.
        completed += 1;
        apply_event(
            &mut ledger,
            RunProgressEvent::Task {
                id: RunTaskId::Chunks,
                done: completed,
                total,
                in_flight: Some((total - completed).min(8)),
            },
            Utc::now(),
        );
        let done = ledger
            .phase(RunPhaseId::Extract)
            .unwrap()
            .task(RunTaskId::Chunks)
            .unwrap()
            .done;
        assert_eq!(done, completed, "at step {i}");
        // Never equals "last started" if that were ahead of completed.
        assert!(done <= (i as u64) + 1);
    }
    assert_eq!(
        ledger
            .phase(RunPhaseId::Extract)
            .unwrap()
            .task(RunTaskId::Chunks)
            .unwrap()
            .done,
        16
    );
}

#[test]
fn figures_after_pages_keep_pages_in_metadata() {
    let mut ledger = RunProgress::default();
    apply_event(
        &mut ledger,
        RunProgressEvent::Task {
            id: RunTaskId::Pages,
            done: 92,
            total: 92,
            in_flight: None,
        },
        Utc::now(),
    );
    apply_event(
        &mut ledger,
        RunProgressEvent::Task {
            id: RunTaskId::Figures,
            done: 3,
            total: 12,
            in_flight: None,
        },
        Utc::now(),
    );
    let mut meta = Map::new();
    apply_run_progress_to_metadata(&mut meta, &ledger);
    let recovered = run_progress_from_metadata(&meta).unwrap();
    assert_eq!(
        recovered
            .phase(RunPhaseId::Prepare)
            .unwrap()
            .task(RunTaskId::Pages)
            .unwrap()
            .done,
        92
    );
    let counts = derive_progress_counts(&recovered).unwrap();
    assert_eq!(counts.unit, "figures");
    assert_eq!(counts.current, 3);
    let msg = derive_stage_message(&recovered);
    assert!(msg.contains("pages"), "{msg}");
    assert!(msg.contains("figures"), "{msg}");
}

#[test]
fn retarget_preserves_prepare_when_extract_starts() {
    let mut ledger = RunProgress::default();
    apply_event(
        &mut ledger,
        RunProgressEvent::Task {
            id: RunTaskId::Pages,
            done: 92,
            total: 92,
            in_flight: None,
        },
        Utc::now(),
    );
    apply_event(
        &mut ledger,
        RunProgressEvent::CompletePhase(RunPhaseId::Prepare),
        Utc::now(),
    );
    // Persist + reload (convert→insert handoff).
    let mut meta = Map::new();
    apply_run_progress_to_metadata(&mut meta, &ledger);
    let mut seeded = run_progress_from_metadata(&meta).unwrap();
    apply_event(
        &mut seeded,
        RunProgressEvent::Task {
            id: RunTaskId::Chunks,
            done: 5,
            total: 40,
            in_flight: Some(3),
        },
        Utc::now(),
    );
    assert_eq!(
        seeded.phase(RunPhaseId::Prepare).unwrap().state,
        RunPhaseState::Done
    );
    assert_eq!(
        seeded
            .phase(RunPhaseId::Prepare)
            .unwrap()
            .task(RunTaskId::Pages)
            .unwrap()
            .done,
        92
    );
}

#[test]
fn reset_clears_for_reprocess() {
    let mut ledger = RunProgress::default();
    apply_event(
        &mut ledger,
        RunProgressEvent::Task {
            id: RunTaskId::Chunks,
            done: 10,
            total: 20,
            in_flight: None,
        },
        Utc::now(),
    );
    apply_event(&mut ledger, RunProgressEvent::Reset, Utc::now());
    assert!(ledger.phase(RunPhaseId::Extract).unwrap().tasks.is_empty());
}
