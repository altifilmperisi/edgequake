//! Startup pending-task hydrate (SPEC-045 / SPEC-054 #298-B).
//!
//! SSOT for reloading `TaskStatus::Pending` rows from durable storage into the
//! in-process `ChannelTaskQueue`. Must run **after** workers are polling so
//! bounded-channel backpressure can drain instead of blocking HTTP bind.
//!
//! Default policy (reliable Vision ingestion): **auto-resume on**. Interrupted
//! Processing tasks become Pending and resume from durable checkpoints so a
//! restart does not strand docs as "Task heartbeat lost". Opt out with
//! `EDGEQUAKE_STARTUP_AUTO_RESUME=0` (or `false` / `off` / `no`) to require
//! manual reprocess (avoids surprise spend on every deploy if desired).

use std::sync::Arc;

use edgequake_tasks::storage::{Pagination, TaskFilter, TaskStorage};
use edgequake_tasks::{TaskQueue, TaskStatus};
use tracing::info;

/// When `true`, boot recovers orphans to `pending` and hydrates the queue.
///
/// Default `true`: resume interrupted work from checkpoints (fail-closed Vision
/// reliability). Set `EDGEQUAKE_STARTUP_AUTO_RESUME=0` to mark orphans failed
/// and require manual reprocess.
pub fn startup_auto_resume_enabled() -> bool {
    match std::env::var("EDGEQUAKE_STARTUP_AUTO_RESUME") {
        Ok(v) => {
            let lower = v.to_ascii_lowercase();
            !matches!(lower.as_str(), "0" | "false" | "no" | "off")
        }
        // Unset → default ON (reliable resume).
        Err(_) => true,
    }
}

/// Report from a startup (or periodic) pending-task hydrate pass.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct RequeuePendingReport {
    pub requeued: usize,
    pub failed: usize,
}

/// Paginated hydrate of pending tasks from storage into the worker channel.
///
/// SPEC-045 SRE-I02: page size 500 — large outages may have thousands pending.
pub async fn requeue_pending_tasks(
    task_storage: Arc<dyn TaskStorage>,
    task_queue: Arc<dyn TaskQueue>,
) -> edgequake_tasks::error::TaskResult<RequeuePendingReport> {
    info!("🔄 Checking for pending tasks to requeue from database...");

    let filter = TaskFilter {
        status: Some(TaskStatus::Pending),
        ..Default::default()
    };

    let mut report = RequeuePendingReport::default();
    let mut page = 1;
    // SPEC-045 SRE-I02: page size 500 — large outages may have thousands pending.
    let page_size = 500;

    loop {
        let pagination = Pagination {
            page,
            page_size,
            ..Default::default()
        };

        let task_list = task_storage.list_tasks(filter.clone(), pagination).await?;
        let batch_len = task_list.tasks.len();

        if batch_len == 0 && page == 1 {
            info!("✅ No pending tasks to requeue");
            return Ok(report);
        }

        if page == 1 && batch_len > 0 {
            info!("📋 Found pending task(s) in database, requeueing to worker pool...");
        }

        for task in task_list.tasks {
            match task_queue.send(task.clone()).await {
                Ok(_) => {
                    info!("✅ Requeued task: {}", task.track_id);
                    report.requeued += 1;
                }
                Err(e) => {
                    tracing::warn!(
                        task_id = %task.track_id,
                        error = %e,
                        "Failed to requeue pending task (non-fatal)"
                    );
                    report.failed += 1;
                }
            }
        }

        if batch_len < page_size as usize {
            break;
        }
        page += 1;
    }

    info!(
        requeued = report.requeued,
        failed = report.failed,
        "🔧 Pending task requeue complete"
    );

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_tasks::memory::MemoryTaskStorage;
    use edgequake_tasks::queue::ChannelTaskQueue;
    use edgequake_tasks::{Task, TaskType, TextInsertData};
    use uuid::Uuid;

    #[test]
    fn startup_auto_resume_defaults_on() {
        let prev = std::env::var("EDGEQUAKE_STARTUP_AUTO_RESUME").ok();
        std::env::remove_var("EDGEQUAKE_STARTUP_AUTO_RESUME");
        assert!(startup_auto_resume_enabled());
        std::env::set_var("EDGEQUAKE_STARTUP_AUTO_RESUME", "0");
        assert!(!startup_auto_resume_enabled());
        std::env::set_var("EDGEQUAKE_STARTUP_AUTO_RESUME", "false");
        assert!(!startup_auto_resume_enabled());
        std::env::set_var("EDGEQUAKE_STARTUP_AUTO_RESUME", "1");
        assert!(startup_auto_resume_enabled());
        std::env::remove_var("EDGEQUAKE_STARTUP_AUTO_RESUME");
        if let Some(v) = prev {
            std::env::set_var("EDGEQUAKE_STARTUP_AUTO_RESUME", v);
        }
    }

    fn minimal_insert_task(n: usize) -> Task {
        let tenant = Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap();
        let workspace = Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap();
        let data = TextInsertData {
            text: format!("hydrate soak doc {n}"),
            file_source: format!("doc-{n}.md"),
            workspace_id: workspace.to_string(),
            metadata: Some(serde_json::json!({ "document_id": format!("doc-{n}") })),
            reuse_excluded_pages: None,
        };
        Task::new(
            tenant,
            workspace,
            TaskType::Insert,
            serde_json::to_value(&data).unwrap(),
        )
    }

    /// SPEC-054/#298-B: >100 pending tasks must hydrate when consumers are active.
    #[tokio::test]
    async fn requeue_over_channel_capacity_with_active_consumer() {
        const PENDING_COUNT: usize = 150;
        const CHANNEL_CAPACITY: usize = 100;

        let storage = Arc::new(MemoryTaskStorage::new());
        let queue = Arc::new(ChannelTaskQueue::new(CHANNEL_CAPACITY));

        for i in 0..PENDING_COUNT {
            let task = minimal_insert_task(i);
            storage.create_task(&task).await.unwrap();
        }

        // Simulate workers polling BEFORE hydrate (main.rs ordering).
        let drain_queue = Arc::clone(&queue) as Arc<dyn TaskQueue>;
        let consumer = tokio::spawn(async move {
            let mut drained = 0usize;
            while drained < PENDING_COUNT {
                match drain_queue.receive().await {
                    Ok(_) => drained += 1,
                    Err(_) => break,
                }
            }
            drained
        });

        tokio::time::sleep(std::time::Duration::from_millis(20)).await;

        let hydrate = tokio::spawn({
            let storage = Arc::clone(&storage);
            let queue = Arc::clone(&queue) as Arc<dyn TaskQueue>;
            async move { requeue_pending_tasks(storage, queue).await }
        });

        let report = tokio::time::timeout(std::time::Duration::from_secs(10), hydrate)
            .await
            .expect("hydrate must not deadlock when workers drain channel")
            .expect("hydrate task join")
            .expect("requeue ok");

        assert_eq!(
            report.requeued, PENDING_COUNT,
            "falsifiable: all pending tasks must enter channel when consumers active"
        );
        assert_eq!(report.failed, 0);

        let drained = tokio::time::timeout(std::time::Duration::from_secs(5), consumer)
            .await
            .expect("consumer must drain all requeued tasks")
            .expect("consumer join");
        assert_eq!(drained, PENDING_COUNT);
    }
}
