//! Workspace-aware effective-provider classifier (SPEC-091 hardening).
//!
//! Implements the tasks-crate [`TaskProviderClassifier`] port using the same
//! SSOT as pipeline resolution: `resolve_role_llm(&ws, LlmRole::Extract)`.
//! A workspace whose extract provider is a local model (ollama / lmstudio)
//! classifies as `Local(provider)` and is throttled by that provider's
//! fair-share lane; cloud providers classify as `Cloud` and bypass the local
//! budget entirely.
//!
//! Fail-safe: when the workspace cannot be resolved (deleted mid-queue,
//! transient storage error), the task classifies as `Local` on the default
//! key — it stays throttled exactly as before this hardening, never
//! accidentally unthrottled.

use std::sync::Arc;

use async_trait::async_trait;
use edgequake_core::LlmRole;
use edgequake_tasks::provider_class::{
    SharedTaskProviderClassifier, TaskProviderClass, TaskProviderClassifier, LOCAL_LANE_DEFAULT_KEY,
};
use edgequake_tasks::Task;

use crate::state::SharedWorkspaceService;

/// Classifies tasks by their workspace's effective extract provider.
pub struct WorkspaceProviderClassifier {
    workspace_service: SharedWorkspaceService,
}

impl WorkspaceProviderClassifier {
    pub fn new(workspace_service: SharedWorkspaceService) -> Self {
        Self { workspace_service }
    }

    /// Shared handle for `WorkerPool::with_provider_classifier`.
    pub fn shared(workspace_service: SharedWorkspaceService) -> SharedTaskProviderClassifier {
        Arc::new(Self::new(workspace_service))
    }
}

#[async_trait]
impl TaskProviderClassifier for WorkspaceProviderClassifier {
    async fn classify(&self, task: &Task) -> TaskProviderClass {
        let fail_safe = || TaskProviderClass::Local(LOCAL_LANE_DEFAULT_KEY.to_string());
        let Ok(ws) = self
            .workspace_service
            .get_workspace(task.workspace_id)
            .await
        else {
            return fail_safe();
        };
        let Some(ws) = ws else {
            return fail_safe();
        };
        let provider = edgequake_core::resolve_role_llm(&ws, LlmRole::Extract)
            .provider
            .to_ascii_lowercase();
        if crate::safety_limits::is_slow_local_provider(&provider) {
            TaskProviderClass::Local(provider)
        } else {
            TaskProviderClass::Cloud
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_core::types::{CreateWorkspaceRequest, Tenant};
    use edgequake_core::{InMemoryWorkspaceService, WorkspaceService};
    use edgequake_tasks::types::{TaskType, TextInsertData};
    use uuid::Uuid;

    fn make_task(workspace_id: Uuid) -> Task {
        Task::new(
            Uuid::new_v4(),
            workspace_id,
            TaskType::Insert,
            serde_json::to_value(TextInsertData {
                text: "body".to_string(),
                file_source: "t".to_string(),
                workspace_id: workspace_id.to_string(),
                metadata: None,
                reuse_excluded_pages: None,
            })
            .unwrap(),
        )
    }

    async fn service_with_workspace(provider: &str) -> (SharedWorkspaceService, Uuid) {
        // Isolate from developer shells where mock is healed away.
        unsafe {
            std::env::set_var("EDGEQUAKE_ALLOW_MOCK_PROVIDER", "1");
        }
        let svc = Arc::new(InMemoryWorkspaceService::new());
        let tenant = svc
            .create_tenant(Tenant::new("t", format!("t-{}", Uuid::new_v4())))
            .await
            .unwrap();
        let ws = svc
            .create_workspace(
                tenant.tenant_id,
                CreateWorkspaceRequest {
                    name: "w".to_string(),
                    slug: Some(format!("w-{}", Uuid::new_v4())),
                    llm_provider: Some(provider.to_string()),
                    llm_model: Some("m".to_string()),
                    embedding_provider: Some("mock".to_string()),
                    embedding_model: Some("e".to_string()),
                    embedding_dimension: Some(1536),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        (svc, ws.workspace_id)
    }

    #[tokio::test]
    async fn local_workspace_classifies_local_with_provider_key() {
        let (svc, ws_id) = service_with_workspace("ollama").await;
        let class = WorkspaceProviderClassifier::new(svc)
            .classify(&make_task(ws_id))
            .await;
        assert_eq!(class, TaskProviderClass::Local("ollama".to_string()));
    }

    #[tokio::test]
    async fn cloud_workspace_classifies_cloud() {
        let (svc, ws_id) = service_with_workspace("openai").await;
        let class = WorkspaceProviderClassifier::new(svc)
            .classify(&make_task(ws_id))
            .await;
        assert_eq!(class, TaskProviderClass::Cloud);
    }

    #[tokio::test]
    async fn missing_workspace_fails_safe_to_local_default() {
        let svc: SharedWorkspaceService = Arc::new(InMemoryWorkspaceService::new());
        let class = WorkspaceProviderClassifier::new(svc)
            .classify(&make_task(Uuid::new_v4()))
            .await;
        assert_eq!(
            class,
            TaskProviderClass::Local(LOCAL_LANE_DEFAULT_KEY.to_string())
        );
    }
}
