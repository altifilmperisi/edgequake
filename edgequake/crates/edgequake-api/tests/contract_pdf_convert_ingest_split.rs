//! SPEC-057 P2: Convert / Ingest stage split contracts.
//!
//! Asserts:
//! - Convert completes + markdown stored; Insert enqueued
//! - Insert failure / timeout does not clear markdown or flip PDF Completed
//! - Cancel after convert with Pending Insert cancels the Insert chain

mod common;

use common::{create_test_app_with_workers, TEST_TENANT_ID, TEST_WORKSPACE_ID};
use edgequake_tasks::{Task, TaskType};
use serde_json::json;
use uuid::Uuid;

#[cfg(feature = "postgres")]
#[tokio::test]
async fn convert_barrier_survives_ingest_failure_simulation() {
    use edgequake_storage::{
        CreatePdfRequest, ExtractionMethod, PdfProcessingStatus, UpdatePdfProcessingRequest,
    };
    use edgequake_tasks::{TaskStatus, TextInsertData};

    let workers = create_test_app_with_workers().await;
    let pdf_storage = workers
        .pdf_storage
        .as_ref()
        .expect("pdf storage under postgres feature");

    let tenant_id = Uuid::parse_str(TEST_TENANT_ID).unwrap();
    let workspace_id = Uuid::parse_str(TEST_WORKSPACE_ID).unwrap();
    let pdf_bytes = b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n1 0 obj<<>>endobj\ntrailer<<>>\n%%EOF\n";
    let pdf_id = pdf_storage
        .create_pdf(CreatePdfRequest {
            workspace_id,
            filename: "p2-split.pdf".to_string(),
            content_type: "application/pdf".to_string(),
            file_size_bytes: pdf_bytes.len() as i64,
            sha256_checksum: format!("p2-split-{}", Uuid::new_v4()),
            page_count: Some(1),
            pdf_data: pdf_bytes.to_vec(),
            vision_model: None,
        })
        .await
        .expect("create pdf");

    let markdown = "# Converted markdown barrier\n\nEntity extraction not required here.";
    pdf_storage
        .update_pdf_processing(UpdatePdfProcessingRequest {
            pdf_id,
            processing_status: PdfProcessingStatus::Completed,
            markdown_content: Some(markdown.to_string()),
            extraction_method: Some(ExtractionMethod::EdgeParse),
            extraction_errors: None,
            document_id: None,
            vision_model: None,
        })
        .await
        .expect("store convert barrier");

    // Follow-on Insert (as Convert would enqueue).
    let text_data = TextInsertData {
        text: markdown.to_string(),
        file_source: "p2-split.pdf".to_string(),
        workspace_id: workspace_id.to_string(),
        metadata: Some(json!({
            "document_id": format!("doc-{}", pdf_id),
            "source": "pdf_upload",
            "source_type": "pdf",
            "pdf_id": pdf_id.to_string(),
        })),
        reuse_excluded_pages: None,
    };
    let mut insert = Task::new(
        tenant_id,
        workspace_id,
        TaskType::Insert,
        serde_json::to_value(&text_data).unwrap(),
    );
    insert.metadata = Some(json!({
        "processing_timeout_secs": 120,
        "source": "pdf_convert_follow_on",
        "pdf_id": pdf_id.to_string(),
    }));
    let insert_track = insert.track_id.clone();
    workers.task_storage.create_task(&insert).await.unwrap();

    // Simulate ingest permanent failure without touching the PDF convert row.
    let mut failed = workers
        .task_storage
        .get_task(&insert_track)
        .await
        .unwrap()
        .unwrap();
    failed.status = TaskStatus::Failed;
    failed.error_message = Some("Simulated extract timeout".to_string());
    workers.task_storage.update_task(&failed).await.unwrap();

    let pdf = pdf_storage.get_pdf(&pdf_id).await.unwrap().unwrap();
    assert_eq!(
        pdf.processing_status,
        PdfProcessingStatus::Completed,
        "ingest failure must not flip PDF Completed"
    );
    assert_eq!(
        pdf.markdown_content.as_deref(),
        Some(markdown),
        "convert markdown barrier must survive ingest failure"
    );

    // Single-flight still sees no active Convert/Insert after failure.
    let active = workers
        .task_storage
        .find_active_pdf_processing_task(pdf_id, workspace_id)
        .await
        .unwrap();
    assert!(active.is_none());
}

