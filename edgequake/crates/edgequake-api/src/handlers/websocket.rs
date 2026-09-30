//! WebSocket handlers for real-time progress streaming.
//!
//! SPEC-083 Sprint 1: WsSession identity, workspace filter, track ownership,
//! Deletion* track matching, Lagged client notify.

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, Query, State,
    },
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use edgequake_observability::ErrorEvent;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashSet;
use std::time::Duration;
use tokio::sync::broadcast;
use tracing::{debug, info};

use crate::middleware::{TenantContext, WsSession};
use crate::services::cancel_track_with_doc_and_pdf_chain;
use crate::services::task_scope::get_task_for_context;
use crate::state::AppState;
use std::sync::Arc;

use crate::handlers::websocket_types::{ClientCommand, MAX_WS_SUBSCRIPTIONS};

/// Optional bearer token for WebSocket auth when `EDGEQUAKE_AUTH_ENABLED=true` (SPEC-027 IMP-006).
#[derive(Debug, Default, Deserialize)]
pub struct WsAuthQuery {
    pub token: Option<String>,
}

/// Gate WebSocket upgrade: CORS origin allow-list + JWT/API-key → [`WsSession`].
async fn authorize_ws_upgrade(
    state: &AppState,
    headers: &HeaderMap,
    query: &WsAuthQuery,
) -> Result<WsSession, StatusCode> {
    crate::middleware::ws_validate_origin(state, headers)?;
    // SPEC-154 Wave 5 / EC-154-10: reject query-string credentials.
    if query.token.is_some() {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let header_token = crate::middleware::extract_token_from_headers(headers);
    crate::middleware::ws_validate_token_with_headers(state, header_token.as_deref(), headers)
        .await
        .ok_or(StatusCode::UNAUTHORIZED)
}

/// Echo `edgequake.bearer` when the client offered it (required by RFC 6455 / browsers).
fn ws_with_auth_protocol(ws: WebSocketUpgrade, headers: &HeaderMap) -> WebSocketUpgrade {
    let offered = headers
        .get("sec-websocket-protocol")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|raw| {
            raw.split(',')
                .map(str::trim)
                .any(|p| p == crate::middleware::WS_AUTH_PROTOCOL)
        });
    if offered {
        ws.protocols([crate::middleware::WS_AUTH_PROTOCOL])
    } else {
        ws
    }
}

// Re-export DTOs from websocket_types for backwards compatibility
pub use crate::handlers::websocket_types::{
    ClientCommand as WsClientCommand, ProgressBroadcaster, ProgressEvent,
};

/// Configuration for WebSocket connections.
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);

#[utoipa::path(
    get,
    path = "/ws/pipeline/progress",
    tag = "WebSocket",
    responses(
        (status = 101, description = "WebSocket upgrade successful"),
        (status = 400, description = "WebSocket upgrade failed")
    )
)]
pub async fn ws_pipeline_progress(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(query): Query<WsAuthQuery>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let session = match authorize_ws_upgrade(&state, &headers, &query).await {
        Ok(session) => session,
        Err(status) => return status.into_response(),
    };
    info!("WebSocket connection requested for pipeline progress");
    ws_with_auth_protocol(ws, &headers)
        .on_upgrade(move |socket| handle_pipeline_socket(socket, state, session))
}

