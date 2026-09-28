//! SPEC-091 R-18 contract — durable fairness-park marker (LAW-Q2/Q5).
//!
//! Before the marker, a parked task stayed plain `pending`, so every idle
//! worker re-claimed it on each 2s poll, hit "already parked", and released
//! it — a hot claim/release spin of wasted DB writes. The durable marker
//! (migration 111, state-machine guard `CLAIM_PENDING_GUARD_SQL`) makes
//! `claim_next` skip parked rows until the park waiter's re-wake clears it.
//!
//! These contracts pin the memory adapter semantics that mirror the Postgres
//! guard; the SQL drift test in `state_machine.rs` pins the SSOT string.

use std::time::Duration;

use edgequake_tasks::memory::MemoryTaskStorage;
use edgequake_tasks::{Task, TaskStatus, TaskStorage, TaskType, TextInsertData};
use uuid::Uuid;

fn make_task() -> Task {
    let workspace_id = Uuid::new_v4();
    Task::new(
        Uuid::new_v4(),
        workspace_id,
        TaskType::Insert,
        serde_json::to_value(TextInsertData {
            text: "body".to_string(),
            file_source: "test".to_string(),
            workspace_id: workspace_id.to_string(),
            metadata: None,
            reuse_excluded_pages: None,
        })
        .unwrap(),
    )
}

async fn enqueue(storage: &MemoryTaskStorage) -> Task {
    let task = make_task();
    storage.create_task(&task).await.unwrap();
    task
}

#[tokio::test]
async fn contract_spec091_parked_task_is_not_claimable() {
    let storage = MemoryTaskStorage::new();
    let task = enqueue(&storage).await;

    // Worker claims the task, then the fair-share lane reports AtCapacity and
    // the worker parks it (release + marker in one step).
    let claimed = storage
        .claim_next("w1", Duration::from_secs(60))
        .await
        .unwrap()
        .expect("task must be claimable before park");
    assert_eq!(claimed.track_id, task.track_id);
    let lease_token = claimed.lease_token.expect("claim sets a lease token");

    assert!(storage
        .mark_fairness_parked(&task.track_id, "w1", lease_token)
        .await
        .unwrap());

    // The spin regression: repeated polls must NOT return the parked task.
    for _ in 0..8 {
        assert!(storage
            .claim_next("w2", Duration::from_secs(60))
            .await
            .unwrap()
            .is_none());
    }

    // Status is still pending — park is scheduling state, not lifecycle.
    let stored = storage.get_task(&task.track_id).await.unwrap().unwrap();
    assert_eq!(stored.status, TaskStatus::Pending);
}

#[tokio::test]
async fn contract_spec091_park_clear_reenables_claim() {
    let storage = MemoryTaskStorage::new();
    let task = enqueue(&storage).await;
    let claimed = storage
        .claim_next("w1", Duration::from_secs(60))
        .await
        .unwrap()
        .unwrap();
    let lease_token = claimed.lease_token.unwrap();

    assert!(storage
        .mark_fairness_parked(&task.track_id, "w1", lease_token)
        .await
        .unwrap());
    assert!(storage
        .claim_next("w2", Duration::from_secs(60))
        .await
        .unwrap()
        .is_none());

    // Park waiter fired → marker cleared before queue re-wake → claimable.
    storage.clear_fairness_park(&task.track_id).await.unwrap();
    let reclaimed = storage
        .claim_next("w2", Duration::from_secs(60))
        .await
        .unwrap();
    assert_eq!(reclaimed.map(|t| t.track_id), Some(task.track_id));
}

#[tokio::test]
async fn contract_spec091_stale_park_sweep_recovers_rows() {
    let storage = MemoryTaskStorage::new();
    let task = enqueue(&storage).await;
    let claimed = storage
        .claim_next("w1", Duration::from_secs(60))
        .await
        .unwrap()
        .unwrap();
    let lease_token = claimed.lease_token.unwrap();

    assert!(storage
        .mark_fairness_parked(&task.track_id, "w1", lease_token)
        .await
        .unwrap());

    // Fresh boot (age 0): no park waiter can be alive — sweep everything.
    let cleared = storage
        .clear_stale_fairness_parks(Duration::ZERO)
        .await
        .unwrap();
    assert_eq!(cleared, 1);
    let reclaimed = storage
        .claim_next("w2", Duration::from_secs(60))
        .await
        .unwrap();
    assert_eq!(reclaimed.map(|t| t.track_id), Some(task.track_id));
}

#[tokio::test]
async fn contract_spec091_mark_park_requires_lease_ownership() {
    let storage = MemoryTaskStorage::new();
    let task = enqueue(&storage).await;
    let claimed = storage
        .claim_next("w1", Duration::from_secs(60))
        .await
        .unwrap()
        .unwrap();
    let lease_token = claimed.lease_token.unwrap();

    // Wrong worker / wrong token cannot park (lease CAS discipline).
    assert!(!storage
        .mark_fairness_parked(&task.track_id, "w2", lease_token)
        .await
        .unwrap());
    assert!(!storage
        .mark_fairness_parked(&task.track_id, "w1", Uuid::new_v4())
        .await
        .unwrap());

    // And the task is still owned by w1 (no accidental release).
    let stored = storage.get_task(&task.track_id).await.unwrap().unwrap();
    assert_eq!(stored.status, TaskStatus::Processing);
    assert_eq!(stored.lease_owner.as_deref(), Some("w1"));
}
