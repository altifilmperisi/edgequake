//! Debounced community index refresh scheduler (SPEC-024 Phase 4.2 / SRP).
//!
//! Coalesces burst ingests into one Louvain run per workspace window.
//! Called from [`crate::community_persist::schedule_community_index_refresh`].
//!
//! SPEC-046 EQ-046-11: optional community_report vector indexing when an
//! embedder + vector store are supplied (DIP — storage never imports LLM).
//!
//! GH-404: workspace-scoped load, process-local single-flight + dirty bit,
//! fail-closed size gate, and cross-replica advisory lock.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::Duration;

use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::community::CommunityConfig;
use crate::community_persist::{community_features_enabled, detect_and_persist_communities};
use crate::community_reports::{community_reports_enabled, index_community_reports_with_embedder};
use crate::traits::{GraphStorage, TextEmbedder, VectorStorage};

/// Debounce window for post-ingest community refresh (default 300s).
pub fn community_refresh_debounce_secs() -> u64 {
    std::env::var("EDGEQUAKE_COMMUNITY_REFRESH_DEBOUNCE_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(300)
}

fn debounce_duration() -> Duration {
    Duration::from_secs(community_refresh_debounce_secs())
}

/// Optional vector indexing hooks for community reports (SPEC-046).
#[derive(Clone, Default)]
pub struct CommunityRefreshExtras {
    pub vector_storage: Option<Arc<dyn VectorStorage>>,
    pub embedder: Option<Arc<dyn TextEmbedder>>,
    pub tenant_id: Option<String>,
}

impl CommunityRefreshExtras {
    pub fn with_report_indexing(
        vector_storage: Arc<dyn VectorStorage>,
        embedder: Arc<dyn TextEmbedder>,
    ) -> Self {
        Self {
            vector_storage: Some(vector_storage),
            embedder: Some(embedder),
            tenant_id: None,
        }
    }

    pub fn with_tenant_id(mut self, tenant_id: Option<String>) -> Self {
        self.tenant_id = tenant_id;
        self
    }
}

/// Schedule a debounced community refresh for a workspace (SPEC-024 1.3).
pub async fn schedule_community_index_refresh(
    graph: Arc<dyn GraphStorage>,
    workspace_id: Option<String>,
) {
    schedule_community_index_refresh_with_extras(
        graph,
        workspace_id,
        CommunityRefreshExtras::default(),
    )
    .await;
}

/// Schedule refresh and optionally index community_report vectors after Louvain.
pub async fn schedule_community_index_refresh_with_extras(
    graph: Arc<dyn GraphStorage>,
    workspace_id: Option<String>,
    extras: CommunityRefreshExtras,
) {
    if !community_features_enabled() {
        return;
    }
    community_scheduler()
        .schedule(workspace_id, graph, extras)
        .await;
}

struct InFlightState {
    dirty: bool,
}

struct CommunityRefreshScheduler {
    timers: Mutex<HashMap<String, JoinHandle<()>>>,
    /// Process-local single-flight: key → running refresh (GH-404).
    inflight: Mutex<HashMap<String, InFlightState>>,
}

impl CommunityRefreshScheduler {
    async fn pending_workspace_count(&self) -> usize {
        self.timers.lock().await.len()
    }

    async fn schedule(
        &self,
        workspace_id: Option<String>,
        graph: Arc<dyn GraphStorage>,
        extras: CommunityRefreshExtras,
    ) {
        let key = workspace_id
            .clone()
            .unwrap_or_else(|| "default".to_string());

        // GH-404: coalesce into the in-flight run instead of stacking another Louvain.
        {
            let mut inflight = self.inflight.lock().await;
            if let Some(state) = inflight.get_mut(&key) {
                state.dirty = true;
                return;
            }
        }

        let debounce = debounce_duration();

        let mut timers = self.timers.lock().await;
        if let Some(handle) = timers.remove(&key) {
            handle.abort();
        }

        let handle = tokio::spawn(async move {
            tokio::time::sleep(debounce).await;
            community_scheduler()
                .run_guarded(graph, workspace_id, extras)
                .await;
        });

        timers.insert(key, handle);
    }

    async fn run_guarded(
        &self,
        graph: Arc<dyn GraphStorage>,
        workspace_id: Option<String>,
        extras: CommunityRefreshExtras,
    ) {
        let key = workspace_id
            .clone()
            .unwrap_or_else(|| "default".to_string());

        {
            let mut inflight = self.inflight.lock().await;
            if let Some(state) = inflight.get_mut(&key) {
                state.dirty = true;
                return;
            }
            inflight.insert(key.clone(), InFlightState { dirty: false });
        }

        {
            let mut timers = self.timers.lock().await;
            timers.remove(&key);
        }

        drain_dirty_inflight(&self.inflight, &key, || {
            let graph = graph.clone();
            let workspace_id = workspace_id.clone();
            let extras = extras.clone();
            async move {
                run_community_refresh(graph, workspace_id, extras).await;
            }
        })
        .await;
    }
}

/// Process-local dirty drain (GH-404): run `work` once, then again while dirty.
async fn drain_dirty_inflight<F, Fut>(
    inflight: &Mutex<HashMap<String, InFlightState>>,
    key: &str,
    mut work: F,
) where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    loop {
        work().await;
        let mut map = inflight.lock().await;
        if let Some(state) = map.get_mut(key) {
            if state.dirty {
                state.dirty = false;
                continue;
            }
            map.remove(key);
        }
        break;
    }
}

