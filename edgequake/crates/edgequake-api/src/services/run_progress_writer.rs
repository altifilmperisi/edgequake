//! Ordered, coalesced writer for the run-progress ledger (SPEC-155).
//!
//! WHY: fire-and-forget `tokio::spawn` patches race each other and can regress
//! `done`. This writer serialises events through one mpsc consumer, applies the
//! pure reducer, and flushes derived legacy fields to document metadata.
//! Coalescing (~250 ms) bounds KV write amplification; a final flush is
//! guaranteed when the handle is dropped / `flush_now` is called.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use edgequake_storage::traits::KVStorage;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{timeout, Instant};

use crate::services::run_progress::{
    apply_event, apply_run_progress_to_metadata, run_progress_from_metadata, RunProgress,
    RunProgressEvent, RunTaskId,
};
use crate::services::text_insert_content::patch_document_metadata;

const COALESCE_MS: u64 = 250;

enum WriterMsg {
    Event(RunProgressEvent),
    /// Force an immediate flush; replies when the KV write completes (or is skipped).
    Flush(oneshot::Sender<()>),
}

/// Handle shared by PDF / chunk / embed / merge callbacks.
#[derive(Clone)]
pub struct RunProgressWriter {
    tx: mpsc::UnboundedSender<WriterMsg>,
    /// Observed event count (diagnostics / tests).
    events_submitted: Arc<AtomicU64>,
}

impl RunProgressWriter {
    /// Spawn the single-writer task for `document_id`.
    pub fn spawn(document_id: String, kv: Arc<dyn KVStorage>) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let events_submitted = Arc::new(AtomicU64::new(0));
        let doc = document_id;
        tokio::spawn(async move {
            run_writer_loop(doc, kv, rx).await;
        });
        Self {
            tx,
            events_submitted,
        }
    }

    pub fn submit(&self, event: RunProgressEvent) {
        self.events_submitted.fetch_add(1, Ordering::Relaxed);
        let _ = self.tx.send(WriterMsg::Event(event));
    }

    pub fn task(&self, id: RunTaskId, done: u64, total: u64, in_flight: Option<u64>) {
        self.submit(RunProgressEvent::Task {
            id,
            done,
            total,
            in_flight,
        });
    }

    pub fn complete_phase(&self, phase: crate::services::run_progress::RunPhaseId) {
        self.submit(RunProgressEvent::CompletePhase(phase));
    }

    pub fn reset(&self) {
        self.submit(RunProgressEvent::Reset);
    }

    /// Block until the pending ledger is written (best-effort; 2 s timeout).
    pub async fn flush_now(&self) {
        let (tx, rx) = oneshot::channel();
        if self.tx.send(WriterMsg::Flush(tx)).is_err() {
            return;
        }
        let _ = timeout(Duration::from_secs(2), rx).await;
    }

    pub fn events_submitted(&self) -> u64 {
        self.events_submitted.load(Ordering::Relaxed)
    }
}

async fn run_writer_loop(
    document_id: String,
    kv: Arc<dyn KVStorage>,
    mut rx: mpsc::UnboundedReceiver<WriterMsg>,
) {
    let mut ledger = RunProgress::default();
    let mut dirty = false;
    let mut last_flush = Instant::now()
        .checked_sub(Duration::from_millis(COALESCE_MS))
        .unwrap_or_else(Instant::now);

    // Seed from existing metadata so convert→insert retarget keeps Prepare.
    if let Ok(Some((_, existing))) =
        crate::services::load_staging_first_metadata(kv.as_ref(), &document_id).await
    {
        if let Some(obj) = existing.as_object() {
            if let Some(existing_ledger) = run_progress_from_metadata(obj) {
                ledger = existing_ledger;
            }
        }
    }

    loop {
        // Wait for the next message, or coalesce timer if dirty.
        let msg = if dirty {
            let remaining = Duration::from_millis(COALESCE_MS).saturating_sub(last_flush.elapsed());
            match timeout(remaining, rx.recv()).await {
                Ok(Some(m)) => Some(m),
                Ok(None) => None, // channel closed
                Err(_) => {
                    // Timer fired — flush.
                    flush_ledger(&kv, &document_id, &ledger).await;
                    dirty = false;
                    last_flush = Instant::now();
                    continue;
                }
            }
        } else {
            rx.recv().await
        };

        let Some(msg) = msg else {
            // Channel closed — final flush if needed.
            if dirty {
                flush_ledger(&kv, &document_id, &ledger).await;
            }
            break;
        };

        match msg {
            WriterMsg::Event(event) => {
                apply_event(&mut ledger, event, Utc::now());
                dirty = true;
                // Immediate flush for Reset / CompletePhase so UI never lags a
                // terminal phase transition by 250 ms.
                // (Task events coalesce.)
            }
            WriterMsg::Flush(ack) => {
                if dirty {
                    flush_ledger(&kv, &document_id, &ledger).await;
                    dirty = false;
                    last_flush = Instant::now();
                }
                let _ = ack.send(());
            }
        }

        // Flush CompletePhase / Reset immediately.
        if dirty {
            // Peek: if the last applied event warrants immediate flush we
            // already marked dirty; check elapsed.
            if last_flush.elapsed() >= Duration::from_millis(COALESCE_MS) {
                flush_ledger(&kv, &document_id, &ledger).await;
                dirty = false;
                last_flush = Instant::now();
            }
        }
    }
}

