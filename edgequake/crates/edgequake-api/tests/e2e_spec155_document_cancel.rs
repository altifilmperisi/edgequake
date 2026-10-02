//! SPEC-155 — `POST /documents/{id}/cancel` works for orphan rows (no `track_id`).
//!
//! Regression: a document stuck on "processing" with no track could only be
//! deleted, because cancel was keyed by `track_id`.

mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use common::{
    create_test_app_with_workers, extract_json, TEST_TENANT_ID, TEST_USER_ID, TEST_WORKSPACE_ID,
};
use edgequake_storage::kv_keys;
use serde_json::{json, Value};
use tower::ServiceExt;

async fn post_cancel_document(app: &axum::Router, doc_id: &str) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/documents/{doc_id}/cancel"))
                .header("X-Tenant-ID", TEST_TENANT_ID)
                .header("X-Workspace-ID", TEST_WORKSPACE_ID)
                .header("X-User-ID", TEST_USER_ID)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn seed_doc(
    workers: &common::WorkerAppGuard,
    doc_id: &str,
    status: &str,
    extra: Value,
) -> String {
    let key = kv_keys::doc_metadata(doc_id);
    let mut meta = json!({
        "id": doc_id,
        "status": status,
        "current_stage": status,
        "stage_progress": 0.01,
        "tenant_id": TEST_TENANT_ID,
        "workspace_id": TEST_WORKSPACE_ID,
    });
    if let (Some(base), Some(add)) = (meta.as_object_mut(), extra.as_object()) {
        base.extend(add.clone());
    }
    edgequake_api::services::upsert_metadata_kv_with_index(workers.kv_storage.as_ref(), &key, meta)
        .await
        .expect("seed metadata");
    key
}

#[tokio::test]
async fn e2e_cancel_document_without_track_id_marks_cancelled() {
    let workers = create_test_app_with_workers().await;
    let key = seed_doc(&workers, "orphan-no-track-doc", "processing", json!({})).await;

    let response = post_cancel_document(workers.app(), "orphan-no-track-doc").await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = extract_json(response).await;
    assert_eq!(body["status"], "cancelled");
    assert_eq!(body["task_cancelled"], false);

    let stored = workers.kv_storage.get_by_id(&key).await.unwrap().unwrap();
    assert_eq!(stored["status"], "cancelled");
    assert_eq!(stored["failure_class"], "cancelled");
}

#[tokio::test]
async fn e2e_cancel_document_is_idempotent() {
    let workers = create_test_app_with_workers().await;
    seed_doc(&workers, "orphan-idem-doc", "preprocessing", json!({})).await;

    for _ in 0..2 {
        let response = post_cancel_document(workers.app(), "orphan-idem-doc").await;
        assert_eq!(response.status(), StatusCode::OK);
    }
}

#[tokio::test]
async fn e2e_cancel_document_rejects_finished_document() {
    let workers = create_test_app_with_workers().await;
    seed_doc(&workers, "finished-doc", "completed", json!({})).await;

    let response = post_cancel_document(workers.app(), "finished-doc").await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn e2e_cancel_document_unknown_is_404() {
    let workers = create_test_app_with_workers().await;
    let response = post_cancel_document(workers.app(), "does-not-exist-doc").await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