fn community_scheduler() -> &'static CommunityRefreshScheduler {
    static SCHEDULER: OnceLock<CommunityRefreshScheduler> = OnceLock::new();
    SCHEDULER.get_or_init(|| CommunityRefreshScheduler {
        timers: Mutex::new(HashMap::new()),
        inflight: Mutex::new(HashMap::new()),
    })
}

async fn run_community_refresh(
    graph: Arc<dyn GraphStorage>,
    workspace_id: Option<String>,
    extras: CommunityRefreshExtras,
) {
    if !community_features_enabled() {
        return;
    }

    let lock_key = workspace_id
        .clone()
        .unwrap_or_else(|| "default".to_string());

    match graph.try_community_refresh_advisory_lock(&lock_key).await {
        Ok(true) => {}
        Ok(false) => {
            tracing::debug!(
                workspace = %lock_key,
                "Skipping community refresh — advisory lock held by another replica (GH-404)"
            );
            return;
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                "Community refresh advisory lock failed — skipping (fail-closed)"
            );
            return;
        }
    }

    let outcome = async {
        // GH-404: fail-closed size gate — count errors skip the heavy path.
        let threshold = crate::community_persist::community_auto_max_nodes();
        let count_result =
            community_refresh_node_count(graph.as_ref(), workspace_id.as_deref()).await;
        if community_refresh_should_skip(count_result.as_ref().copied(), threshold) {
            match &count_result {
                Ok(node_count) => {
                    tracing::warn!(
                        node_count,
                        threshold,
                        workspace_id = workspace_id.as_deref(),
                        "Skipping community index refresh — graph too large"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        workspace_id = workspace_id.as_deref(),
                        "Skipping community index refresh — node count failed (fail-closed GH-404)"
                    );
                }
            }
            return Ok::<(), crate::error::StorageError>(());
        }

        let config = CommunityConfig {
            workspace_id: workspace_id.clone(),
            tenant_id: extras.tenant_id.clone(),
            ..CommunityConfig::default()
        };

        let result = detect_and_persist_communities(graph.clone(), &config).await?;
        tracing::debug!(
            communities = result.communities.len(),
            labeled_nodes = result.node_to_community.len(),
            workspace_id = workspace_id.as_deref(),
            "Community index refreshed after ingest"
        );
        maybe_index_community_reports(&result, workspace_id.as_deref(), &extras).await;
        Ok(())
    }
    .await;

    if let Err(e) = graph
        .release_community_refresh_advisory_lock(&lock_key)
        .await
    {
        tracing::warn!(error = %e, "Community refresh advisory unlock failed");
    }

    if let Err(e) = outcome {
        tracing::warn!(
            error = %e,
            "Community index refresh failed (non-fatal)"
        );
    }
}

/// Resolve node count for the refresh size gate (workspace-scoped when possible).
async fn community_refresh_node_count(
    graph: &dyn GraphStorage,
    workspace_id: Option<&str>,
) -> crate::error::Result<usize> {
    match workspace_id.and_then(|w| uuid::Uuid::parse_str(w).ok()) {
        Some(ws) => graph.node_count_by_workspace(&ws).await,
        None => graph.node_count_fast().await,
    }
}

/// True when a count result should skip detection (error or over threshold).
pub(crate) fn community_refresh_should_skip(
    count: Result<usize, &crate::error::StorageError>,
    threshold: usize,
) -> bool {
    match count {
        Ok(n) => n > threshold,
        Err(_) => true,
    }
}

