//! Provider classification for fair-share lane routing (SPEC-091 hardening,
//! LAW-Q5 refinement: limits are provider-limited, keyed by the local model).
//!
//! The fair-share ingest lane exists because a *local* provider's capacity is
//! the scarce cluster resource. A task whose **effective** extract provider is
//! a cloud API (workspace override) does not consume that resource, so it must
//! not wait on the local-budget lane — otherwise cloud tasks get throttled by
//! an unrelated local model's saturation (observed in mixed deployments:
//! server default ollama, workspace override mistral).
//!
//! The classifier is a port (SOLID): the tasks crate owns the contract, the
//! API layer supplies the workspace-aware implementation. Claim-time
//! classification (not enqueue-time) keeps the decision fresh when a
//! workspace's provider changes while the task sits queued.

use async_trait::async_trait;

use crate::types::Task;

/// Effective provider class for a task's extraction work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskProviderClass {
    /// Local provider (ollama / lmstudio / …) — the provider budget is the
    /// scarce resource; the fair-share lane applies, keyed by provider name.
    Local(String),
    /// Cloud/external provider — bypasses the local-budget fair-share lane.
    /// The QW1 ledger (call-time gate) does not gate cloud providers either,
    /// so scheduling must not pretend they share the local budget.
    Cloud,
}

impl TaskProviderClass {
    /// Provider key used to key fair-share lanes; cloud tasks share none.
    pub fn lane_key(&self) -> Option<&str> {
        match self {
            TaskProviderClass::Local(key) => Some(key.as_str()),
            TaskProviderClass::Cloud => None,
        }
    }
}

/// Lane key for local tasks whose provider name is not distinguished by the
/// classifier (single-provider deployments, static fallback). Using a stable
/// shared key keeps the degenerate single-lane behavior identical to the
/// pre-hardening single lane.
pub const LOCAL_LANE_DEFAULT_KEY: &str = "local";

/// Classifies the EFFECTIVE extract provider for a task.
#[async_trait]
pub trait TaskProviderClassifier: Send + Sync {
    async fn classify(&self, task: &Task) -> TaskProviderClass;
}

/// Shared classifier handle (port injection).
pub type SharedTaskProviderClassifier = std::sync::Arc<dyn TaskProviderClassifier>;

/// Static classification from the server's extraction provider.
///
/// Default when no workspace-aware resolver is wired (tests, embedded pools):
/// preserves the pre-existing single-lane behavior — one key, one lane.
#[derive(Debug, Clone)]
pub struct StaticProviderClassifier {
    class: TaskProviderClass,
}

impl StaticProviderClassifier {
    pub fn local(key: impl Into<String>) -> Self {
        Self {
            class: TaskProviderClass::Local(key.into()),
        }
    }

    pub fn cloud() -> Self {
        Self {
            class: TaskProviderClass::Cloud,
        }
    }
}

#[async_trait]
impl TaskProviderClassifier for StaticProviderClassifier {
    async fn classify(&self, _task: &Task) -> TaskProviderClass {
        self.class.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{TaskType, TextInsertData};
    use uuid::Uuid;

    fn make_task() -> Task {
        let workspace_id = Uuid::new_v4();
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

    #[tokio::test]
    async fn static_classifier_returns_fixed_class() {
        let task = make_task();
        let local = StaticProviderClassifier::local("ollama");
        assert_eq!(
            local.classify(&task).await,
            TaskProviderClass::Local("ollama".to_string())
        );
        let cloud = StaticProviderClassifier::cloud();
        assert_eq!(cloud.classify(&task).await, TaskProviderClass::Cloud);
    }

    #[test]
    fn lane_key_only_for_local() {
        assert_eq!(
            TaskProviderClass::Local("ollama".into()).lane_key(),
            Some("ollama")
        );
        assert_eq!(TaskProviderClass::Cloud.lane_key(), None);
    }
}