#[cfg(feature = "postgres")]
#[tokio::test]
async fn cancel_after_convert_cancels_pending_insert() {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use edgequake_storage::{
        CreatePdfRequest, ExtractionMethod, PdfProcessingStatus, UpdatePdfProcessingRequest,
    };
    use edgequake_tasks::{TaskStatus, TextInsertData};
    use tower::ServiceExt;

    let workers = create_test_app_with_workers().await;
    let pdf_storage = workers
        .pdf_storage
        .as_ref()
        .expect("pdf storage under postgres feature");

    let tenant_id = Uuid::parse_str(TEST_TENANT_ID).unwrap();
    let workspace_id = Uuid::parse_str(TEST_WORKSPACE_ID).unwrap();
    let pdf_bytes = b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n1 0 obj<<>>endobj\ntrailer<<>>\n%%EOF\n";
    let pdf_id = pdf_storage
        .create_pdf(CreatePdfRequest {
            workspace_id,
            filename: "p2-cancel-ingest.pdf".to_string(),
            content_type: "application/pdf".to_string(),
            file_size_bytes: pdf_bytes.len() as i64,
            sha256_checksum: format!("p2-cancel-{}", Uuid::new_v4()),
            page_count: Some(1),
            pdf_data: pdf_bytes.to_vec(),
            vision_model: None,
        })
        .await
        .unwrap();

    pdf_storage
        .update_pdf_processing(UpdatePdfProcessingRequest {
            pdf_id,
            processing_status: PdfProcessingStatus::Completed,
            markdown_content: Some("# barrier".to_string()),
            extraction_method: Some(ExtractionMethod::EdgeParse),
            extraction_errors: None,
            document_id: None,
            vision_model: None,
        })
        .await
        .unwrap();

    let text_data = TextInsertData {
        text: "# barrier".to_string(),
        file_source: "p2-cancel-ingest.pdf".to_string(),
        workspace_id: workspace_id.to_string(),
        metadata: Some(json!({
            "document_id": format!("doc-{}", pdf_id),
            "pdf_id": pdf_id.to_string(),
            "source_type": "pdf",
        })),
        reuse_excluded_pages: None,
    };
    let insert = Task::new(
        tenant_id,
        workspace_id,
        TaskType::Insert,
        serde_json::to_value(&text_data).unwrap(),
    );
    let insert_track = insert.track_id.clone();
    workers.task_storage.create_task(&insert).await.unwrap();

    let response = workers
        .app()
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/documents/pdf/{pdf_id}/cancel"))
                .header("X-Tenant-ID", TEST_TENANT_ID)
                .header("X-Workspace-ID", TEST_WORKSPACE_ID)
                .header("X-User-ID", common::TEST_USER_ID)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let stored = workers
        .task_storage
        .get_task(&insert_track)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.status, TaskStatus::Cancelled);

    let pdf = pdf_storage.get_pdf(&pdf_id).await.unwrap().unwrap();
    assert_eq!(
        pdf.processing_status,
        PdfProcessingStatus::Completed,
        "cancel after convert must leave PDF Completed (convert survives)"
    );
    assert_eq!(pdf.markdown_content.as_deref(), Some("# barrier"));
}

#[tokio::test]
async fn find_active_matches_insert_metadata_pdf_id() {
    let workers = create_test_app_with_workers().await;
    let tenant_id = Uuid::parse_str(TEST_TENANT_ID).unwrap();
    let workspace_id = Uuid::parse_str(TEST_WORKSPACE_ID).unwrap();
    let pdf_id = Uuid::new_v4();

    let insert = Task::new(
        tenant_id,
        workspace_id,
        TaskType::Insert,
        json!({
            "text": "x",
            "file_source": "a.pdf",
            "workspace_id": workspace_id.to_string(),
            "metadata": { "pdf_id": pdf_id.to_string() },
        }),
    );
    workers.task_storage.create_task(&insert).await.unwrap();

    let found = workers
        .task_storage
        .find_active_pdf_processing_task(pdf_id, workspace_id)
        .await
        .unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().task_type, TaskType::Insert);

    let ingest = workers
        .task_storage
        .find_active_pdf_ingest_task(pdf_id, workspace_id)
        .await
        .unwrap();
    assert!(ingest.is_some());
}