async fn maybe_index_community_reports(
    result: &crate::community::CommunityDetectionResult,
    workspace_id: Option<&str>,
    extras: &CommunityRefreshExtras,
) {
    if !community_reports_enabled() {
        return;
    }
    let (Some(vs), Some(embedder)) = (extras.vector_storage.as_ref(), extras.embedder.as_ref())
    else {
        tracing::debug!(
            "SPEC-046: community reports enabled but embedder/vector missing — props only"
        );
        return;
    };
    match index_community_reports_with_embedder(
        result,
        vs.as_ref(),
        embedder.as_ref(),
        workspace_id,
        extras.tenant_id.as_deref(),
    )
    .await
    {
        Ok(n) => tracing::debug!(
            reports_indexed = n,
            "SPEC-046: community_report vectors upserted"
        ),
        Err(e) => tracing::warn!(
            error = %e,
            "SPEC-046: community_report vector index failed (non-fatal)"
        ),
    }
}

/// Number of workspaces with a debounced community refresh timer pending (scale signal).
pub async fn pending_community_refresh_workspaces() -> usize {
    if !community_features_enabled() {
        return 0;
    }
    community_scheduler().pending_workspace_count().await
}

/// Run community refresh immediately (tests / one-shot hooks).
pub async fn refresh_community_index_now(graph: Arc<dyn GraphStorage>) {
    run_community_refresh(graph, None, CommunityRefreshExtras::default()).await;
}

/// Immediate refresh + optional report vector indexing (tests / ops).
pub async fn refresh_community_index_now_with_extras(
    graph: Arc<dyn GraphStorage>,
    workspace_id: Option<String>,
    extras: CommunityRefreshExtras,
) {
    run_community_refresh(graph, workspace_id, extras).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::StorageError;

    #[test]
    fn debounce_default_is_five_minutes() {
        std::env::remove_var("EDGEQUAKE_COMMUNITY_REFRESH_DEBOUNCE_SECS");
        assert_eq!(community_refresh_debounce_secs(), 300);
    }

    #[test]
    fn fail_closed_size_gate_skips_on_count_error() {
        let err = StorageError::Database("count failed".into());
        assert!(community_refresh_should_skip(Err(&err), 50_000));
        assert!(community_refresh_should_skip(Ok(50_001), 50_000));
        assert!(!community_refresh_should_skip(Ok(49_999), 50_000));
    }

    #[tokio::test]
    async fn schedule_while_inflight_marks_dirty_not_second_timer() {
        let sched = community_scheduler();
        let key = "ws-single-flight-test";
        {
            let mut inflight = sched.inflight.lock().await;
            inflight.insert(key.to_string(), InFlightState { dirty: false });
        }
        let graph: Arc<dyn GraphStorage> =
            Arc::new(crate::adapters::memory::MemoryGraphStorage::new("sf"));
        sched
            .schedule(
                Some(key.to_string()),
                graph,
                CommunityRefreshExtras::default(),
            )
            .await;
        {
            let inflight = sched.inflight.lock().await;
            let state = inflight.get(key).expect("still inflight");
            assert!(state.dirty, "re-schedule during inflight must set dirty");
        }
        {
            let timers = sched.timers.lock().await;
            assert!(
                !timers.contains_key(key),
                "must not start a second debounce timer while inflight"
            );
        }
        // cleanup
        sched.inflight.lock().await.remove(key);
    }

    #[tokio::test]
    async fn dirty_drain_runs_work_a_second_time() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let inflight = Mutex::new(HashMap::new());
        let key = "dirty-rerun";
        inflight
            .lock()
            .await
            .insert(key.to_string(), InFlightState { dirty: false });

        let runs = AtomicUsize::new(0);
        drain_dirty_inflight(&inflight, key, || {
            let runs = &runs;
            let inflight = &inflight;
            async move {
                let n = runs.fetch_add(1, Ordering::SeqCst);
                if n == 0 {
                    if let Some(state) = inflight.lock().await.get_mut(key) {
                        state.dirty = true;
                    }
                }
            }
        })
        .await;

        assert_eq!(
            runs.load(Ordering::SeqCst),
            2,
            "dirty bit mid-flight must trigger exactly one extra run"
        );
        assert!(
            !inflight.lock().await.contains_key(key),
            "inflight entry must be cleared after drain"
        );
    }
}
