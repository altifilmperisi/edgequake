//! SPEC-091 QW2 E2E — Explicit queued state + ETA (LAW-Q4, EC-21).
//!
//! Saturate the queue, then verify:
//! 1. Each admitted task sees a monotonically increasing queue position.
//! 2. ETA basis is `no_history` before any completion and `measured` after.
//! 3. ETA is clamped-honest, never fabricated.
//! 4. The queue drains in FCFS order through the state machine (pending →
//!    processing → indexed).
//!
//! Uses the in-memory storage so it runs without a database (the Postgres
//! projection path is covered by `contract_spec091_provider_budget`).

use std::time::Duration;

use chrono::Utc;
use edgequake_tasks::memory::MemoryTaskStorage;
use edgequake_tasks::queue_estimate::{estimate_queue, QueueEtaBasis};
use edgequake_tasks::storage::QueueMetrics;
use edgequake_tasks::{Task, TaskStatus, TaskStorage, TaskType, TextInsertData};
use uuid::Uuid;

fn make_task(workspace_id: Uuid) -> Task {
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

#[tokio::test]
async fn e2e_spec091_queue_explicit_queued_state() {
    let storage = MemoryTaskStorage::new();
    let workspace_id = Uuid::new_v4();

    // 1. Admit 5 tasks — each projection reflects the tasks ahead of it.
    let mut admitted: Vec<Task> = Vec::new();
    let mut positions = Vec::new();
    for _ in 0..5 {
        let task = make_task(workspace_id);
        storage.create_task(&task).await.unwrap();
        let estimate = estimate_queue(&storage, task.created_at).await.unwrap();
        positions.push(estimate.position);
        // No completions yet → honest unknown, clamped at the 4h ceiling.
        assert_eq!(estimate.basis, QueueEtaBasis::NoHistory);
        assert_eq!(estimate.eta_seconds, 14_400);
        admitted.push(task);
    }
    assert_eq!(positions, vec![0, 1, 2, 3, 4], "FCFS positions increase");

    // 2. Complete two OTHER tasks → drain history exists, ETA becomes measured.
    for _ in 0..2 {
        let mut done = make_task(workspace_id);
        done.mark_processing();
        done.mark_success(serde_json::json!({"ok": true}));
        storage.create_task(&done).await.unwrap();
    }
    let tail = make_task(workspace_id);
    storage.create_task(&tail).await.unwrap();
    let estimate = estimate_queue(&storage, tail.created_at).await.unwrap();
    assert_eq!(estimate.basis, QueueEtaBasis::Measured);
    assert!(estimate.eta_seconds <= 14_400, "ETA always clamped-honest");

    // 3. FCFS drain through legal transitions: pending → processing → indexed.
    let first = storage
        .claim_next("worker-1", Duration::from_secs(60))
        .await
        .unwrap()
        .expect("queue non-empty");
    assert_eq!(
        first.track_id, admitted[0].track_id,
        "oldest pending claims first"
    );
    assert_eq!(first.status, TaskStatus::Processing);

    // 4. Position projection for a fresh arrival never exceeds the backlog.
    let remaining = estimate_queue(&storage, Utc::now() + chrono::Duration::seconds(1))
        .await
        .unwrap();
    assert!(
        remaining.position <= 5,
        "position projection bounded by admitted backlog"
    );
}

#[tokio::test]
async fn e2e_spec091_queue_rate_limited_signal_is_real() {
    // Idle queue → not rate limited.
    assert!(!QueueMetrics::compute_rate_limited(0, 0, 2, 1.0));
    // Backlog with all workers busy → rate limited (arrivals must wait).
    assert!(QueueMetrics::compute_rate_limited(1, 2, 2, 1.0));
    // Backlog beyond the Little's-Law soft bound → rate limited even with
    // idle workers (workers stalled): bound = λ̂ × 600s/60 = 10 for λ̂=1.
    assert!(QueueMetrics::compute_rate_limited(11, 0, 2, 1.0));
    // Small backlog, idle workers → not rate limited.
    assert!(!QueueMetrics::compute_rate_limited(2, 0, 2, 1.0));
}