/// Handle the WebSocket connection for pipeline progress.
async fn handle_pipeline_socket(socket: WebSocket, state: AppState, session: WsSession) {
    let (mut sender, mut receiver) = socket.split();
    let tenant_ctx = session.to_tenant_context();
    // SPEC-149: multiplexed authorized subscriptions (LAW-149-3/4).
    let mut subscribed: HashSet<String> = HashSet::new();

    info!("WebSocket connection established for pipeline progress");

    let connected_event = ProgressEvent::Connected {
        message: "Connected to pipeline progress stream".to_string(),
    };
    if let Err(e) = send_event(&mut sender, &connected_event, "pipeline_progress").await {
        ws_log_error(
            "send_connected",
            &e.to_string(),
            json!({ "endpoint": "pipeline_progress" }),
        );
        return;
    }

    // Status snapshot is process-global — only send when session is unscoped (auth off).
    if session.workspace_id.is_none() {
        let status = state.tasks.pipeline_state.get_status().await;
        let snapshot_event = ProgressEvent::StatusSnapshot {
            is_busy: status.is_busy,
            job_name: status.job_name.clone(),
            processed_documents: status.processed_documents,
            total_documents: status.total_documents,
            current_batch: status.current_batch,
            total_batches: status.total_batches,
        };
        if let Err(e) = send_event(&mut sender, &snapshot_event, "pipeline_progress").await {
            ws_log_error(
                "send_status_snapshot",
                &e.to_string(),
                json!({ "endpoint": "pipeline_progress" }),
            );
            return;
        }
    }

    let mut progress_rx = state.tasks.progress_broadcaster.subscribe();
    let mut heartbeat_interval = tokio::time::interval(HEARTBEAT_INTERVAL);

    loop {
        tokio::select! {
            Some(msg) = receiver.next() => {
                match msg {
                    Ok(Message::Text(text)) => {
                        debug!("Received text message: {}", text);
                        if text.trim() == "status" {
                            if session.workspace_id.is_none() {
                                let status = state.tasks.pipeline_state.get_status().await;
                                let snapshot = ProgressEvent::StatusSnapshot {
                                    is_busy: status.is_busy,
                                    job_name: status.job_name.clone(),
                                    processed_documents: status.processed_documents,
                                    total_documents: status.total_documents,
                                    current_batch: status.current_batch,
                                    total_batches: status.total_batches,
                                };
                                if let Err(e) = send_event(&mut sender, &snapshot, "pipeline_progress").await {
                                    ws_log_error("send_status_snapshot", &e.to_string(), json!({ "endpoint": "pipeline_progress" }));
                                    break;
                                }
                            }
                        } else if let Ok(cmd) = serde_json::from_str::<ClientCommand>(&text) {
                            match cmd {
                                ClientCommand::Subscribe { track_ids } => {
                                    let (accepted, requested) = apply_subscribe(
                                        &state,
                                        &tenant_ctx,
                                        &track_ids,
                                        &mut subscribed,
                                    )
                                    .await;
                                    let ack = ProgressEvent::SubscribedAck {
                                        accepted,
                                        requested,
                                    };
                                    if let Err(e) =
                                        send_event(&mut sender, &ack, "pipeline_progress").await
                                    {
                                        ws_log_error(
                                            "send_subscribed_ack",
                                            &e.to_string(),
                                            json!({ "endpoint": "pipeline_progress" }),
                                        );
                                        break;
                                    }
                                }
                                ClientCommand::Unsubscribe { track_ids } => {
                                    for id in track_ids {
                                        subscribed.remove(&id);
                                    }
                                }
                                ClientCommand::Cancel { track_id } => {
                                    if let Err(e) = cancel_track_for_session(
                                        &state,
                                        &tenant_ctx,
                                        &track_id,
                                    )
                                    .await
                                    {
                                        tracing::warn!(
                                            track_id = %track_id,
                                            error = %e,
                                            "WebSocket cancel failed"
                                        );
                                    }
                                }
                                ClientCommand::Ping { .. } => {
                                    let heartbeat = ProgressEvent::Heartbeat {
                                        timestamp: chrono::Utc::now().to_rfc3339(),
                                    };
                                    if let Err(e) = send_event(
                                        &mut sender,
                                        &heartbeat,
                                        "pipeline_progress",
                                    )
                                    .await
                                    {
                                        ws_log_error(
                                            "send_ping_heartbeat",
                                            &e.to_string(),
                                            json!({ "endpoint": "pipeline_progress" }),
                                        );
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    Ok(Message::Ping(data)) => {
                        debug!("Received ping, sending pong");
                        if sender.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    Ok(Message::Close(_)) => {
                        info!("WebSocket client disconnected");
                        break;
                    }
                    Err(e) => {
                        ws_log_warn("receive_error", &e.to_string(), json!({ "endpoint": "pipeline_progress" }));
                        break;
                    }
                    _ => {}
                }
            }

            result = progress_rx.recv() => {
                match result {
                    Ok(event) => {
                        if !should_forward_pipeline_event(&event, &session, &subscribed) {
                            continue;
                        }
                        if let Err(e) = send_event(&mut sender, &event, "pipeline_progress").await {
                            ws_log_error("send_progress_event", &e.to_string(), json!({ "endpoint": "pipeline_progress" }));
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        ws_log_warn(
                            "client_lagged",
                            "WebSocket client lagged behind broadcast",
                            json!({ "endpoint": "pipeline_progress", "lagged_events": n }),
                        );
                        // SPEC-083 X-23: notify client (or disconnect).
                        let warn = ProgressEvent::Message {
                            level: "warn".to_string(),
                            message: format!(
                                "Client lagged; {n} events dropped. Reconnect if progress looks incomplete."
                            ),
                            timestamp: chrono::Utc::now().to_rfc3339(),
                        };
                        if send_event(&mut sender, &warn, "pipeline_progress").await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        ws_log_warn(
                            "broadcast_closed",
                            "Progress broadcast channel closed",
                            json!({ "endpoint": "pipeline_progress" }),
                        );
                        break;
                    }
                }
            }

            _ = heartbeat_interval.tick() => {
                let heartbeat = ProgressEvent::Heartbeat {
                    timestamp: chrono::Utc::now().to_rfc3339(),
                };
                if let Err(e) = send_event(&mut sender, &heartbeat, "pipeline_progress").await {
                    ws_log_error("send_heartbeat", &e.to_string(), json!({ "endpoint": "pipeline_progress" }));
                    break;
                }
            }
        }
    }

    info!("WebSocket connection closed for pipeline progress");
}

fn ws_log_error(action: &str, message: &str, details: serde_json::Value) {
    ErrorEvent::log_domain_error("websocket", action, message, details);
}

fn ws_log_warn(action: &str, message: &str, details: serde_json::Value) {
    ErrorEvent::log_domain_warn("websocket", action, message, details);
}

/// Send a progress event as JSON over the WebSocket.
async fn send_event(
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    event: &ProgressEvent,
    endpoint: &str,
) -> Result<(), axum::Error> {
    let json = serde_json::to_string(event).map_err(|e| {
        ws_log_error(
            "serialize_event",
            &e.to_string(),
            json!({ "endpoint": endpoint }),
        );
        axum::Error::new(e)
    })?;
    sender
        .send(Message::Text(json.into()))
        .await
        .map_err(axum::Error::new)
}

async fn cancel_track_for_session(
    state: &AppState,
    tenant_ctx: &TenantContext,
    track_id: &str,
) -> Result<(), String> {
    // SPEC-083 S-02: ownership check before cancel.
    get_task_for_context(state, track_id, tenant_ctx)
        .await
        .map_err(|e| e.to_string())?;

    let vector = state.storage.vector_registry.default_storage();
    let applied = cancel_track_with_doc_and_pdf_chain(
        &state.tasks.storage,
        &state.tasks.cancellation_registry,
        Arc::clone(&state.storage.kv_storage),
        &state.storage.graph_storage,
        &vector,
        track_id,
    )
    .await?;

    info!(
        track_id = %track_id,
        was_running = applied.was_running,
        cancelled = applied.cancelled,
        "WebSocket cancel command applied"
    );
    Ok(())
}

/// Validate and retain owned track subscriptions (SPEC-149 LAW-149-4/5).
///
/// Returns `(accepted_ids, requested_count)`. Rejected ids are omitted from
/// the response so foreign tracks cannot be probed.
async fn apply_subscribe(
    state: &AppState,
    tenant_ctx: &TenantContext,
    track_ids: &[String],
    subscribed: &mut HashSet<String>,
) -> (Vec<String>, usize) {
    let requested = track_ids.len();
    let mut accepted = Vec::new();
    for track_id in track_ids {
        let tid = track_id.trim();
        if tid.is_empty() {
            continue;
        }
        if subscribed.contains(tid) {
            accepted.push(tid.to_string());
            continue;
        }
        if subscribed.len() >= MAX_WS_SUBSCRIPTIONS {
            break;
        }
        match get_task_for_context(state, tid, tenant_ctx).await {
            Ok(_) => {
                subscribed.insert(tid.to_string());
                accepted.push(tid.to_string());
            }
            Err(_) => {
                // Opaque reject — do not reveal whether the track exists.
            }
        }
    }
    (accepted, requested)
}

/// Extract the track/task id from a progress event (DRY for filter + per-track).
pub(crate) fn event_track_id(event: &ProgressEvent) -> Option<&str> {
    match event {
        ProgressEvent::PdfPageProgress { task_id, .. }
        | ProgressEvent::ChunkFailure { task_id, .. }
        | ProgressEvent::ChunkProgress { task_id, .. }
        | ProgressEvent::StageTransition { task_id, .. } => Some(task_id.as_str()),
        ProgressEvent::GraphStorageProgress { track_id, .. }
        | ProgressEvent::DeletionStarted { track_id, .. }
        | ProgressEvent::DeletionPhase { track_id, .. }
        | ProgressEvent::DeletionCompleted { track_id, .. }
        | ProgressEvent::DeletionFailed { track_id, .. } => Some(track_id.as_str()),
        ProgressEvent::BulkDeletionStarted {
            wipe_track_id: Some(tid),
            ..
        }
        | ProgressEvent::BulkDeletionItemProgress {
            wipe_track_id: Some(tid),
            ..
        }
        | ProgressEvent::BulkDeletionCompleted {
            wipe_track_id: Some(tid),
            ..
        }
        | ProgressEvent::BulkDeletionFailed {
            wipe_track_id: tid, ..
        } => Some(tid.as_str()),
        _ => None,
    }
}

/// Filter broadcast events for the multiplexed pipeline socket (SPEC-083 + SPEC-149).
fn should_forward_pipeline_event(
    event: &ProgressEvent,
    session: &WsSession,
    subscribed: &HashSet<String>,
) -> bool {
    match event {
        ProgressEvent::Heartbeat { .. }
        | ProgressEvent::Connected { .. }
        | ProgressEvent::CancellationRequested
        | ProgressEvent::Message { .. }
        | ProgressEvent::SubscribedAck { .. } => true,
        ProgressEvent::BulkDeletionStarted { workspace_id, .. }
        | ProgressEvent::BulkDeletionItemProgress { workspace_id, .. }
        | ProgressEvent::BulkDeletionCompleted { workspace_id, .. }
        | ProgressEvent::BulkDeletionFailed { workspace_id, .. } => {
            // Prefer wipe-track subscription when present; else workspace match.
            if let Some(tid) = event_track_id(event) {
                if subscribed.contains(tid) {
                    return true;
                }
            }
            match (session.workspace_id.as_deref(), workspace_id.as_deref()) {
                (Some(session_ws), Some(event_ws)) => session_ws == event_ws,
                (Some(_), None) => false,
                (None, _) => true,
            }
        }
        // Global pipeline events — only unscoped sessions (auth disabled).
        ProgressEvent::JobStarted { .. }
        | ProgressEvent::DocumentProgress { .. }
        | ProgressEvent::DocumentFailed { .. }
        | ProgressEvent::BatchCompleted { .. }
        | ProgressEvent::JobFinished { .. }
        | ProgressEvent::StatusSnapshot { .. } => session.workspace_id.is_none(),
        // Track-scoped: deliver only when subscribed (ownership checked at subscribe).
        ProgressEvent::ChunkFailure { .. }
        | ProgressEvent::ChunkProgress { .. }
        | ProgressEvent::StageTransition { .. }
        | ProgressEvent::PdfPageProgress { .. }
        | ProgressEvent::GraphStorageProgress { .. }
        | ProgressEvent::DeletionStarted { .. }
        | ProgressEvent::DeletionPhase { .. }
        | ProgressEvent::DeletionCompleted { .. }
        | ProgressEvent::DeletionFailed { .. } => event_track_id(event)
            .map(|tid| subscribed.contains(tid))
            .unwrap_or(false),
    }
}

/// Back-compat name used by older unit tests — unsubscribed track events stay hidden.
#[cfg(test)]
fn event_visible_to_session(event: &ProgressEvent, session: &WsSession) -> bool {
    should_forward_pipeline_event(event, session, &HashSet::new())
}

#[utoipa::path(
    get,
    path = "/ws/progress/{track_id}",
    params(
        ("track_id" = String, Path, description = "Upload tracking ID")
    ),
    tag = "WebSocket",
    responses(
        (status = 101, description = "WebSocket upgrade successful"),
        (status = 400, description = "WebSocket upgrade failed"),
        (status = 404, description = "Track not found for this workspace")
    )
)]
pub async fn ws_progress_by_track_id(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Path(track_id): Path<String>,
    Query(query): Query<WsAuthQuery>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let session = match authorize_ws_upgrade(&state, &headers, &query).await {
        Ok(session) => session,
        Err(status) => return status.into_response(),
    };

    // SPEC-083 S-02: foreign track → 404 before upgrade.
    let tenant_ctx = session.to_tenant_context();
    if let Err(err) = get_task_for_context(&state, &track_id, &tenant_ctx).await {
        let status = match err {
            crate::error::ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            crate::error::ApiError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            crate::error::ApiError::Forbidden(_) => StatusCode::FORBIDDEN,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        return status.into_response();
    }

    info!("WebSocket connection requested for track_id={}", track_id);
    ws_with_auth_protocol(ws, &headers)
        .on_upgrade(move |socket| handle_filtered_progress_socket(socket, state, track_id, session))
}

/// Handle the filtered WebSocket connection for PDF progress.
async fn handle_filtered_progress_socket(
    socket: WebSocket,
    state: AppState,
    track_id: String,
    session: WsSession,
) {
    let (mut sender, mut receiver) = socket.split();
    let tenant_ctx = session.to_tenant_context();

    info!("WebSocket connection established for track_id={}", track_id);

    let connected_event = ProgressEvent::Connected {
        message: format!("Connected to progress stream for {}", track_id),
    };
    if let Err(e) = send_event(&mut sender, &connected_event, &track_id).await {
        ws_log_error(
            "send_connected",
            &e.to_string(),
            json!({ "endpoint": "progress_by_track", "track_id": track_id }),
        );
        return;
    }

    if let Some(progress) = state.tasks.pipeline_state.get_pdf_progress(&track_id).await {
        if let Ok(json) = serde_json::to_value(&progress) {
            let snapshot_msg = serde_json::json!({
                "type": "ProgressSnapshot",
                "data": json
            });
            if let Ok(json_str) = serde_json::to_string(&snapshot_msg) {
                if sender.send(Message::Text(json_str.into())).await.is_err() {
                    ws_log_error(
                        "send_progress_snapshot",
                        "Failed to send progress snapshot",
                        json!({ "endpoint": "progress_by_track", "track_id": track_id }),
                    );
                    return;
                }
            }
        }
    }

    let mut progress_rx = state.tasks.progress_broadcaster.subscribe();
    let mut heartbeat_interval = tokio::time::interval(HEARTBEAT_INTERVAL);

    loop {
        tokio::select! {
            Some(msg) = receiver.next() => {
                match msg {
                    Ok(Message::Text(text)) => {
                        debug!("Received text message from track_id={}: {}", track_id, text);
                        if text.trim() == "status" {
                            if let Some(progress) = state.tasks.pipeline_state.get_pdf_progress(&track_id).await {
                                if let Ok(json) = serde_json::to_value(&progress) {
                                    let snapshot_msg = serde_json::json!({
                                        "type": "ProgressSnapshot",
                                        "data": json
                                    });
                                    if let Ok(json_str) = serde_json::to_string(&snapshot_msg) {
                                        if sender.send(Message::Text(json_str.into())).await.is_err() {
                                            break;
                                        }
                                    }
                                }
                            }
                        } else if let Ok(cmd) = serde_json::from_str::<serde_json::Value>(&text) {
                            if cmd.get("type").and_then(|v| v.as_str()) == Some("cancel") {
                                let id = cmd
                                    .get("track_id")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or(track_id.as_str());
                                if let Err(e) =
                                    cancel_track_for_session(&state, &tenant_ctx, id).await
                                {
                                    tracing::warn!(
                                        track_id = %id,
                                        error = %e,
                                        "WebSocket per-track cancel failed"
                                    );
                                }
                            }
                        }
                    }
                    Ok(Message::Ping(data)) => {
                        debug!("Received ping, sending pong");
                        if sender.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    Ok(Message::Close(_)) => {
                        info!("WebSocket client disconnected for track_id={}", track_id);
                        break;
                    }
                    Err(e) => {
                        ws_log_warn(
                            "receive_error",
                            &e.to_string(),
                            json!({ "endpoint": "progress_by_track", "track_id": track_id }),
                        );
                        break;
                    }
                    _ => {}
                }
            }

            result = progress_rx.recv() => {
                match result {
                    Ok(event) => {
                        if matches_track_id(&event, &track_id) {
                            if let Err(e) = send_event(&mut sender, &event, &track_id).await {
                                ws_log_error(
                                    "send_progress_event",
                                    &e.to_string(),
                                    json!({ "endpoint": "progress_by_track", "track_id": track_id }),
                                );
                                break;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        ws_log_warn(
                            "client_lagged",
                            "WebSocket client lagged behind broadcast",
                            json!({
                                "endpoint": "progress_by_track",
                                "track_id": track_id,
                                "lagged_events": n,
                            }),
                        );
                        let warn = ProgressEvent::Message {
                            level: "warn".to_string(),
                            message: format!(
                                "Client lagged; {n} events dropped. Reconnect if progress looks incomplete."
                            ),
                            timestamp: chrono::Utc::now().to_rfc3339(),
                        };
                        if send_event(&mut sender, &warn, &track_id).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        ws_log_warn(
                            "broadcast_closed",
                            "Progress broadcast channel closed",
                            json!({ "endpoint": "progress_by_track", "track_id": track_id }),
                        );
                        break;
                    }
                }
            }

            _ = heartbeat_interval.tick() => {
                let heartbeat = ProgressEvent::Heartbeat {
                    timestamp: chrono::Utc::now().to_rfc3339(),
                };
                if let Err(e) = send_event(&mut sender, &heartbeat, &track_id).await {
                    ws_log_error(
                        "send_heartbeat",
                        &e.to_string(),
                        json!({ "endpoint": "progress_by_track", "track_id": track_id }),
                    );
                    break;
                }
            }
        }
    }

    info!("WebSocket connection closed for track_id={}", track_id);
}

/// Check if a ProgressEvent matches the specified track_id (SPEC-083 C-24).
fn matches_track_id(event: &ProgressEvent, track_id: &str) -> bool {
    event_track_id(event) == Some(track_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handlers::websocket_types::DeletionPhaseKind;

    #[test]
    fn contract_matches_track_id_deletion_variants() {
        let tid = "track-del-1";
        assert!(matches_track_id(
            &ProgressEvent::DeletionStarted {
                document_id: "d1".into(),
                track_id: tid.into(),
            },
            tid
        ));
        assert!(matches_track_id(
            &ProgressEvent::DeletionPhase {
                document_id: "d1".into(),
                track_id: tid.into(),
                phase: DeletionPhaseKind::RemovingKv,
                phase_label: "Removing document records".into(),
                items_processed: 0,
                items_total: 1,
            },
            tid
        ));
        assert!(matches_track_id(
            &ProgressEvent::DeletionCompleted {
                document_id: "d1".into(),
                track_id: tid.into(),
                chunks_deleted: 1,
                entities_removed: 0,
                relationships_removed: 0,
                embeddings_deleted: 0,
                partial_failure: false,
                error: None,
            },
            tid
        ));
        assert!(matches_track_id(
            &ProgressEvent::DeletionFailed {
                document_id: "d1".into(),
                track_id: tid.into(),
                error: "boom".into(),
            },
            tid
        ));
        assert!(!matches_track_id(
            &ProgressEvent::DeletionStarted {
                document_id: "d1".into(),
                track_id: "other".into(),
            },
            tid
        ));
    }

    #[test]
    fn e2e_ws_tenant_a_never_sees_tenant_b() {
        // Matrix Cluster 01: workspace filter hides foreign progress.
        let session = WsSession {
            tenant_id: Some("t1".into()),
            workspace_id: Some("ws-a".into()),
            user_id: Some("u1".into()),
        };
        assert!(event_visible_to_session(
            &ProgressEvent::BulkDeletionStarted {
                total: 1,
                wipe_track_id: None,
                workspace_id: Some("ws-a".into()),
            },
            &session
        ));
        assert!(!event_visible_to_session(
            &ProgressEvent::BulkDeletionStarted {
                total: 1,
                wipe_track_id: None,
                workspace_id: Some("ws-b".into()),
            },
            &session
        ));
        assert!(!event_visible_to_session(
            &ProgressEvent::JobStarted {
                job_name: "j".into(),
                total_documents: 1,
                total_batches: 1,
            },
            &session
        ));
    }

    #[test]
    fn spec149_subscribed_track_forwards_stage_transition() {
        let session = WsSession {
            tenant_id: Some("t1".into()),
            workspace_id: Some("ws-a".into()),
            user_id: Some("u1".into()),
        };
        let mut subscribed = HashSet::new();
        subscribed.insert("track-owned".into());
        let event = ProgressEvent::StageTransition {
            document_id: "d1".into(),
            task_id: "track-owned".into(),
            stage: "extracting".into(),
            stage_message: "Extracting".into(),
            stage_progress: Some(0.5),
        };
        assert!(should_forward_pipeline_event(&event, &session, &subscribed));
        assert!(!should_forward_pipeline_event(
            &event,
            &session,
            &HashSet::new()
        ));
        let foreign = ProgressEvent::StageTransition {
            document_id: "d2".into(),
            task_id: "track-other".into(),
            stage: "extracting".into(),
            stage_message: "Extracting".into(),
            stage_progress: None,
        };
        assert!(!should_forward_pipeline_event(
            &foreign,
            &session,
            &subscribed
        ));
    }

    #[test]
    fn spec149_client_command_subscribe_deserializes() {
        let raw = r#"{"type":"subscribe","track_ids":["a","b"]}"#;
        let cmd: ClientCommand = serde_json::from_str(raw).unwrap();
        match cmd {
            ClientCommand::Subscribe { track_ids } => {
                assert_eq!(track_ids, vec!["a", "b"]);
            }
            _ => panic!("expected Subscribe"),
        }
    }
}
