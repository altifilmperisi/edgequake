//! Worker pre-flight: never spend PDF/LLM budget on a tombstoned document.
//!
//! The persist gate (`allocate_ingest_generation`) already refuses to ingest
//! into a tombstoned document — but only *after* conversion, vision and entity
//! extraction have run (≈50 min / $0.26 / 600k tokens in the incident this
//! guard exists for). Checking the same authority at task start moves the
//! failure to t=0 and covers **every** entry path (reprocess, re-upload,
//! retry, recovery) with one gate.

use super::*;

/// Task types that ingest into an existing document and so must pass the gate.
fn is_ingest_task(task_type: TaskType) -> bool {
    matches!(
        task_type,
        TaskType::Insert | TaskType::Upload | TaskType::PdfProcessing
    )
}

/// Document an ingest task writes into (`existing_document_id` for PDF
/// conversion, `metadata.document_id` for text insert).
pub(super) fn task_document_id(task: &Task) -> Option<String> {
    task.task_data
        .get("existing_document_id")
        .and_then(|v| v.as_str())
        .or_else(|| {
            task.task_data
                .get("metadata")
                .and_then(|m| m.get("document_id"))
                .and_then(|v| v.as_str())
        })
        .map(str::to_string)
}

impl DocumentTaskProcessor {
    /// `Some(error)` when the task targets a tombstoned document.
    ///
    /// Also heals the stale projection (`delete_failed`) so the row stops
    /// offering Retry/Reprocess. No AppState (unit harness) ⇒ never blocks.
    pub(super) async fn reject_if_document_tombstoned(
        &self,
        task: &Task,
    ) -> Option<edgequake_tasks::TaskError> {
        if !is_ingest_task(task.task_type) {
            return None;
        }
        let state = self.app_state.as_ref()?;
        let document_id = task_document_id(task)?;
        let tenant_ctx = crate::middleware::TenantContext {
            tenant_id: Some(task.tenant_id.to_string()),
            workspace_id: Some(task.workspace_id.to_string()),
            user_id: None,
        };
        if !crate::services::document_tombstone::is_document_tombstoned(
            state,
            &tenant_ctx,
            &document_id,
        )
        .await
        {
            return None;
        }
        warn!(
            task_id = %task.track_id,
            document_id = %document_id,
            "Refusing ingest task: document is tombstoned (delete did not finish)"
        );
        crate::services::document_tombstone::flag_delete_incomplete(state, &document_id).await;
        Some(edgequake_tasks::TaskError::Processing(
            crate::services::document_tombstone::document_deleted_message(&document_id),
        ))
    }

    /// `on_permanent_failure` hook for the tombstone class: keep the row
    /// `delete_failed` instead of overwriting it with a retryable-looking
    /// `failed`. Returns `true` when handled.
    pub(super) async fn handle_deleted_document_failure(
        &self,
        document_id: &str,
        error_msg: &str,
    ) -> bool {
        if edgequake_tasks::classify_ingestion_failure(error_msg)
            != edgequake_tasks::IngestionFailureClass::DocumentDeleted
        {
            return false;
        }
        if let Some(state) = self.app_state.as_ref() {
            crate::services::document_tombstone::flag_delete_incomplete(state, document_id).await;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task_with(data: serde_json::Value, task_type: TaskType) -> Task {
        Task::new(uuid::Uuid::nil(), uuid::Uuid::nil(), task_type, data)
    }

    #[test]
    fn document_id_from_pdf_and_insert_payloads() {
        let pdf = task_with(
            serde_json::json!({"existing_document_id": "d-pdf"}),
            TaskType::PdfProcessing,
        );
        assert_eq!(task_document_id(&pdf).as_deref(), Some("d-pdf"));
        let insert = task_with(
            serde_json::json!({"metadata": {"document_id": "d-ins"}}),
            TaskType::Insert,
        );
        assert_eq!(task_document_id(&insert).as_deref(), Some("d-ins"));
        let none = task_with(serde_json::json!({}), TaskType::Insert);
        assert_eq!(task_document_id(&none), None);
    }

    #[test]
    fn only_ingest_tasks_are_gated() {
        assert!(is_ingest_task(TaskType::Insert));
        assert!(is_ingest_task(TaskType::Upload));
        assert!(is_ingest_task(TaskType::PdfProcessing));
        assert!(!is_ingest_task(TaskType::Deletion));
        assert!(!is_ingest_task(TaskType::BatchDeletion));
        assert!(!is_ingest_task(TaskType::WorkspaceWipe));
    }
}