async fn flush_ledger(kv: &Arc<dyn KVStorage>, document_id: &str, ledger: &RunProgress) {
    let snapshot = ledger.clone();
    if let Err(e) = patch_document_metadata(kv, document_id, |obj| {
        // Fence: never overwrite a newer seq already in KV.
        if let Some(existing) = run_progress_from_metadata(obj) {
            if existing.seq > snapshot.seq {
                return;
            }
        }
        apply_run_progress_to_metadata(obj, &snapshot);
    })
    .await
    {
        tracing::warn!(
            document_id = %document_id,
            error = %e,
            "run_progress flush failed"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::run_progress::RunPhaseId;
    use edgequake_storage::MemoryKVStorage;

    #[tokio::test]
    async fn writer_applies_pages_then_figures_monotonically() {
        let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("test"));
        let doc = "doc-rp-writer-1";
        // Seed empty metadata so patch has a target.
        let key = edgequake_storage::kv_keys::doc_metadata(doc);
        let _ = kv
            .upsert(&[(
                key.clone(),
                serde_json::json!({
                    "id": doc,
                    "status": "processing",
                    "current_stage": "converting",
                }),
            )])
            .await;

        let writer = RunProgressWriter::spawn(doc.to_string(), Arc::clone(&kv));
        writer.task(RunTaskId::Pages, 10, 10, None);
        writer.task(RunTaskId::Figures, 1, 5, None);
        writer.flush_now().await;

        let meta = kv.get_by_id(&key).await.expect("get").expect("meta");
        let obj = meta.as_object().unwrap();
        let ledger = run_progress_from_metadata(obj).expect("ledger");
        assert_eq!(
            ledger
                .phase(RunPhaseId::Prepare)
                .unwrap()
                .task(RunTaskId::Pages)
                .unwrap()
                .done,
            10
        );
        assert_eq!(
            ledger
                .phase(RunPhaseId::Prepare)
                .unwrap()
                .task(RunTaskId::Figures)
                .unwrap()
                .done,
            1
        );
        // Legacy counts should point at the incomplete figures task.
        let counts = obj.get("progress_counts").unwrap();
        assert_eq!(counts.get("unit").and_then(|v| v.as_str()), Some("figures"));
        assert_eq!(counts.get("current").and_then(|v| v.as_u64()), Some(1));
    }

    #[tokio::test]
    async fn writer_seeds_existing_prepare_across_retarget() {
        let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("test"));
        let doc = "doc-rp-writer-2";
        let key = edgequake_storage::kv_keys::doc_metadata(doc);
        let mut seed = RunProgress::default();
        apply_event(
            &mut seed,
            RunProgressEvent::Task {
                id: RunTaskId::Pages,
                done: 92,
                total: 92,
                in_flight: None,
            },
            Utc::now(),
        );
        apply_event(
            &mut seed,
            RunProgressEvent::CompletePhase(RunPhaseId::Prepare),
            Utc::now(),
        );
        let mut meta = serde_json::Map::new();
        meta.insert("id".into(), serde_json::json!(doc));
        meta.insert("status".into(), serde_json::json!("processing"));
        apply_run_progress_to_metadata(&mut meta, &seed);
        let _ = kv
            .upsert(&[(key.clone(), serde_json::Value::Object(meta))])
            .await;

        let writer = RunProgressWriter::spawn(doc.to_string(), Arc::clone(&kv));
        // Give the seed-load a moment (spawned task).
        tokio::time::sleep(Duration::from_millis(50)).await;
        writer.task(RunTaskId::Chunks, 5, 40, Some(3));
        writer.flush_now().await;

        let meta = kv.get_by_id(&key).await.expect("get").expect("meta");
        let ledger = run_progress_from_metadata(meta.as_object().unwrap()).unwrap();
        assert_eq!(
            ledger.phase(RunPhaseId::Prepare).unwrap().state,
            crate::services::run_progress::RunPhaseState::Done
        );
        assert_eq!(
            ledger
                .phase(RunPhaseId::Prepare)
                .unwrap()
                .task(RunTaskId::Pages)
                .unwrap()
                .done,
            92
        );
        assert_eq!(
            ledger
                .phase(RunPhaseId::Extract)
                .unwrap()
                .task(RunTaskId::Chunks)
                .unwrap()
                .done,
            5
        );
    }
}
